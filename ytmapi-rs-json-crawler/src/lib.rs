//! Library to crawl Json using the pointer syntax.
use error::ParseTarget;
use serde::de::DeserializeOwned;
use std::{slice::IterMut, str::FromStr, sync::Arc, vec::IntoIter};

pub use error::{Error, Result};

mod error;

#[derive(Clone, PartialEq, Debug)]
pub enum JsonPath {
    Pointer(String),
    IndexNum(usize),
}
#[derive(Clone, Default, PartialEq, Debug)]
struct PathList {
    list: Vec<JsonPath>,
}
#[derive(Clone, PartialEq, Debug)]
pub struct JsonCrawler {
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

/// Iterator extension trait containing special methods for Json Crawler
/// iterators to help with error handling.
pub trait JsonCrawlerIterator: Iterator {
    /// Return the first crawler found at `path`, or error.
    fn find_path(self, path: impl AsRef<str>) -> Result<Self::Item>;
    /// Consume self to return (`source`, `path`).
    fn get_context(self) -> (Arc<String>, String);
    /// Return the last item of the array, or return an error with context.
    fn try_last(self) -> Result<Self::Item>;
}

pub struct JsonCrawlerArrayIterMut<'a> {
    source: Arc<String>,
    array: IterMut<'a, serde_json::Value>,
    path: PathList,
    cur_front: usize,
    cur_back: usize,
}
#[derive(Clone)]
pub struct JsonCrawlerArrayIntoIter {
    source: Arc<String>,
    array: IntoIter<serde_json::Value>,
    path: PathList,
    cur_front: usize,
    cur_back: usize,
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
impl From<&PathList> for String {
    fn from(value: &PathList) -> Self {
        let mut path = String::new();
        for p in &value.list {
            path.push_str(String::from(p).as_str());
        }
        path
    }
}
// TODO: Merge with above (AsRef<&PathList>) or specialize.
impl From<PathList> for String {
    fn from(value: PathList) -> Self {
        let mut path = String::new();
        for p in &value.list {
            path.push_str(String::from(p).as_str());
        }
        path
    }
}

impl<'a> Iterator for JsonCrawlerArrayIterMut<'a> {
    type Item = JsonCrawlerBorrowed<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let crawler = self.array.next()?;
        let out = Some(JsonCrawlerBorrowed {
            // Low cost as this is an Arc
            source: self.source.clone(),
            crawler,
            // Ideally there should be a Borrowed version of this struct - otherwise we need to
            // clone every time here.
            path: self.path.clone().with(JsonPath::IndexNum(self.cur_front)),
        });
        self.cur_front += 1;
        out
    }
    // Required to be exact to implement ExactSizeIterator.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.array.len(), Some(self.array.len()))
    }
}

// Default implementation is correct, due to implementation of size_hint.
impl<'a> ExactSizeIterator for JsonCrawlerArrayIterMut<'a> {}

impl<'a> DoubleEndedIterator for JsonCrawlerArrayIterMut<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let crawler = self.array.next_back()?;
        let out = Some(JsonCrawlerBorrowed {
            // Low cost as this is an Arc
            source: self.source.clone(),
            crawler,
            // Ideally there should be a Borrowed version of this struct - otherwise we need to
            // clone every time here.
            path: self.path.clone().with(JsonPath::IndexNum(self.cur_back)),
        });
        self.cur_back -= 1;
        out
    }
}

impl<'a> JsonCrawlerIterator for JsonCrawlerArrayIterMut<'a> {
    fn find_path(mut self, path: impl AsRef<str>) -> Result<Self::Item> {
        self.find_map(|crawler| crawler.navigate_pointer(path.as_ref()).ok())
            .ok_or_else(|| Error::path_not_found_in_array(self.path, self.source, path.as_ref()))
    }
    fn get_context(self) -> (Arc<String>, String) {
        let Self { source, path, .. } = self;
        (source, path.into())
    }
    fn try_last(self) -> Result<Self::Item> {
        let Self {
            source,
            array,
            mut path,
            ..
        } = self;
        let len = array.len();
        path.push(JsonPath::IndexNum(len));
        let Some(last_item) = array.last() else {
            return Err(Error::array_size(path, source, 0));
        };
        Ok(Self::Item {
            source,
            crawler: last_item,
            path,
        })
    }
}

