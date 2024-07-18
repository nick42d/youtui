//! # ytmapi_rs
//! Library into YouTube Music's internal API.
//! ## Examples
//! Basic usage with a pre-created cookie file :
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let cookie_path = std::path::Path::new("./cookie.txt");
//!     let yt = ytmapi_rs::YtMusic::from_cookie_file(cookie_path).await?;
//!     yt.get_search_suggestions("Beatles").await?;
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
//! Basic usage - oauth:
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let (code, url) = ytmapi_rs::generate_oauth_code_and_url().await?;
//!     println!("Go to {url}, finish the login flow, and press enter when done");
//!     let mut _buf = String::new();
//!     let _ = std::io::stdin().read_line(&mut _buf);
//!     let token = ytmapi_rs::generate_oauth_token(code).await?;
//!     // NOTE: The token can be re-used until it expires, and refreshed once it has,
//!     // so it's recommended to save it to a file here.
//!     let yt = ytmapi_rs::YtMusic::from_oauth_token(token);
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
//! ## Optional Features
//! ### TLS
//! NOTE: To use an alternative TLS, you will need to specify `default-features
//! = false`. As reqwest preferentially uses default-tls when multiple TLS
//! features are enabled. See reqwest docs for more information.
//! <https://docs.rs/reqwest/latest/reqwest/tls/index.html>
//! - **default-tls** *(enabled by default)*: Utilises the default TLS from
//!   reqwest - at the time of writing is native-tls.
//! - **native-tls**: This feature forces use of the the native-tls crate,
//!   reliant on vendors tls.
//! - **rustls-tls**: This feature forces use of the rustls crate, written in
//!   rust.
// For feature specific documentation.
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(not(any(
    feature = "rustls-tls",
    feature = "native-tls",
    feature = "default-tls"
)))]
compile_error!("One of the TLS features must be enabled for this crate");
use auth::{
    browser::BrowserToken, oauth::OAuthDeviceCode, AuthToken, OAuthToken, OAuthTokenGenerator,
};
pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{ParseFrom, ProcessedResult};
pub use process::RawResult;
use query::Query;
use reqwest::Client;
use std::path::Path;

// TODO: Confirm if auth should be pub
pub mod auth;
#[macro_use]
mod utils;
mod locales {}
mod nav_consts;
// Consider if pub is correct for this
pub mod common;
mod crawler;
pub mod error;
pub mod parse;
mod process;
pub mod query;
#[cfg(feature = "simplified-queries")]
pub mod simplified_queries;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
/// A handle to the YouTube Music API, wrapping a reqwest::Client.
/// Generic over AuthToken, as different AuthTokens may allow different queries
/// to be executed.
/// # Documentation note
/// Examples given for methods on this struct will use fake or mock
/// constructors. When using in a realy environment, you will need to construct
/// using a real token or cookie.
pub struct YtMusic<A: AuthToken> {
    // TODO: add language
    // TODO: add location
    client: Client,
    token: A,
}

