//! This module contains the representation of Json used in this library.
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

/// Basic representation of any valid Json value, wrapping a
/// `serde_json::Value`, with the minimum required features to allow consumers
/// to implement their own parsers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash, Default)]
pub struct Json {
    inner: Value,
}

/// Basic representation of any valid borrowed Json value, wrapping a
/// `serde_json::Value`, with the minimum required features to allow consumers
/// to implement their own parsers.
#[derive(Debug, Clone, Serialize, PartialEq, Hash)]
pub struct JsonBorrowed<'a> {
    inner: &'a Value,
}

/// Basic representation of any valid mutably borrowed Json value, wrapping a
/// `serde_json::Value`, with the minimum required features to allow consumers
/// to implement their own parsers.
#[derive(Debug, Serialize, PartialEq, Hash)]
pub struct JsonBorrowedMut<'a> {
    inner: &'a mut Value,
}

impl Json {
    /// Take the value and deserialize, leaving a null in it's place.
    // TODO: Determine error type.
    pub fn take<T: DeserializeOwned>(&mut self) -> Option<T> {
        serde_json::from_value(self.inner.take()).ok()
    }
    pub fn pointer(&self, pointer: &str) -> Option<JsonBorrowed<'_>> {
        Some(JsonBorrowed {
            inner: self.inner.pointer(pointer)?,
        })
    }
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<JsonBorrowedMut<'_>> {
        Some(JsonBorrowedMut {
            inner: self.inner.pointer_mut(pointer)?,
        })
    }
}

impl<'a> JsonBorrowed<'a> {
    pub fn pointer(&self, pointer: &str) -> Option<JsonBorrowed<'_>> {
        Some(JsonBorrowed {
            inner: self.inner.pointer(pointer)?,
        })
    }
}

impl<'a> JsonBorrowedMut<'a> {
    /// Take the value and deserialize, leaving a null in it's place.
    // TODO: Determine error type.
    pub fn take<T: DeserializeOwned>(&mut self) -> Option<T> {
        serde_json::from_value(self.inner.take()).ok()
    }
    pub fn pointer(&self, pointer: &str) -> Option<JsonBorrowed<'_>> {
        Some(JsonBorrowed {
            inner: self.inner.pointer(pointer)?,
        })
    }
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<JsonBorrowedMut<'_>> {
        Some(JsonBorrowedMut {
            inner: self.inner.pointer_mut(pointer)?,
        })
    }
}
