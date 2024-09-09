//! # ytmapi_rs
//! Library into YouTube Music's internal API.
//! ## Examples
//! For additional examples using builder, see [`builder`] module.
//! ### Basic usage with a pre-created cookie file.
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
//! ### OAuth usage, using the workflow, and builder method to re-use the `Client`.
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let client = ytmapi_rs::Client::new().unwrap();
//!     let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client).await?;
//!     println!("Go to {url}, finish the login flow, and press enter when done");
//!     let mut _buf = String::new();
//!     let _ = std::io::stdin().read_line(&mut _buf);
//!     let token = ytmapi_rs::generate_oauth_token(&client, code).await?;
//!     // NOTE: The token can be re-used until it expires, and refreshed once it has,
//!     // so it's recommended to save it to a file here.
//!     let yt = ytmapi_rs::YtMusicBuilder::new_with_client(client)
//!         .with_oauth_token(token)
//!         .build()
//!         .unwrap();
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
//! ## Optional Features
//! ### TLS
//! NOTE: reqwest will prefer to utilise default-tls if multiple features are
//! built when using the standard constructors. Use `YtMusicBuilder` to ensure
//! the preferred choice of TLS is used. See reqwest docs for more information <https://docs.rs/reqwest/latest/reqwest/tls/index.html>.
//! - **default-tls** *(enabled by default)*: Utilises the default TLS from
//!   reqwest - at the time of writing is native-tls.
//! - **native-tls**: This feature allows use of the the native-tls crate,
//!   reliant on vendors tls.
//! - **rustls-tls**: This feature allows use of the rustls crate, written in
//!   rust.
//! ### Other
//! - **simplified_queries**: Adds convenience methods to [`YtMusic`].
//! - **serde_json**: Enables some interoperability functions with `serde_json`.
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
use parse::ParseFrom;
use query::{Continuable, GetContinuationsQuery, Query, QueryMethod, StreamingQuery};
use std::{
    borrow::Borrow,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    pin::pin,
};
use tokio_stream::Stream;

#[doc(inline)]
pub use builder::YtMusicBuilder;
#[doc(inline)]
pub use client::Client;
#[doc(inline)]
pub use error::{Error, Result};
#[doc(inline)]
pub use parse::ProcessedResult;
#[doc(inline)]
pub use process::RawResult;

#[macro_use]
mod utils;
mod nav_consts;
mod process;
mod youtube_enums;

pub mod auth;
pub mod builder;
pub mod client;
pub mod common;
pub mod error;
pub mod json;
pub mod parse;
pub mod query;

