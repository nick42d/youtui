//! Module to contain code related to errors that could be produced by the API.
use core::fmt::{Debug, Display};
pub use json_crawler::CrawlerError as JsonError;
use std::hash::{Hash, Hasher};
use std::io;
use std::time::SystemTimeError;

/// Alias for a Result with the error type ytmapi-rs::Error.
pub type Result<T> = core::result::Result<T, Error>;

/// This type represents all errors this API could produce.
pub struct Error {
    // This is boxed to avoid passing around very large errors - in the case of an Api error we
    // want to provide the source file to the caller.
    inner: Box<ErrorKind>,
}

/// The kind of the error.
/// This list may grow over time, and it's not recommended to exhaustively match
/// on it.
#[non_exhaustive]
pub enum ErrorKind {
    /// Error parsing Json response from InnerTube.
    JsonParsing(JsonError),
    /// Error from HTTP client.
    Web {
        message: String,
    },
    /// General io error.
    // TODO: improve
    Io(io::Error),
    /// Received a response from InnerTube that was not in the expected (JSON)
    /// format.
    InvalidResponse {
        response: String,
    },
    /// InnerTube credential header not in expected format.
    Header,
    UnableToSerializeGoogleOAuthToken {
        response: String,
        err: serde_json::Error,
    },
    /// ytcfg not in expected format.
    UnableToParseYtCfg {
        ytcfg: String,
    },
    /// ytcfg didn't include visitor data.
    NoVisitorData,
    /// InnerTube rejected the User Agent we are using.
    InvalidUserAgent(String),
    /// OAuthToken has expired.
    /// Returns a hash of the expired token generated using the default hasher.
    OAuthTokenExpired {
        token_hash: u64,
    },
    // This is a u64 not a usize as that is what serde_json will deserialize to.
    // TODO: Could use a library to handle these.
    /// Recieved an error code in the Json reply from InnerTube.
    OtherErrorCodeInResponse {
        code: u64,
        message: String,
    },
    /// Innertube returned a STATUS_FAILED for the query.
    ApiStatusFailed,
    /// Unable to obtain system time for the query to Innertube.
    SystemTimeError {
        message: String,
    },
    /// Tried to upload a song with an invalid upload filename.
    InvalidUploadFilename {
        filename: String,
        message: String,
    },
    MissingUploadUrl,
}
/// The type we were attempting to pass from the Json.
#[derive(Debug, Clone)]
pub enum ParseTarget {
    Array,
    Other(String),
}
impl Error {
    /// Extract the inner kind from the error for pattern matching.
    pub fn into_kind(self) -> ErrorKind {
        *self.inner
    }
    pub(crate) fn invalid_user_agent<S: Into<String>>(user_agent: S) -> Self {
        Self {
            inner: Box::new(ErrorKind::InvalidUserAgent(user_agent.into())),
        }
    }
    pub(crate) fn oauth_token_expired(token: &crate::auth::OAuthToken) -> Self {
        let mut h = std::hash::DefaultHasher::new();
        token.hash(&mut h);
        let token_hash = h.finish();
        Self {
            inner: Box::new(ErrorKind::OAuthTokenExpired { token_hash }),
        }
    }
    pub(crate) fn header() -> Self {
        Self {
            inner: Box::new(ErrorKind::Header),
        }
    }
    pub(crate) fn ytcfg(ytcfg: impl Into<String>) -> Self {
        Self {
            inner: Box::new(ErrorKind::UnableToParseYtCfg {
                ytcfg: ytcfg.into(),
            }),
        }
    }
    pub(crate) fn no_visitor_data() -> Self {
        Self {
            inner: Box::new(ErrorKind::NoVisitorData),
        }
    }
    pub(crate) fn response<S: Into<String>>(response: S) -> Self {
        let response = response.into();
        Self {
            inner: Box::new(ErrorKind::InvalidResponse { response }),
        }
    }
    pub(crate) fn unable_to_serialize_oauth<S: Into<String>>(
        response: S,
        err: serde_json::Error,
    ) -> Self {
        let response = response.into();
        Self {
            inner: Box::new(ErrorKind::UnableToSerializeGoogleOAuthToken { response, err }),
        }
    }
    pub(crate) fn other_code(code: u64, message: String) -> Self {
        Self {
            inner: Box::new(ErrorKind::OtherErrorCodeInResponse { code, message }),
        }
    }
    pub(crate) fn status_failed() -> Self {
        Self {
            inner: Box::new(ErrorKind::ApiStatusFailed),
        }
    }
    pub(crate) fn invalid_upload_filename(filename: String, message: String) -> Self {
        Self {
            inner: Box::new(ErrorKind::InvalidUploadFilename { filename, message }),
        }
    }
    pub(crate) fn missing_upload_url() -> Self {
        Self {
            inner: Box::new(ErrorKind::MissingUploadUrl),
        }
    }
    pub(crate) fn web(message: impl Into<String>) -> Self {
        Self {
            inner: Box::new(ErrorKind::Web {
                message: message.into(),
            }),
        }
    }
}