impl YtMusic<BrowserToken> {
    /// Create a new API handle using a BrowserToken.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "default-tls")]
    pub fn from_browser_token(token: BrowserToken) -> YtMusic<BrowserToken> {
        let client = Client::builder()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using a BrowserToken. Forces the use of
    /// `rustls-tls`
    /// # Optional
    /// This requires the optional `rustls-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub fn from_browser_token_rustls_tls(token: BrowserToken) -> YtMusic<BrowserToken> {
        let client = Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using a BrowserToken. Forces the use of
    /// `native-tls`
    /// # Optional
    /// This requires the optional `native-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn from_browser_token_native_tls(token: BrowserToken) -> YtMusic<BrowserToken> {
        let client = Client::builder()
            .use_native_tls()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using a real browser authentication cookie saved
    /// to a file on disk.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub async fn from_cookie_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::builder()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie saved
    /// to a file on disk. Forces the use of `rustls-tls`
    /// # Optional
    /// This requires the optional `rustls-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub async fn from_cookie_file_rustls_tls<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie saved
    /// to a file on disk. Forces the use of `native-tls`
    /// Utilises the default TLS option for the enabled features.
    /// # Optional
    /// This requires the optional `native-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub async fn from_cookie_file_native_tls<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::builder()
            .use_native_tls()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a
    /// String.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub async fn from_cookie<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::builder()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a
    /// String. Forces the use of `rustls-tls`
    /// # Optional
    /// This requires the optional `rustls-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub async fn from_cookie_rustls_tls<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a
    /// String. Forces the use of `native-tls`
    /// # Optional
    /// This requires the optional `native-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub async fn from_cookie_native_tls<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::builder()
            .use_native_tls()
            .build()
            .expect("Expected Client build to succeed");
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(Self { client, token })
    }
}
impl YtMusic<OAuthToken> {
    /// Create a new API handle using an OAuthToken.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub fn from_oauth_token(token: OAuthToken) -> YtMusic<OAuthToken> {
        let client = Client::builder()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using an OAuthToken.
    /// Forces the use of `rustls-tls`.
    /// # Optional
    /// This requires the optional `rustls-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub fn from_oauth_token_rustls_tls(token: OAuthToken) -> YtMusic<OAuthToken> {
        let client = Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using an OAuthToken.
    /// Forces the use of `native-tls`.
    /// # Optional
    /// This requires the optional `native-tls` feature.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn from_oauth_token_native_tls(token: OAuthToken) -> YtMusic<OAuthToken> {
        let client = Client::builder()
            .use_native_tls()
            .build()
            .expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Refresh the internal oauth token, and return a clone of it (for user to
    /// store locally, e.g).
    pub async fn refresh_token(&mut self) -> Result<OAuthToken> {
        let refreshed_token = self.token.refresh(&self.client).await?;
        self.token = refreshed_token.clone();
        Ok(refreshed_token)
    }
}
impl<A: AuthToken> YtMusic<A> {
    /// Return a raw result from YouTube music for query Q that requires further
    /// processing.
    /// # Usage
    /// ```no_run
    /// use ytmapi_rs::parse::ParseFrom;
    /// use ytmapi_rs::auth::BrowserToken;
    ///
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let raw_result = yt.raw_query(query).await?;
    /// let result =
    ///     <Vec::<ytmapi_rs::parse::SearchResultArtist> as ParseFrom<_,BrowserToken>>::parse_from(raw_result.process()?)?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn raw_query<Q: Query<A>>(&self, query: Q) -> Result<RawResult<Q, A>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.token.raw_query(&self.client, query).await
    }
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Usage
    /// ```no_run
    /// use ytmapi_rs::parse::ParseFrom;
    /// use ytmapi_rs::auth::BrowserToken;
    ///
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let processed_result = yt.processed_query(query).await?;
    /// let result =
    ///     <Vec::<ytmapi_rs::parse::SearchResultArtist> as ParseFrom<_,BrowserToken>>::parse_from(processed_result)?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn processed_query<Q: Query<A>>(&self, query: Q) -> Result<ProcessedResult<Q>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.token.raw_query(&self.client, query).await?.process()
    }
    /// Return the raw JSON returned by YouTube music for Query Q.
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Usage
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let json_string = yt.json_query(query).await?;
    /// assert!(serde_json::from_str::<serde_json::Value>(&json_string).is_ok());
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn json_query<Q: Query<A>>(&self, query: Q) -> Result<String> {
        // TODO: Remove allocation
        let json = self.raw_query(query).await?.process()?.clone_json();
        Ok(json)
    }
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Usage
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("").await?;
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let result = yt.query(query).await?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn query<Q: Query<A>>(&self, query: Q) -> Result<Q::Output> {
        query.call(self).await
    }
}
// TODO: Keep session alive after calling these methods.
/// Generates a tuple containing fresh OAuthDeviceCode and corresponding url for
/// you to authenticate yourself at. (OAuthDeviceCode, URL)
/// # Usage
/// ```no_run
/// #  async {
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url().await?;
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_code_and_url() -> Result<(OAuthDeviceCode, String)> {
    let client = Client::new();
    let code = OAuthTokenGenerator::new(&client).await?;
    let url = format!("{}?user_code={}", code.verification_url, code.user_code);
    Ok((code.device_code, url))
}
// TODO: Keep session alive after calling these methods.
/// Generates an OAuth Token when given an OAuthDeviceCode.
/// # Usage
/// ```no_run
/// #  async {
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url().await?;
/// println!("Go to {url}, finish the login flow, and press enter when done");
/// let mut buf = String::new();
/// let _ = std::io::stdin().read_line(&mut buf);
/// let token = ytmapi_rs::generate_oauth_token(code).await;
/// assert!(token.is_ok());
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_token(code: OAuthDeviceCode) -> Result<OAuthToken> {
    let client = Client::new();
    OAuthToken::from_code(&client, code).await
}
// TODO: Keep session alive after calling these methods.
/// Generates a Browser Token when given a browser cookie.
/// # Usage
/// ```no_run
/// # async {
/// let cookie = "FAKE COOKIE";
/// let token = ytmapi_rs::generate_browser_token(cookie).await;
/// assert!(matches!(token.unwrap_err().into_kind(),ytmapi_rs::error::ErrorKind::Header));
/// # };
/// ```
pub async fn generate_browser_token<S: AsRef<str>>(cookie: S) -> Result<BrowserToken> {
    let client = Client::new();
    BrowserToken::from_str(cookie.as_ref(), &client).await
}
/// Process a string of JSON as if it had been directly received from the
/// api for a query. Note that this is generic across AuthToken, and you may
/// need to provide the AuthToken type using 'turbofish'.
/// # Usage
/// ```
/// let json = r#"{ "test" : true }"#.to_string();
/// let query = ytmapi_rs::query::SearchQuery::new("Beatles");
/// let result = ytmapi_rs::process_json::<_,ytmapi_rs::auth::BrowserToken>(json, query);
/// assert!(result.is_err());
/// ```
pub fn process_json<Q: Query<A>, A: AuthToken>(json: String, query: Q) -> Result<Q::Output> {
    Q::Output::parse_from(RawResult::from_raw(json, query).process()?)
}
