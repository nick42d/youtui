//! Module to contain code related to errors that could be produced by the API.
//! This module aims to wrap any non-std crates to avoid leaking dependencies.
use core::fmt::{Debug, Display};
use std::{io, sync::Arc};

/// Alias for a Result with the error type ytmapi-rs::Error.
pub type Result<T> = core::result::Result<T, Error>;

/// This type represents all errors this API could produce.
pub struct Error {
    // This is boxed to avoid passing around very large errors - in the case of an Api error we want to provide the source file to the caller.
    inner: Box<Inner>,
}

enum Inner {
    // Wrapper for reqwest::Error currently
    Web(reqwest::Error),
    // Wrapper for std::io::Error currently
    Io(io::Error),
    // Api was not in the expected format for the library (e.g, expected an array).
    // TODO: Add query type to error.
    Parsing {
        // The target path (JSON pointer notation) that we tried to parse.
        key: String,
        // The source json from Innertube that we were trying to parse.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
        target: ParseTarget,
    },
    // Expected key did not occur in the JSON file.
    Navigation {
        // The target path (JSON pointer notation) that we tried to parse.
        key: String,
        // The source json from Innertube.
        // NOTE: API could theoretically produce multiple errors referring to the same source json.
        // Hence reference counted, Arc particularly to ensure Error is thread safe.
        json: Arc<String>,
    },
    // TODO: Add more detail.
    // Guessing this means we got an invalid response from Innertube.
    // Currently looks to be getting returned when we fail to deserialize the JSON initially.
    InvalidResponse {
        response: String,
    },
    // XXX: Seems to get returned when Innertube Browser Authentication Response doesn't contain the required fields.
    Header,
    Other(String), // Generic catchall - TODO: Remove all of these.
    BrowserTokenExpired,
    OAuthTokenExpired,
    // Received an error code in the JSON message from Innertube.
    // This is a u64 not a usize as that is what serde_json will deserialize to.
    // TODO: Could use a library to handle these.
    OtherErrorCodeInResponse(u64),
}
#[derive(Debug, Clone)]
pub enum ParseTarget {
    Array,
    String,
}
impl Error {
    pub fn oauth_token_expired() -> Self {
        Self {
            inner: Box::new(Inner::OAuthTokenExpired),
        }
    }
    pub fn is_oauth_expired(&self) -> bool {
        if let Inner::OAuthTokenExpired = *self.inner {
            true
        } else {
            false
        }
    }
    pub fn browser_token_expired() -> Self {
        Self {
            inner: Box::new(Inner::BrowserTokenExpired),
        }
    }
    pub fn is_browser_expired(&self) -> bool {
        if let Inner::BrowserTokenExpired = *self.inner {
            true
        } else {
            false
        }
    }
    pub fn navigation<S: Into<String>>(key: S, json: Arc<String>) -> Self {
        Self {
            inner: Box::new(Inner::Navigation {
                key: key.into(),
                json,
            }),
        }
    }
    pub fn parsing<S: Into<String>>(key: S, json: Arc<String>, target: ParseTarget) -> Self {
        Self {
            inner: Box::new(Inner::Parsing {
                key: key.into(),
                json,
                target,
            }),
        }
    }
    pub fn header() -> Self {
        Self {
            inner: Box::new(Inner::Header),
        }
    }
    pub fn response<S: Into<String>>(response: S) -> Self {
        let response = response.into();
        Self {
            inner: Box::new(Inner::InvalidResponse { response }),
        }
    }
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self {
            inner: Box::new(Inner::Other(msg.into())),
        }
    }
    pub fn other_code(code: u64) -> Self {
        Self {
            inner: Box::new(Inner::OtherErrorCodeInResponse(code)),
        }
    }
    pub fn get_json_and_key(&self) -> Option<(String, &String)> {
        match self.inner.as_ref() {
            Inner::Navigation { json, key } => Some((json.to_string(), &key)),
            Inner::Parsing { json, key, .. } => Some((json.to_string(), &key)),
            Inner::Web(_)
            | Inner::Io(_)
            | Inner::InvalidResponse { .. }
            | Inner::Header
            | Inner::Other(_)
            | Inner::OtherErrorCodeInResponse(_) => None,
            Inner::OAuthTokenExpired => None,
            Inner::BrowserTokenExpired => None,
        }
    }
}

impl std::error::Error for Error {}
impl Display for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inner::Web(e) => write!(f, "Web error {e} received."),
            Inner::Io(e) => write!(f, "IO error {e} recieved."),
            Inner::Header => write!(f, "Error parsing header."),
            Inner::InvalidResponse { response: _ } => {
                write!(f, "Response is invalid json - unable to deserialize.")
            }
            Inner::Other(msg) => write!(f, "Generic error - {msg} - recieved."),
            Inner::OtherErrorCodeInResponse(code) => {
                write!(f, "Http error code {code} recieved in response.")
            }
            Inner::Navigation { key, json: _ } => {
                write!(f, "Key {key} not found in Api response.")
            }
            Inner::Parsing {
                key,
                json: _,
                target,
            } => write!(f, "Unable to parse into {:?} at {key}", target),
            Inner::OAuthTokenExpired => write!(f, "OAuth token has expired"),
            Inner::BrowserTokenExpired => write!(f, "Browser token has expired"),
        }
    }
}
// As this is displayed when unwrapping, we don't want to end up including the entire format of this struct
// (potentially including entire source json file).
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
            inner: Box::new(Inner::Web(e)),
        }
    }
}
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            inner: Box::new(Inner::Io(err)),
        }
    }
}