impl std::error::Error for Error {}
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Web { message } => write!(f, "Web error <{message}> received."),
            ErrorKind::Io(e) => write!(f, "IO error {e} recieved."),
            ErrorKind::Header => write!(f, "Error parsing header."),
            ErrorKind::InvalidResponse { response } => {
                write!(
                    f,
                    "Response is invalid json - unable to deserialize. <{response}>"
                )
            }
            ErrorKind::OtherErrorCodeInResponse { code, message } => {
                write!(
                    f,
                    "Http error code {code} recieved in response. Message: <{message}>."
                )
            }
            ErrorKind::ApiStatusFailed => write!(f, "Api returned STATUS_FAILED for the query"),
            ErrorKind::OAuthTokenExpired { token_hash: _ } => write!(f, "OAuth token has expired"),
            ErrorKind::InvalidUserAgent(u) => write!(f, "InnerTube rejected User Agent {u}"),
            ErrorKind::UnableToSerializeGoogleOAuthToken { response, err } => write!(
                f,
                "Unable to serialize Google auth token {response}, received error {err}"
            ),
            ErrorKind::SystemTimeError { message } => write!(
                f,
                "Error obtaining system time to use in API query. <{message}>"
            ),
            ErrorKind::JsonParsing(e) => write!(f, "{e}"),
            ErrorKind::UnableToParseYtCfg { ytcfg } => write!(
                f,
                "Unable to parse ytcfg - expected the function to exist and contain json. Received: {ytcfg}"
            ),
            ErrorKind::NoVisitorData => write!(f, "ytcfg didn't include VISITOR_DATA"),
            ErrorKind::InvalidUploadFilename {
                filename,
                message: msg,
            } => write!(
                f,
                "Invalid upload filename {filename}. Error message: {msg}"
            ),
            ErrorKind::MissingUploadUrl => {
                write!(f, "expected an x-goog-upload-url but didn't get one")
            }
        }
    }
}
// As this is displayed when unwrapping, we don't want to end up including the
// entire format of this struct (potentially including entire source json file).
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Improve implementation
        Display::fmt(&*self.inner, f)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.inner, f)
    }
}
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let message = err.to_string();
        Self {
            inner: Box::new(ErrorKind::Web { message }),
        }
    }
}
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            inner: Box::new(ErrorKind::Io(err)),
        }
    }
}
impl From<SystemTimeError> for Error {
    fn from(err: SystemTimeError) -> Self {
        let message = err.to_string();
        Self {
            inner: Box::new(ErrorKind::SystemTimeError { message }),
        }
    }
}
impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}
impl From<JsonError> for Error {
    fn from(value: JsonError) -> Self {
        let e = ErrorKind::JsonParsing(value);
        Self { inner: Box::new(e) }
    }
}
