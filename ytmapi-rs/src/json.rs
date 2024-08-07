//! This module contains the representation of Json exposed in the default
//! public API in this library.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Basic representation of any valid Json value, wrapping a
/// `serde_json::Value`. For use if you are implementing [`crate::query::Query`]
/// from scratch. To parse this value, you can either utilise the Serialize /
/// Deserialize traits, or enable the `serde_json` feature to expose the
/// internals via feature gated function `Json::into_inner`.
/// # Note
/// This struct does not implement Deserializer, as it would require it's own
/// Error type. Recommend using `serde_json::Value` as the Deserializer if
/// needed.
#[derive(Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json {
    pub(crate) inner: Value,
}

impl std::fmt::Debug for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Json {
    #[cfg(feature = "serde_json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "serde_json")))]
    /// Extract the inner `serde_json::Value`
    fn into_inner(self) -> serde_json::Value {
        self.inner
    }
    pub(crate) fn new(json: serde_json::Value) -> Self {
        Self { inner: json }
    }
}