impl Iterator for JsonCrawlerArrayIntoIter {
    type Item = JsonCrawler;
    fn next(&mut self) -> Option<Self::Item> {
        let crawler = self.array.next()?;
        let out = Some(JsonCrawler {
            // Low cost as this is an Arc
            source: self.source.clone(),
            crawler,
            // Ideally there should be a Borrowed version of this struct - otherwise we need to
            // clone every time here.
            path: self.path.clone().with(JsonPath::IndexNum(self.cur_front)),
        });
        self.cur_front += 1;
        out
    }
    // Required to be exact to implement ExactSizeIterator.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.array.len(), Some(self.array.len()))
    }
}
// Default implementation is correct, due to implementation of size_hint.
impl ExactSizeIterator for JsonCrawlerArrayIntoIter {}

impl DoubleEndedIterator for JsonCrawlerArrayIntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        let crawler = self.array.next_back()?;
        let out = Some(JsonCrawler {
            // Low cost as this is an Arc
            source: self.source.clone(),
            crawler,
            // Ideally there should be a Borrowed version of this struct - otherwise we need to
            // clone every time here.
            path: self.path.clone().with(JsonPath::IndexNum(self.cur_back)),
        });
        self.cur_back = self.cur_back.saturating_sub(1);
        out
    }
}
impl JsonCrawlerIterator for JsonCrawlerArrayIntoIter {
    fn find_path(mut self, path: impl AsRef<str>) -> Result<Self::Item> {
        self.find_map(|crawler| crawler.navigate_pointer(path.as_ref()).ok())
            .ok_or_else(|| Error::path_not_found_in_array(self.path, self.source, path.as_ref()))
    }
    fn get_context(self) -> (Arc<String>, String) {
        let Self { source, path, .. } = self;
        (source, path.into())
    }
    fn try_last(self) -> Result<Self::Item> {
        let Self {
            source,
            array,
            mut path,
            ..
        } = self;
        let len = array.len();
        path.push(JsonPath::IndexNum(len));
        let Some(last_item) = array.last() else {
            return Err(Error::array_size(path, source, 0));
        };
        Ok(Self::Item {
            source,
            crawler: last_item,
            path,
        })
    }
}

