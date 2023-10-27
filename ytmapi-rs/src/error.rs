use std::{fmt::Display, io, sync::Arc};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    // This is boxed to avoid passing around very large errors - in the case of an Api error we want to provide the source file to the caller.
    inner: Box<Inner>,
}

#[derive(Debug)]
enum Inner {
    Web(reqwest::Error), // Basic from handling
    Io,                  // Currently limited in information.
    // Api was not in the expected format for the library.
    // TODO: Add query type to error.
    Parsing {
        key: String,
        json: Arc<serde_json::Value>, // Ownership shared between error type and api itself.
        target: ParseTarget,
    },
    Navigation {
        key: String,
        json: Arc<serde_json::Value>, // Ownership shared between error type and api itself.
    },
    InvalidResponse {
        response: String,
    },
    Header,        // Currently limited in information.
    Other(String), // Generic catchall - TODO: Remove all of these.
    NotAuthenticated,
}
#[derive(Debug, Clone)]
pub enum ParseTarget {
    Array,
    String,
}
impl Error {
    pub fn not_authenticated() -> Self {
        Self {
            inner: Box::new(Inner::NotAuthenticated),
        }
    }
    pub fn navigation<S: Into<String>>(key: S, json: Arc<serde_json::Value>) -> Self {
        Self {
            inner: Box::new(Inner::Navigation {
                key: key.into(),
                json,
            }),
        }
    }
    pub fn parsing<S: Into<String>>(
        key: S,
        json: Arc<serde_json::Value>,
        target: ParseTarget,
    ) -> Self {
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
    pub fn get_json_and_key(&self) -> Option<(String, &String)> {
        match self.inner.as_ref() {
            Inner::Navigation { json, key } => Some((json.to_string(), &key)),
            Inner::Parsing { json, key, .. } => Some((json.to_string(), &key)),
            Inner::Web(_)
            | Inner::Io
            | Inner::InvalidResponse { .. }
            | Inner::Header
            | Inner::Other(_)
            | Inner::NotAuthenticated => None,
        }
    }
}

impl std::error::Error for Error {}
impl Display for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Web(e) => write!(f, "Web error {e} received."),
            Self::Io => write!(f, "IO error recieved."),
            Self::Header => write!(f, "Error parsing header."),
            Self::InvalidResponse { response: _ } => {
                write!(f, "Response is invalid json - unable to deserialize.")
            }
            Self::Other(msg) => write!(f, "Generic error - {msg} - recieved."),
            Self::Navigation { key, json: _ } => {
                write!(f, "Key {key} not found in Api response.")
            }
            Self::Parsing {
                key,
                json: _,
                target,
            } => write!(f, "Unable to parse into {:?} at {key}", target),
            Self::NotAuthenticated => write!(f, "API not authenticated"),
        }
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
    fn from(_: io::Error) -> Self {
        Self {
            inner: Box::new(Inner::Io),
        }
    }
}
