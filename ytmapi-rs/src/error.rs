//! Module to contain code related to errors that could be produced by the API.
use core::fmt::{Debug, Display};
use std::{io, sync::Arc};

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
    /// General web error.
    // TODO: improve and avoid leaking reqwest::Error
    Web(reqwest::Error),
    /// General io error.
    // TODO: improve
    Io(io::Error),
    // Api was not in the expected format for the library (e.g, expected an array).
    // TODO: Add query type to error.
    /// Field of the JSON file was not in the expected format.
    Parsing {
        /// The target path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        /// The format we were trying to parse into.
        target: ParseTarget,
    },
    /// Expected key did not occur in the JSON file.
    Navigation {
        /// The target path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
    },
    /// Received a response from InnerTube that was not in the expected (JSON)
    /// format.
    InvalidResponse {
        response: String,
    },
    /// InnerTube credential header not in expected format.
    Header,
    Other(String), // Generic catchall - TODO: Remove all of these.
    UnableToSerializeGoogleOAuthToken {
        response: String,
        err: serde_json::Error,
    },
    /// InnerTube rejected the User Agent we are using.
    InvalidUserAgent(String),
    /// Failed to authenticate using Browse Auth credentials (may have expired,
    /// or been incorrectly provided).
    BrowserAuthenticationFailed,
    /// OAuthToken has expired.
    OAuthTokenExpired,
    // This is a u64 not a usize as that is what serde_json will deserialize to.
    // TODO: Could use a library to handle these.
    /// Recieved an error code in the Json reply from InnerTube.
    OtherErrorCodeInResponse {
        code: u64,
        message: String,
    },
}
/// The type we were attempting to pass from the Json.
#[derive(Debug, Clone)]
pub enum ParseTarget {
    Array,
    String,
    Enum,
}
impl Error {
    /// Extract the inner kind from the error for pattern matching.
    pub fn into_kind(self) -> ErrorKind {
        *self.inner
    }
    // Only used for tests currently.
    pub(crate) fn is_oauth_expired(&self) -> bool {
        if let ErrorKind::OAuthTokenExpired = *self.inner {
            true
        } else {
            false
        }
    }
    // Only used for tests currently.
    pub(crate) fn is_browser_authentication_failed(&self) -> bool {
        if let ErrorKind::BrowserAuthenticationFailed = *self.inner {
            true
        } else {
            false
        }
    }
    /// If an error is a Navigation or Parsing error, return the source Json and
    /// key at the location of the error.
    pub fn get_json_and_key(&self) -> Option<(String, &String)> {
        match self.inner.as_ref() {
            ErrorKind::Navigation { json, key } => Some((json.to_string(), &key)),
            ErrorKind::Parsing { json, key, .. } => Some((json.to_string(), &key)),
            ErrorKind::Web(_)
            | ErrorKind::Io(_)
            | ErrorKind::InvalidResponse { .. }
            | ErrorKind::Header
            | ErrorKind::Other(_)
            | ErrorKind::UnableToSerializeGoogleOAuthToken { .. }
            | ErrorKind::OtherErrorCodeInResponse { .. }
            | ErrorKind::OAuthTokenExpired
            | ErrorKind::BrowserAuthenticationFailed
            | ErrorKind::InvalidUserAgent(_) => None,
        }
    }
    pub(crate) fn invalid_user_agent<S: Into<String>>(user_agent: S) -> Self {
        Self {
            inner: Box::new(ErrorKind::InvalidUserAgent(user_agent.into())),
        }
    }
    pub(crate) fn oauth_token_expired() -> Self {
        Self {
            inner: Box::new(ErrorKind::OAuthTokenExpired),
        }
    }
    pub(crate) fn browser_authentication_failed() -> Self {
        Self {
            inner: Box::new(ErrorKind::BrowserAuthenticationFailed),
        }
    }
    pub(crate) fn navigation<S: Into<String>>(key: S, json: Arc<String>) -> Self {
        Self {
            inner: Box::new(ErrorKind::Navigation {
                key: key.into(),
                json,
            }),
        }
    }
    pub(crate) fn parsing<S: Into<String>>(key: S, json: Arc<String>, target: ParseTarget) -> Self {
        Self {
            inner: Box::new(ErrorKind::Parsing {
                key: key.into(),
                json,
                target,
            }),
        }
    }
    pub(crate) fn header() -> Self {
        Self {
            inner: Box::new(ErrorKind::Header),
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
    pub(crate) fn other<S: Into<String>>(msg: S) -> Self {
        Self {
            inner: Box::new(ErrorKind::Other(msg.into())),
        }
    }
    pub(crate) fn other_code(code: u64, message: String) -> Self {
        Self {
            inner: Box::new(ErrorKind::OtherErrorCodeInResponse { code, message }),
        }
    }
}

impl std::error::Error for Error {}
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Web(e) => write!(f, "Web error {e} received."),
            ErrorKind::Io(e) => write!(f, "IO error {e} recieved."),
            ErrorKind::Header => write!(f, "Error parsing header."),
            ErrorKind::InvalidResponse { response: _ } => {
                write!(f, "Response is invalid json - unable to deserialize.")
            }
            ErrorKind::Other(msg) => write!(f, "Generic error - {msg} - recieved."),
            ErrorKind::OtherErrorCodeInResponse { code, message } => {
                write!(
                    f,
                    "Http error code {code} recieved in response. Message: <{message}>."
                )
            }
            ErrorKind::Navigation { key, json: _ } => {
                write!(f, "Key {key} not found in Api response.")
            }
            ErrorKind::Parsing {
                key,
                json: _,
                target,
            } => write!(f, "Unable to parse into {:?} at {key}", target),
            ErrorKind::OAuthTokenExpired => write!(f, "OAuth token has expired"),
            ErrorKind::InvalidUserAgent(u) => write!(f, "InnerTube rejected User Agent {u}"),
            ErrorKind::BrowserAuthenticationFailed => write!(f, "Browser authentication failed"),
            ErrorKind::UnableToSerializeGoogleOAuthToken { response, err } => write!(
                f,
                "Unable to serialize Google auth token {}, received error {}",
                response, err
            ),
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
    fn from(e: reqwest::Error) -> Self {
        Self {
            inner: Box::new(ErrorKind::Web(e)),
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
