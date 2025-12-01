use std::fmt::{Debug, Display};
use std::sync::Arc;

pub struct CrawlerError {
    inner: Box<ErrorKind>,
}

pub type CrawlerResult<T> = std::result::Result<T, CrawlerError>;

/// The kind of the error.
enum ErrorKind {
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
    /// Tried multiple ways to pass, and each one returned an error.
    MultipleParseError {
        key: String,
        json: Arc<String>,
        messages: Vec<String>,
    },
}

/// The type we were attempting to pass from the Json.
#[derive(Debug, Clone)]
pub(crate) enum ParseTarget {
    Array,
    Other(String),
}

impl std::fmt::Display for ParseTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseTarget::Array => write!(f, "Array"),
            ParseTarget::Other(t) => write!(f, "{t}"),
        }
    }
}

impl std::error::Error for CrawlerError {}

impl CrawlerError {
    /// Return the source Json and key at the location of the error.
    pub fn get_json_and_key(&self) -> (String, &String) {
        match self.inner.as_ref() {
            ErrorKind::Navigation { json, key } => (json.to_string(), key),
            ErrorKind::Parsing { json, key, .. } => (json.to_string(), key),
            ErrorKind::PathNotFoundInArray { key, json, .. } => (json.to_string(), key),
            ErrorKind::PathsNotFound { key, json, .. } => (json.to_string(), key),
            ErrorKind::ArraySize { key, json, .. } => (json.to_string(), key),
            ErrorKind::MultipleParseError { key, json, .. } => (json.to_string(), key),
        }
    }
    pub(crate) fn multiple_parse_error(
        key: impl Into<String>,
        json: Arc<String>,
        errors: Vec<CrawlerError>,
    ) -> Self {
        let messages = errors.into_iter().map(|e| format!("{e}")).collect();
        Self {
            inner: Box::new(ErrorKind::MultipleParseError {
                key: key.into(),
                json,
                messages,
            }),
        }
    }
    pub(crate) fn navigation(key: impl Into<String>, json: Arc<String>) -> Self {
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
}
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::PathsNotFound {
                key, target_paths, ..
            } => write!(
                f,
                "Expected {key} to contain one of the following paths: {target_paths:?}"
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
                "Error {}. Unable to parse into {target} at {key}",
                message.as_deref().unwrap_or_default()
            ),
            ErrorKind::MultipleParseError {
                key,
                json: _,
                messages,
            } => write!(
                f,
                "Expected one of the parsing functions at {key} to succeed, but all failed with the following errors: {messages:?}"
            ),
        }
    }
}

// As this is displayed when unwrapping, we don't want to end up including the
// entire format of this struct (potentially including entire source json file).
impl Debug for CrawlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Consider customising.
        Display::fmt(&*self.inner, f)
    }
}
impl Display for CrawlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.inner, f)
    }
}