#[cfg(feature = "simplified-queries")]
#[cfg_attr(docsrs, doc(cfg(feature = "simplified-queries")))]
pub mod simplified_queries;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
// XXX: Note that we would then need to use a RwLock if we wanted to use mutability for
// refresh_token().
/// A handle to the YouTube Music API, wrapping a http client.
/// Generic over AuthToken, as different AuthTokens may allow different queries
/// to be executed.
/// It is recommended to re-use these as they internally contain a connection
/// pool.
/// # Documentation note
/// Examples given for methods on this struct will use fake or mock
/// constructors. When using in a real environment, you will need to construct
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
    pub fn from_browser_token(token: BrowserToken) -> YtMusic<BrowserToken> {
        let client = Client::new().expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Create a new API handle using a real browser authentication cookie saved
    /// to a file on disk.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub async fn from_cookie_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new().expect("Expected Client build to succeed");
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a
    /// String.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub async fn from_cookie<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::new().expect("Expected Client build to succeed");
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
        let client = Client::new().expect("Expected Client build to succeed");
        YtMusic { client, token }
    }
    /// Refresh the internal oauth token, and return a clone of it (for user to
    /// store locally, e.g).
    pub async fn refresh_token(&mut self) -> Result<OAuthToken> {
        let refreshed_token = self.token.refresh(&self.client).await?;
        self.token = refreshed_token.clone();
        Ok(refreshed_token)
    }
    /// Get a hash of the internal oauth token, for use in comparison
    /// operations.
    pub fn get_token_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.token.hash(&mut h);
        h.finish()
    }
}
impl<A: AuthToken> YtMusic<A> {
    /// Return a raw result from YouTube music for query Q that requires further
    /// processing.
    /// # Note
    /// The returned raw result will hold a reference to the query it was called
    /// with. Therefore, passing an owned value is not permitted.
    /// # Usage
    /// ```no_run
    /// use ytmapi_rs::auth::BrowserToken;
    /// use ytmapi_rs::parse::ParseFrom;
    ///
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query =
    ///     ytmapi_rs::query::SearchQuery::new("Beatles").with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let raw_result = yt.raw_query(&query).await?;
    /// let result: Vec<ytmapi_rs::parse::SearchResultArtist> =
    ///     ParseFrom::parse_from(raw_result.process()?)?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn raw_query<'a, Q: Query<A>>(&self, query: &'a Q) -> Result<RawResult<'a, Q, A>> {
        // TODO: Check for a response the reflects an expired Headers token
        Q::Method::call(query, &self.client, &self.token).await
    }
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Note
    /// The returned raw result will hold a reference to the query it was called
    /// with. Therefore, passing an owned value is not permitted.
    /// # Usage
    /// ```no_run
    /// use ytmapi_rs::auth::BrowserToken;
    /// use ytmapi_rs::parse::ParseFrom;
    ///
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query =
    ///     ytmapi_rs::query::SearchQuery::new("Beatles").with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let processed_result = yt.processed_query(&query).await?;
    /// let result: Vec<ytmapi_rs::parse::SearchResultArtist> =
    ///     ParseFrom::parse_from(processed_result)?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn processed_query<'a, Q: Query<A>>(
        &self,
        query: &'a Q,
    ) -> Result<ProcessedResult<'a, Q>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.raw_query(query).await?.process()
    }
    /// Return the raw JSON returned by YouTube music for Query Q.
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Usage
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await?;
    /// let query =
    ///     ytmapi_rs::query::SearchQuery::new("Beatles").with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let json_string = yt.json_query(query).await?;
    /// assert!(serde_json::from_str::<serde_json::Value>(&json_string).is_ok());
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn json_query<Q: Query<A>>(&self, query: impl Borrow<Q>) -> Result<String> {
        // TODO: Remove allocation
        let json = self
            .raw_query(query.borrow())
            .await?
            .process()?
            .clone_json();
        Ok(json)
    }
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    /// # Usage
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("").await?;
    /// let query =
    ///     ytmapi_rs::query::SearchQuery::new("Beatles").with_filter(ytmapi_rs::query::ArtistsFilter);
    /// let result = yt.query(query).await?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn query<Q: Query<A>>(&self, query: impl Borrow<Q>) -> Result<Q::Output> {
        Q::Output::parse_from(self.processed_query(query.borrow()).await?)
    }
    // Stream is tied to the lifetime of self, since it's self's client that will
    // emit the results. It's also tied to the lifetime of query, but ideally it
    // could take either owned or borrowed query.
    pub fn stream<'a, Q: StreamingQuery<'a, A>>(
        &'a self,
        query: &'a Q,
    ) -> impl Stream<Item = Result<Q::Output>> + 'a
    where
        Q::Output: Continuable,
        // May be able to be encoded in StreamingQuery itself
        Q::Output: ParseFrom<GetContinuationsQuery<'a, Q>>,
    {
        query.stream(&self.client, &self.token)
    }
}
/// Generates a tuple containing fresh OAuthDeviceCode and corresponding url for
/// you to authenticate yourself at.
/// This requires a [`Client`] to run.
/// (OAuthDeviceCode, URL)
/// # Usage
/// ```no_run
/// #  async {
/// let client = ytmapi_rs::Client::new().unwrap();
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client).await?;
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_code_and_url(client: &Client) -> Result<(OAuthDeviceCode, String)> {
    let code = OAuthTokenGenerator::new(client).await?;
    let url = format!("{}?user_code={}", code.verification_url, code.user_code);
    Ok((code.device_code, url))
}
/// Generates an OAuth Token when given an OAuthDeviceCode.
/// This requires a [`Client`] to run.
/// # Usage
/// ```no_run
/// #  async {
/// let client = ytmapi_rs::Client::new().unwrap();
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client).await?;
/// println!("Go to {url}, finish the login flow, and press enter when done");
/// let mut buf = String::new();
/// let _ = std::io::stdin().read_line(&mut buf);
/// let token = ytmapi_rs::generate_oauth_token(&client, code).await;
/// assert!(token.is_ok());
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_token(client: &Client, code: OAuthDeviceCode) -> Result<OAuthToken> {
    let token = OAuthToken::from_code(client, code).await?;
    Ok(token)
}
/// Generates a Browser Token when given a browser cookie.
/// This requires a [`Client`] to run.
/// # Usage
/// ```no_run
/// # async {
/// let client = ytmapi_rs::Client::new().unwrap();
/// let cookie = "FAKE COOKIE";
/// let token = ytmapi_rs::generate_browser_token(&client, cookie).await;
/// assert!(matches!(
///     token.unwrap_err().into_kind(),
///     ytmapi_rs::error::ErrorKind::Header
/// ));
/// # };
/// ```
pub async fn generate_browser_token<S: AsRef<str>>(
    client: &Client,
    cookie: S,
) -> Result<BrowserToken> {
    let token = BrowserToken::from_str(cookie.as_ref(), client).await?;
    Ok(token)
}
/// Process a string of JSON as if it had been directly received from the
/// api for a query. Note that this is generic across AuthToken, and you may
/// need to provide the AuthToken type using 'turbofish'.
/// # Usage
/// ```
/// let json = r#"{ "test" : true }"#.to_string();
/// let query = ytmapi_rs::query::SearchQuery::new("Beatles");
/// let result = ytmapi_rs::process_json::<_, ytmapi_rs::auth::BrowserToken>(json, query);
/// assert!(result.is_err());
/// ```
pub fn process_json<Q: Query<A>, A: AuthToken>(
    json: String,
    query: impl Borrow<Q>,
) -> Result<Q::Output> {
    Q::Output::parse_from(RawResult::from_raw(json, query.borrow()).process()?)
}
