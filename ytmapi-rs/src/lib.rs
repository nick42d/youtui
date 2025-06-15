//! # ytmapi_rs
//! Library into YouTube Music's internal API.
//! ## Examples
//! For additional examples using builder, see [`builder`] module.
//! ### Unauthenticated usage - note, not all queries supported.
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let yt = ytmapi_rs::YtMusic::new_unauthenticated().await?;
//!     yt.get_search_suggestions("Beatles").await?;
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
//! ### Basic authenticated usage with a pre-created cookie file.
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
//! ### OAuth authenticated usage, using the workflow, and builder method to re-use the `Client`.
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let client = ytmapi_rs::Client::new().unwrap();
//!     // A Client ID and Client Secret must be provided - see `youtui` README.md.
//!     // In this example, I assume they were put in environment variables beforehand.
//!     let client_id = std::env::var("YOUTUI_OAUTH_CLIENT_ID").unwrap();
//!     let client_secret = std::env::var("YOUTUI_OAUTH_CLIENT_SECRET").unwrap();
//!     let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client, &client_id).await?;
//!     println!("Go to {url}, finish the login flow, and press enter when done");
//!     let mut _buf = String::new();
//!     let _ = std::io::stdin().read_line(&mut _buf);
//!     let token =
//!         ytmapi_rs::generate_oauth_token(&client, code, client_id, client_secret).await?;
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
//! - **reqwest**: Enables some interoperability functions with `reqwest`.
// For feature specific documentation.
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(not(any(
    feature = "rustls-tls",
    feature = "native-tls",
    feature = "default-tls"
)))]
compile_error!("One of the TLS features must be enabled for this crate");
use auth::browser::BrowserToken;
use auth::noauth::NoAuthToken;
use auth::oauth::OAuthDeviceCode;
use auth::{AuthToken, LoggedIn, OAuthToken, OAuthTokenGenerator};
#[doc(inline)]
pub use builder::YtMusicBuilder;
use client::Body;
#[doc(inline)]
pub use client::Client;
use continuations::Continuable;
#[doc(inline)]
pub use error::{Error, Result};
use futures::Stream;
use parse::ParseFrom;
#[doc(inline)]
pub use parse::ProcessedResult;
#[doc(inline)]
pub use process::RawResult;
use query::{PostQuery, Query, QueryMethod};
use serde_json::Value;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use utils::constants::DEFAULT_X_GOOG_AUTHUSER;

#[macro_use]
mod utils;
mod nav_consts;
mod process;
mod youtube_enums;

pub mod auth;
pub mod builder;
pub mod client;
pub mod common;
pub mod continuations;
pub mod error;
pub mod json;
pub mod parse;
pub mod query;

