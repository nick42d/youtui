//! Module to contain code related to errors that could be produced by the API.
use core::fmt::{Debug, Display};
use std::{
    hash::{Hash, Hasher},
    io,
    sync::Arc,
    time::SystemTimeError,
};

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
    /// Error from HTTP client.
    Web { message: String },
    /// General io error.
    // TODO: improve
    Io(io::Error),
    /// Expected array at `key` to contain a minimum number of elements.
    ArraySize {
        /// The target path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        /// The minimum number of expected elements.
        min_elements: usize,
    },
    /// Expected the array at `key` to contain a `target_path`
    PathNotFoundInArray {
        /// The target path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        /// The path (JSON pointer notation) we tried to find in the elements of
        /// the array.
        target_path: String,
    },
    /// Expected `key` to contain at least one of `target_paths`
    PathsNotFound {
        /// The path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        /// The paths (JSON pointer notation) we tried to find.
        target_paths: Vec<String>,
    },
    // TODO: Consider adding query type to error.
    /// Field of the JSON file was not in the expected format (e.g expected an
    /// array).
    Parsing {
        /// The target path (JSON pointer notation) that we tried to parse.
        key: String,
        /// The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        /// The format we were trying to parse into.
        target: ParseTarget,
        /// The message we received from the parser, if any.
        //TODO: Include in ParseTarget.
        message: Option<String>,
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
    InvalidResponse { response: String },
    /// InnerTube credential header not in expected format.
    Header,
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
    /// Returns a hash of the expired token generated using the default hasher.
    OAuthTokenExpired { token_hash: u64 },
    // This is a u64 not a usize as that is what serde_json will deserialize to.
    // TODO: Could use a library to handle these.
    /// Recieved an error code in the Json reply from InnerTube.
    OtherErrorCodeInResponse { code: u64, message: String },
    /// Innertube returned a STATUS_FAILED for the query.
    ApiStatusFailed,
    /// Unable to obtain system time for the query to Innertube.
    SystemTimeError { message: String },
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
    /// If an error is a Navigation or Parsing error, return the source Json and
    /// key at the location of the error.
    pub fn get_json_and_key(&self) -> Option<(String, &String)> {
        match self.inner.as_ref() {
            ErrorKind::Navigation { json, key } => Some((json.to_string(), key)),
            ErrorKind::Parsing { json, key, .. } => Some((json.to_string(), key)),
            ErrorKind::PathNotFoundInArray { key, json, .. } => Some((json.to_string(), key)),
            ErrorKind::PathsNotFound { key, json, .. } => Some((json.to_string(), key)),
            ErrorKind::ArraySize { key, json, .. } => Some((json.to_string(), key)),
            ErrorKind::Web { .. }
            | ErrorKind::Io(_)
            | ErrorKind::InvalidResponse { .. }
            | ErrorKind::Header
            | ErrorKind::ApiStatusFailed
            | ErrorKind::UnableToSerializeGoogleOAuthToken { .. }
            | ErrorKind::OtherErrorCodeInResponse { .. }
            | ErrorKind::OAuthTokenExpired { .. }
            | ErrorKind::BrowserAuthenticationFailed
            | ErrorKind::SystemTimeError { .. }
            | ErrorKind::InvalidUserAgent(_) => None,
        }
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
    pub(crate) fn array_size(
        key: impl Into<String>,
        json: Arc<String>,
        min_elements: usize,
    ) -> Self {
        let key = key.into();
        Self {
            inner: Box::new(ErrorKind::ArraySize {
                key,
                json,
                min_elements,
            }),
        }
    }
    pub(crate) fn path_not_found_in_array(
        key: impl Into<String>,
        json: Arc<String>,
        target_path: impl Into<String>,
    ) -> Self {
        let key = key.into();
        let target_path = target_path.into();
        Self {
            inner: Box::new(ErrorKind::PathNotFoundInArray {
                key,
                json,
                target_path,
            }),
        }
    }
    pub(crate) fn paths_not_found(
        key: impl Into<String>,
        json: Arc<String>,
        target_paths: Vec<String>,
    ) -> Self {
        let key = key.into();
        Self {
            inner: Box::new(ErrorKind::PathsNotFound {
                key,
                json,
                target_paths,
            }),
        }
    }
    pub(crate) fn parsing<S: Into<String>>(
        key: S,
        json: Arc<String>,
        target: ParseTarget,
        message: Option<String>,
    ) -> Self {
        Self {
            inner: Box::new(ErrorKind::Parsing {
                key: key.into(),
                json,
                target,
                message,
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
            ErrorKind::PathsNotFound {
                key, target_paths, ..
            } => write!(
                f,
                "Expected {key} to contain one of the following paths: {:?}",
                target_paths
            ),
            ErrorKind::PathNotFoundInArray {
                key, target_path, ..
            } => write!(f, "Expected {key} to contain a {target_path}"),
            ErrorKind::Navigation { key, json: _ } => {
                write!(f, "Key {key} not found in Api response.")
            }
            ErrorKind::ArraySize {
                key,
                json: _,
                min_elements,
            } => {
                write!(
                    f,
                    "Expected {key} to contain at least {min_elements} elements."
                )
            }
            ErrorKind::Parsing {
                key,
                json: _,
                target,
                message,
            } => write!(
                f,
                "Error {:?}. Unable to parse into {:?} at {key}",
                message, target
            ),
            ErrorKind::ApiStatusFailed => write!(f, "Api returned STATUS_FAILED for the query"),
            ErrorKind::OAuthTokenExpired { token_hash: _ } => write!(f, "OAuth token has expired"),
            ErrorKind::InvalidUserAgent(u) => write!(f, "InnerTube rejected User Agent {u}"),
            ErrorKind::BrowserAuthenticationFailed => write!(f, "Browser authentication failed"),
            ErrorKind::UnableToSerializeGoogleOAuthToken { response, err } => write!(
                f,
                "Unable to serialize Google auth token {}, received error {}",
                response, err
            ),
            ErrorKind::SystemTimeError { message } => write!(
                f,
                "Error obtaining system time to use in API query. <{message}>"
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
