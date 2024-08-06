//! This module contains the representation of Json exposed in the default
//! public API in this library.
use serde::{de::Visitor, forward_to_deserialize_any, Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Basic representation of any valid Json value, wrapping a
/// `serde_json::Value`. For use if you are implementing [`crate::query::Query`]
/// from scratch. To parse this value, you can either utilise the Serialize /
/// Deserialize traits, or enable the `serde_json` feature to expose the
/// internals via feature gated function `Json::into_inner`.
#[derive(Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json {
    pub(crate) inner: Value,
}
#[derive(Debug)]
pub struct JsonError;
impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl std::error::Error for JsonError {}
impl serde::de::Error for JsonError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

impl std::fmt::Debug for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'de> Deserializer<'de> for Json {
    type Error = JsonError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::deserialize_any(self.inner, visitor).map_err(|_| JsonError)
    }
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
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
