//! Library to crawl Json using the pointer syntax and return useful errors.
//! Documentation is a work in progress.
use error::ParseTarget;
pub use error::{CrawlerError, CrawlerResult};
pub use iter::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
// Currently the only way to create a crawler is from a serde_json::Value, so we
// might as well re-export it.
// doc(no_inline) means that the re-export will be clear in the docs.
#[doc(no_inline)]
pub use serde_json::Value;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::str::FromStr;
use std::sync::Arc;

mod error;
mod iter;

/// Trait to represent a JsonCrawler that may own or borrow from the original
/// `serde_json::Value`.
pub trait JsonCrawler
where
    Self: Sized,
{
    type BorrowTo<'a>: JsonCrawler
    where
        Self: 'a;
    type IterMut<'a>: Iterator<Item = Self::BorrowTo<'a>>
    where
        Self: 'a;
    type IntoIter: Iterator<Item = Self>;
    fn navigate_pointer(self, new_path: impl AsRef<str>) -> CrawlerResult<Self>;
    fn navigate_index(self, index: usize) -> CrawlerResult<Self>;
    fn borrow_pointer(&mut self, path: impl AsRef<str>) -> CrawlerResult<Self::BorrowTo<'_>>;
    fn borrow_index(&mut self, index: usize) -> CrawlerResult<Self::BorrowTo<'_>>;
    fn borrow_mut(&mut self) -> Self::BorrowTo<'_>;
    fn try_into_iter(self) -> CrawlerResult<Self::IntoIter>;
    fn try_iter_mut(&mut self) -> CrawlerResult<Self::IterMut<'_>>;
    fn path_exists(&self, path: &str) -> bool;
    fn get_path(&self) -> String;
    fn get_source(&self) -> Arc<String>;
    fn take_value<T: DeserializeOwned>(&mut self) -> CrawlerResult<T>;
    fn take_value_pointer<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T>;
    fn borrow_value<T: for<'de> Deserialize<'de>>(&self) -> CrawlerResult<T>;
    fn borrow_value_pointer<T: for<'de> Deserialize<'de>>(
        &self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T>;
    /// For use when you want to try and take value that could be at multiple
    /// valid locations. Returns an error message that notes that all valid
    /// locations were attempted.
    ///
    /// # Usage
    /// ```no_run
    /// # use json_crawler::*;
    /// # let mut crawler = JsonCrawlerOwned::new(String::new(), serde_json::Value::Null);
    /// // Output will be an error that path should contain "header" and "headerName", if crawler contains neither.
    /// let output: CrawlerResult<String> = crawler.take_value_pointers(&["header", "headerName"]);
    /// ```
    fn take_value_pointers<T: DeserializeOwned, S: AsRef<str>>(
        &mut self,
        paths: &[S],
    ) -> CrawlerResult<T>;
    /// For use when you want to apply some operations that return Option, but
    /// still return an error with context if they fail. For convenience,
    /// closure return type is fallible, allowing you to see the cause of the
    /// error at the failure point as well, if you have it.
    ///
    /// # Usage
    /// ```no_run
    /// # use json_crawler::*;
    /// # let mut crawler = JsonCrawlerOwned::new(String::new(), serde_json::Value::Null);
    /// // Returns Ok(42) if crawler parses into 42.
    /// // Returns parsing from string error, plus the message that output should be 42, if output fails to parse from string.
    /// // Returns message that output should be 42, if output parses from string, but is not 42.
    /// let forty_two: CrawlerResult<usize> = crawler.try_expect("Output should be 42", |crawler| {
    ///     let num = crawler.take_and_parse_str::<usize>()?;
    ///     if num == 42 {
    ///         return Ok(Some(num));
    ///     }
    ///     Ok(None)
    /// });
    /// ```
    fn try_expect<F, O>(&mut self, msg: impl ToString, f: F) -> CrawlerResult<O>
    where
        F: FnOnce(&mut Self) -> CrawlerResult<Option<O>>,
    {
        match f(self) {
            Ok(Some(r)) => Ok(r),
            Ok(None) => Err(CrawlerError::parsing(
                self.get_path(),
                self.get_source(),
                crate::error::ParseTarget::Other(std::any::type_name::<O>().to_string()),
                Some(msg.to_string()),
            )),
            // In this case, we've got a nested error, and should display both sets of context.
            Err(e) => {
                let msg = format!("Expected {} but encountered '{e}'", msg.to_string());
                Err(CrawlerError::parsing(
                    self.get_path(),
                    self.get_source(),
                    crate::error::ParseTarget::Other(std::any::type_name::<O>().to_string()),
                    Some(msg),
                ))
            }
        }
    }
    /// Take the value as a String, and apply FromStr to return the desired
    /// type.
    fn take_and_parse_str<F: FromStr>(&mut self) -> CrawlerResult<F>
    where
        F::Err: Display,
    {
        let as_string = self.take_value::<String>()?;
        str::parse::<F>(as_string.as_str()).map_err(|e| {
            CrawlerError::parsing(
                self.get_path(),
                self.get_source(),
                crate::error::ParseTarget::Other(std::any::type_name::<F>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    /// Try to apply each function in a list of functions, returning the first
    /// Ok result, or the last Err result if none returned Ok.
    ///
    /// # Warning
    /// If one of the functions mutates before failing, the mutation will still
    /// be applied. Also, the mutations are applied sequentially - mutation 1
    /// could impact mutation 2 for example.
    fn try_functions<O>(
        &mut self,
        functions: Vec<fn(&mut Self) -> CrawlerResult<O>>,
    ) -> CrawlerResult<O> {
        let original_path = self.get_path();
        let source_ptr = self.get_source();
        let output = functions.into_iter().try_fold(Vec::new(), |mut acc, f| {
            let res = f(self);
            let e = match res {
                Ok(ret) => return ControlFlow::Break(ret),
                Err(e) => e,
            };
            acc.push(e);
            ControlFlow::Continue(acc)
        });
        match output {
            ControlFlow::Continue(c) => Err(CrawlerError::multiple_parse_error(
                original_path,
                source_ptr,
                c,
            )),
            ControlFlow::Break(b) => Ok(b),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct JsonCrawlerOwned {
    // Source is wrapped in an Arc as we are going to pass ownership when returning an error and we
    // want it to be thread safe.
    source: Arc<String>,
    crawler: serde_json::Value,
    path: PathList,
}
pub struct JsonCrawlerBorrowed<'a> {
    // Source is wrapped in an Arc as we are going to pass ownership when returning an error and we
    // want it to be thread safe.
    source: Arc<String>,
    crawler: &'a mut serde_json::Value,
    path: PathList,
}

impl JsonCrawlerOwned {
    /// Create a new JsonCrawler, where 'json' is the `serde_json::Value` that
    /// you wish to crawl and 'source' represents a serialized copy of the same
    /// `serde_json::Value`.
    // TODO: Safer constructor that avoids 'source' being out of sync with 'json'
    pub fn new(source: String, json: serde_json::Value) -> Self {
        Self {
            source: Arc::new(source),
            crawler: json,
            path: Default::default(),
        }
    }
}

impl<'a> JsonCrawler for JsonCrawlerBorrowed<'a> {
    type BorrowTo<'b>
        = JsonCrawlerBorrowed<'b>
    where
        Self: 'b;
    type IterMut<'b>
        = JsonCrawlerArrayIterMut<'b>
    where
        Self: 'b;
    type IntoIter = JsonCrawlerArrayIterMut<'a>;
    fn take_value_pointer<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path.as_ref())
                .map(|v| v.take())
                .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?,
        )
        .map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn borrow_pointer(&mut self, path: impl AsRef<str>) -> CrawlerResult<Self::BorrowTo<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    fn navigate_pointer(self, path: impl AsRef<str>) -> CrawlerResult<Self> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(Self {
            source: self.source,
            crawler,
            path: path_clone,
        })
    }
    fn try_into_iter(self) -> CrawlerResult<Self::IntoIter> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            CrawlerError::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
        })?;
        let path_clone = self.path.clone();
        let cur_back = json_array.len().saturating_sub(1);
        Ok(JsonCrawlerArrayIterMut {
            source: self.source,
            array: json_array.iter_mut(),
            path: path_clone,
            cur_front: 0,
            cur_back,
        })
    }
    fn try_iter_mut(&mut self) -> CrawlerResult<Self::IterMut<'_>> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            CrawlerError::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
        })?;
        let path_clone = self.path.clone();
        let cur_back = json_array.len().saturating_sub(1);
        Ok(JsonCrawlerArrayIterMut {
            source: self.source.clone(),
            array: json_array.iter_mut(),
            path: path_clone,
            cur_front: 0,
            cur_back,
        })
    }
    fn navigate_index(self, index: usize) -> CrawlerResult<Self> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(index));
        let crawler = self
            .crawler
            .get_mut(index)
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(Self {
            source: self.source,
            crawler,
            path: path_clone,
        })
    }
    fn borrow_index(&mut self, index: usize) -> CrawlerResult<Self::BorrowTo<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(index));
        let crawler = self
            .crawler
            .get_mut(index)
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    fn borrow_mut(&mut self) -> Self::BorrowTo<'_> {
        JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: self.crawler,
            path: self.path.to_owned(),
        }
    }
    fn get_path(&self) -> String {
        (&self.path).into()
    }
    fn take_value<T: DeserializeOwned>(&mut self) -> CrawlerResult<T> {
        serde_json::from_value(self.crawler.take()).map_err(|e| {
            CrawlerError::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn take_value_pointers<T: DeserializeOwned, S: AsRef<str>>(
        &mut self,
        paths: &[S],
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        let Some((found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p.as_ref()).map(|v| (v.take(), p)))
        else {
            return Err(CrawlerError::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.as_ref().to_string()).collect(),
            ));
        };
        path_clone.push(JsonPath::Pointer(path.as_ref().to_string()));
        serde_json::from_value(found).map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn borrow_value<T: for<'de> Deserialize<'de>>(&self) -> CrawlerResult<T> {
        T::deserialize(&*self.crawler).map_err(|e| {
            CrawlerError::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn borrow_value_pointer<T: for<'de> Deserialize<'de>>(
        &self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        // Deserialize without taking ownership or cloning.
        T::deserialize(
            self.crawler
                .pointer(path.as_ref())
                .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?,
        )
        .map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    fn get_source(&self) -> Arc<String> {
        self.source.clone()
    }
}

impl JsonCrawler for JsonCrawlerOwned {
    type BorrowTo<'a>
        = JsonCrawlerBorrowed<'a>
    where
        Self: 'a;
    type IterMut<'a>
        = JsonCrawlerArrayIterMut<'a>
    where
        Self: 'a;
    type IntoIter = JsonCrawlerArrayIntoIter;
    fn try_into_iter(self) -> CrawlerResult<Self::IntoIter> {
        if let JsonCrawlerOwned {
            source,
            crawler: serde_json::Value::Array(array),
            path,
        } = self
        {
            let cur_back = array.len().saturating_sub(1);
            return Ok(JsonCrawlerArrayIntoIter {
                source,
                array: array.into_iter(),
                path,
                cur_front: 0,
                cur_back,
            });
        }
        Err(CrawlerError::parsing(
            &self.path,
            self.source.clone(),
            ParseTarget::Array,
            None,
        ))
    }
    fn try_iter_mut(&mut self) -> CrawlerResult<Self::IterMut<'_>> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            CrawlerError::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
        })?;
        let path_clone = self.path.clone();
        let cur_back = json_array.len().saturating_sub(1);
        Ok(JsonCrawlerArrayIterMut {
            source: self.source.clone(),
            array: json_array.iter_mut(),
            path: path_clone,
            cur_front: 0,
            cur_back,
        })
    }
    fn navigate_pointer(self, new_path: impl AsRef<str>) -> CrawlerResult<Self> {
        let Self {
            source,
            crawler: mut old_crawler,
            mut path,
        } = self;
        path.push(JsonPath::pointer(new_path.as_ref()));
        let crawler = old_crawler
            .pointer_mut(new_path.as_ref())
            .map(|v| v.take())
            .ok_or_else(|| CrawlerError::navigation(&path, source.clone()))?;
        Ok(Self {
            source,
            crawler,
            path,
        })
    }
    fn navigate_index(self, index: usize) -> CrawlerResult<Self> {
        let Self {
            source,
            crawler: mut old_crawler,
            mut path,
        } = self;
        path.push(JsonPath::IndexNum(index));
        let crawler = old_crawler
            .get_mut(index)
            .map(|v| v.take())
            .ok_or_else(|| CrawlerError::navigation(&path, source.clone()))?;
        Ok(Self {
            source,
            crawler,
            path,
        })
    }
    fn borrow_pointer(&mut self, path: impl AsRef<str>) -> CrawlerResult<Self::BorrowTo<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::Pointer(path.as_ref().to_owned()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    fn borrow_index(&mut self, index: usize) -> CrawlerResult<Self::BorrowTo<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(index));
        let crawler = self
            .crawler
            .get_mut(index)
            .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    fn borrow_mut(&mut self) -> Self::BorrowTo<'_> {
        JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: &mut self.crawler,
            path: self.path.to_owned(),
        }
    }
    fn take_value<T: DeserializeOwned>(&mut self) -> CrawlerResult<T> {
        serde_json::from_value(self.crawler.take()).map_err(|e| {
            CrawlerError::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn take_value_pointer<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path.as_ref())
                .map(|v| v.take())
                .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?,
        )
        .map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn take_value_pointers<T: DeserializeOwned, S: AsRef<str>>(
        &mut self,
        paths: &[S],
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        let Some((found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p.as_ref()).map(|v| (v.take(), p)))
        else {
            return Err(CrawlerError::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.as_ref().to_string()).collect(),
            ));
        };
        path_clone.push(JsonPath::Pointer(path.as_ref().to_string()));
        serde_json::from_value(found).map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn borrow_value<T: DeserializeOwned>(&self) -> CrawlerResult<T> {
        T::deserialize(&self.crawler).map_err(|e| {
            CrawlerError::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn borrow_value_pointer<T: DeserializeOwned>(&self, path: impl AsRef<str>) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        T::deserialize(
            self.crawler
                .pointer(path.as_ref())
                .ok_or_else(|| CrawlerError::navigation(&path_clone, self.source.clone()))?,
        )
        .map_err(|e| {
            CrawlerError::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    fn get_source(&self) -> Arc<String> {
        self.source.clone()
    }
    fn get_path(&self) -> String {
        (&self.path).into()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum JsonPath {
    Pointer(String),
    IndexNum(usize),
}
#[derive(Clone, Default, PartialEq, Debug)]
struct PathList {
    list: Vec<JsonPath>,
}

impl From<&JsonPath> for String {
    fn from(value: &JsonPath) -> Self {
        match value {
            JsonPath::Pointer(p) => p.to_owned(),
            JsonPath::IndexNum(i) => format! {"/{i}"},
        }
    }
}
impl JsonPath {
    pub fn pointer<S: Into<String>>(path: S) -> Self {
        JsonPath::Pointer(path.into())
    }
}
impl PathList {
    fn with(mut self, path: JsonPath) -> Self {
        self.list.push(path);
        self
    }
    fn push(&mut self, path: JsonPath) {
        self.list.push(path)
    }
}

// I believe both implementations are required, due to orphan rules.
impl From<&PathList> for String {
    fn from(value: &PathList) -> Self {
        let mut path = String::new();
        for p in &value.list {
            path.push_str(String::from(p).as_str());
        }
        path
    }
}
impl From<PathList> for String {
    fn from(value: PathList) -> Self {
        let mut path = String::new();
        for p in &value.list {
            path.push_str(String::from(p).as_str());
        }
        path
    }
}