impl<'a> JsonCrawlerBorrowed<'a> {
    pub fn get_path(&self) -> String {
        (&self.path).into()
    }
    pub fn into_array_iter_mut(self) -> Result<JsonCrawlerArrayIterMut<'a>> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            Error::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
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
    pub fn as_array_iter_mut(&mut self) -> Result<JsonCrawlerArrayIterMut<'_>> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            Error::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
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
    pub fn borrow_index(&mut self, index: usize) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(index));
        let crawler = self
            .crawler
            .get_mut(index)
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn borrow_pointer<S: AsRef<str>>(&mut self, path: S) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn borrow_mut(&mut self) -> JsonCrawlerBorrowed<'_> {
        JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: self.crawler,
            path: self.path.to_owned(),
        }
    }
    // Seems to be a duplicate of borrow_pointer.
    // Only difference is by ref vs by value.
    pub fn navigate_pointer<S: AsRef<str>>(self, path: S) -> Result<JsonCrawlerBorrowed<'a>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(Self {
            source: self.source,
            crawler,
            path: path_clone,
        })
    }
    // A Json string can't directly be serialized to another type, so we can go via
    // str::parse.
    pub fn take_and_parse_str<F: FromStr>(&mut self) -> Result<F>
    where
        F::Err: std::fmt::Display,
    {
        let as_string = self.take_value::<String>()?;
        str::parse::<F>(as_string.as_str()).map_err(|e| {
            Error::parsing(
                self.get_path(),
                // TODO: Remove allocation.
                Arc::new(self.get_source().to_owned()),
                crate::error::ParseTarget::Other(std::any::type_name::<F>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    pub fn take_value<T: DeserializeOwned>(&mut self) -> Result<T> {
        serde_json::from_value(self.crawler.take()).map_err(|e| {
            Error::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    pub fn take_value_pointer<T: DeserializeOwned>(&mut self, path: impl AsRef<str>) -> Result<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path.as_ref())
                .map(|v| v.take())
                .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?,
        )
        .map_err(|e| {
            Error::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    /// Try to take the first value from a list of pointers.
    // TODO: Reduce allocation, complete error, don't require Vec.
    pub fn take_value_pointers<T: DeserializeOwned>(
        &mut self,
        paths: Vec<&'static str>,
    ) -> Result<T> {
        let mut path_clone = self.path.clone();
        let Some((found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p).map(|v| (v.take(), p)))
        else {
            return Err(Error::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.to_string()).collect(),
            ));
        };
        path_clone.push(JsonPath::Pointer(path.to_string()));
        serde_json::from_value(found).map_err(|e| {
            Error::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    pub fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    pub fn get_source(&self) -> &str {
        &self.source
    }
}

impl JsonCrawler {
    pub fn into_array_into_iter(self) -> Result<JsonCrawlerArrayIntoIter> {
        if let JsonCrawler {
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
        Err(Error::parsing(
            &self.path,
            self.source.clone(),
            ParseTarget::Array,
            None,
        ))
    }
    pub fn as_array_iter_mut(&mut self) -> Result<JsonCrawlerArrayIterMut<'_>> {
        let json_array = self.crawler.as_array_mut().ok_or_else(|| {
            Error::parsing(&self.path, self.source.clone(), ParseTarget::Array, None)
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
    // Allow dead code - library type code that may be used in future.
    #[allow(dead_code)]
    pub fn borrow_index(&mut self, index: usize) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(index));
        let crawler = self
            .crawler
            .get_mut(index)
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn borrow_pointer(&mut self, path: &str) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::Pointer(path.to_owned()));
        let crawler = self
            .crawler
            .pointer_mut(path)
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn borrow_mut(&mut self) -> JsonCrawlerBorrowed<'_> {
        JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: &mut self.crawler,
            path: self.path.to_owned(),
        }
    }
    pub fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    // Allow dead code - library type code that may be used in future.
    #[allow(dead_code)]
    pub fn navigate_index(self, index: usize) -> Result<Self> {
        let Self {
            source,
            crawler: mut old_crawler,
            mut path,
        } = self;
        path.push(JsonPath::IndexNum(index));
        let crawler = old_crawler
            .get_mut(index)
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(&path, source.clone()))?;
        Ok(Self {
            source,
            crawler,
            path,
        })
    }
    pub fn navigate_pointer(self, new_path: impl AsRef<str>) -> Result<Self> {
        let Self {
            source,
            crawler: mut old_crawler,
            mut path,
        } = self;
        path.push(JsonPath::pointer(new_path.as_ref()));
        let crawler = old_crawler
            .pointer_mut(new_path.as_ref())
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(&path, source.clone()))?;
        Ok(Self {
            source,
            crawler,
            path,
        })
    }
    // // Allow dead code - library type code that may be used in future.
    // #[allow(dead_code)]
    // pub fn from_string(string: String) -> Result<Self> {
    //     Ok(Self {
    //         crawler: serde_json::from_str(string.as_ref())
    //             .map_err(|_| error::Error::response("Error serializing"))?,
    //         source: Arc::new(string),
    //         path: PathList::default(),
    //     })
    // }
    pub fn take_value<T: DeserializeOwned>(&mut self) -> Result<T> {
        serde_json::from_value(self.crawler.take()).map_err(|e| {
            Error::parsing(
                &self.path,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    pub fn take_value_pointer<T: DeserializeOwned>(&mut self, path: impl AsRef<str>) -> Result<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path.as_ref())
                .map(|v| v.take())
                .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?,
        )
        // XXX: ParseTarget String is incorrect
        .map_err(|e| {
            Error::parsing(
                &path_clone,
                self.source.clone(),
                ParseTarget::Other(std::any::type_name::<T>().to_string()),
                Some(format!("{e}")),
            )
        })
    }
    /// Get the first valid result from a list of closure / path pairs.
    // TODO: Reduce allocation, complete error, don't require Vec.
    pub fn map_paths<T, F>(&mut self, f: F, paths: Vec<&'static str>) -> Result<T>
    where
        F: FnOnce(JsonCrawlerBorrowed) -> T,
    {
        let mut path_clone = self.path.clone();
        let Some((mut found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p).map(|v| (v.take(), p)))
        else {
            return Err(Error::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.to_string()).collect(),
            ));
        };
        todo
        path_clone.push(JsonPath::Pointer(path.to_string()));
        let crawler = JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: &mut found,
            path: path_clone,
        };
        Ok(f(crawler))
    }
    // Allow dead code - library type code that may be used in future.
    #[allow(dead_code)]
    pub fn get_source(&self) -> &str {
        &self.source
    }
}
