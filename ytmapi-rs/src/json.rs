//! This module contains the representation of Json exposed in the default
//! public API in this library.
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Basic representation of any valid Json value, wrapping a
/// `serde_json::Value`. For use if you are implementing [`crate::query::Query`]
/// from scratch. To parse this value, you can utilise the Serialize /
/// Deserialize traits, the [`from_json`] function to convert to a concrete
/// type, or enable the `serde_json` feature to expose the internals via feature
/// gated function `Json::into_inner`.
/// # Note
/// This struct does not implement Deserializer, as implementation is more
/// complex than this thin wrapper.
#[derive(Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json {
    pub(crate) inner: Value,
}

/// Interpret Json as an instance of type T.
/// See [`crate::parse`] for a usage example.
pub fn from_json<T: DeserializeOwned>(json: Json) -> crate::Result<T> {
    serde_json::from_value(json.inner).map_err(|e| crate::Error::from(std::io::Error::from(e)))
}

impl std::fmt::Debug for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Json {
    /// Extract the inner `serde_json::Value`
    #[cfg(feature = "serde_json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "serde_json")))]
    pub fn into_inner(self) -> serde_json::Value {
        self.inner
    }
    pub(crate) fn new(json: serde_json::Value) -> Self {
        Self { inner: json }
    }
}