#[cfg(feature = "simplified-queries")]
#[cfg_attr(docsrs, doc(cfg(feature = "simplified-queries")))]
pub mod simplified_queries;

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
impl YtMusic<NoAuthToken> {
    /// Create a new unauthenticated API handle.
    /// In unauthenticated mode, less queries are supported.
    /// Utilises the default TLS option for the enabled features.
    /// # Panics
    /// This will panic in some situations - see <https://docs.rs/reqwest/latest/reqwest/struct.Client.html#panics>
    pub async fn new_unauthenticated() -> Result<Self> {
        let client = Client::new().expect("Expected Client build to succeed");
        let token = NoAuthToken::new(&client).await?;
        Ok(YtMusic { client, token })
    }
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
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::search::ArtistsFilter);
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
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::search::ArtistsFilter);
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
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::search::ArtistsFilter);
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
    /// let query = ytmapi_rs::query::SearchQuery::new("Beatles")
    ///     .with_filter(ytmapi_rs::query::search::ArtistsFilter);
    /// let result = yt.query(query).await?;
    /// assert_eq!(result[0].artist, "The Beatles");
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub async fn query<Q: Query<A>>(&self, query: impl Borrow<Q>) -> Result<Q::Output> {
        Q::Output::parse_from(self.processed_query(query.borrow()).await?)
    }
    /// Stream a query that has 'continuations', i.e can continue to stream
    /// results.
    /// # Return type lifetime notes
    /// The returned Impl Stream is tied to the lifetime of self, since it's
    /// self's client that will emit the results. It's also tied to the
    /// lifetime of query, but ideally it could take either owned or
    /// borrowed query.
    /// # Usage
    /// ```no_run
    /// use futures::stream::TryStreamExt;
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("").await?;
    /// let query = ytmapi_rs::query::GetLibrarySongsQuery::default();
    /// let results = yt.stream(&query).try_collect::<Vec<_>>().await?;
    /// # Ok::<(), ytmapi_rs::Error>(())
    /// # };
    /// ```
    pub fn stream<'a, Q>(&'a self, query: &'a Q) -> impl Stream<Item = Result<Q::Output>> + 'a
    where
        Q: Query<A>,
        Q: PostQuery,
        Q::Output: Continuable<Q>,
    {
        continuations::stream(query, &self.client, &self.token)
    }
}
impl<A: LoggedIn> YtMusic<A> {
    pub async fn upload_song(&self, file_path: impl AsRef<Path>) -> Result<()> {
        const ALLOWED_UPLOAD_EXTENSIONS: &[&str] = &["mp3", "m4a", "wma", "flac", "ogg"];
        let file_path = file_path.as_ref();
        let upload_fileext: String = file_path
            .extension()
            .and_then(OsStr::to_str)
            // "Fileext required for GetUploadSongQuery"
            .unwrap()
            .into();
        if !ALLOWED_UPLOAD_EXTENSIONS
            .iter()
            .any(|ext| upload_fileext.as_str() == *ext)
        {
            panic!(
                "Fileext not in allowed list. Allowed values: {:?}",
                ALLOWED_UPLOAD_EXTENSIONS
            );
        }
        let song_file = tokio::fs::File::open(&file_path).await.unwrap();
        let upload_filesize_bytes = song_file.metadata().await.unwrap().len();
        const MAX_UPLOAD_FILESIZE_MB: u64 = 300;
        if upload_filesize_bytes > MAX_UPLOAD_FILESIZE_MB * (1024 * 1024) {
            panic!(
                "Unable to upload song greater than {} MB, size is {} MB",
                MAX_UPLOAD_FILESIZE_MB,
                upload_filesize_bytes / (1024 * 1024)
            );
        }
        let additional_headers: [(&str, Cow<str>); 4] = [
            (
                "Content-Type",
                "application/x-www-form-urlencoded;charset=utf-8".into(),
            ),
            ("X-Goog-Upload-Command", "start".into()),
            (
                "X-Goog-Upload-Header-Content-Length",
                upload_filesize_bytes.to_string().into(),
            ),
            ("X-Goog-Upload-Protocol", "resumable".into()),
        ];
        let combined_headers = self
            .token
            .headers()
            .unwrap()
            .into_iter()
            .chain(additional_headers)
            .collect::<HashMap<_, _>>();
        let upload_url_raw = self
            .client
            .post_query(
                "https://upload.youtube.com/upload/usermusic/http",
                combined_headers,
                Body::FromString(format!(
                    "filename={}",
                    file_path.file_name().unwrap().to_string_lossy()
                )),
                &[("authuser", DEFAULT_X_GOOG_AUTHUSER)],
            )
            .await
            .unwrap()
            .text;
        let upload_url_json = serde_json::from_str::<Value>(&upload_url_raw)
            .unwrap()
            .get_mut("upload_url")
            .unwrap()
            .take();
        let upload_url: String = serde_json::from_value(upload_url_json).unwrap();
        let additional_headers: [(&str, Cow<str>); 6] = [
            (
                "Content-Type",
                "application/x-www-form-urlencoded;charset=utf-8".into(),
            ),
            ("X-Goog-Upload-Command", "start".into()),
            (
                "X-Goog-Upload-Header-Content-Length",
                upload_filesize_bytes.to_string().into(),
            ),
            ("X-Goog-Upload-Protocol", "resumable".into()),
            ("X-Goog-Upload-Command", "upload, finalize".into()),
            ("X-Goog-Upload-Offset", "0".into()),
        ];
        let combined_headers = self
            .token
            .headers()
            .unwrap()
            .into_iter()
            .chain(additional_headers)
            .collect::<HashMap<_, _>>();
        let result = self
            .client
            .post_query(
                upload_url,
                combined_headers,
                Body::FromFileRef(&song_file),
                &(),
            )
            .await
            .unwrap()
            .status_code;
        Ok(())
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
/// // A Client ID must be provided - see `youtui` README.md.
/// // In this example, I assume it was put in an environment variable beforehand.
/// let client_id = std::env::var("YOUTUI_OAUTH_CLIENT_ID").unwrap();
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client, client_id).await?;
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_code_and_url(
    client: &Client,
    client_id: impl Into<String>,
) -> Result<(OAuthDeviceCode, String)> {
    let code = OAuthTokenGenerator::new(client, client_id).await?;
    let url = format!("{}?user_code={}", code.verification_url, code.user_code);
    Ok((code.device_code, url))
}
/// Generates an OAuth Token when given an OAuthDeviceCode.
/// This requires a [`Client`] to run.
/// # Usage
/// ```no_run
/// #  async {
/// let client = ytmapi_rs::Client::new().unwrap();
/// // A Client ID and Client Secret must be provided - see `youtui` README.md.
/// // In this example, I assume they were put in environment variables beforehand.
/// let client_id = std::env::var("YOUTUI_OAUTH_CLIENT_ID").unwrap();
/// let client_secret = std::env::var("YOUTUI_OAUTH_CLIENT_SECRET").unwrap();
/// let (code, url) = ytmapi_rs::generate_oauth_code_and_url(&client, &client_id).await?;
/// println!("Go to {url}, finish the login flow, and press enter when done");
/// let mut buf = String::new();
/// let _ = std::io::stdin().read_line(&mut buf);
/// let token = ytmapi_rs::generate_oauth_token(&client, code, client_id, client_secret).await;
/// assert!(token.is_ok());
/// # Ok::<(), ytmapi_rs::Error>(())
/// # };
/// ```
pub async fn generate_oauth_token(
    client: &Client,
    code: OAuthDeviceCode,
    client_id: impl Into<String>,
    client_secret: impl Into<String>,
) -> Result<OAuthToken> {
    let token = OAuthToken::from_code(client, code, client_id, client_secret).await?;
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
