//! Library to crawl Json using the pointer syntax and return useful errors.
//! Documentation is a work in progress.
use error::ParseTarget;
use serde::de::DeserializeOwned;
use std::{fmt::Display, ops::ControlFlow, str::FromStr, sync::Arc};

pub use error::{CrawlerError, CrawlerResult};
pub use iter::*;
// Currently the only way to create a crawler is from a serde_json::Value, so we
// might as well re-export it.
pub use serde_json::Value;

mod error;
mod iter;

pub trait JsonCrawlerGeneral
where
    Self: Sized,
{
    type BorrowTo<'a>: JsonCrawlerGeneral
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
    fn get_path(&self) -> String;
    fn take_value<T: DeserializeOwned>(&mut self) -> CrawlerResult<T>;
    fn take_value_pointer<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<str>,
    ) -> CrawlerResult<T>;
    fn take_value_pointers<T: DeserializeOwned>(
        &mut self,
        paths: Vec<&'static str>,
    ) -> CrawlerResult<T>;
    fn path_exists(&self, path: &str) -> bool;
    fn get_source(&self) -> Arc<String>;
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
    /// # Warning
    /// If one of the functions mutates before failing, the mutation will still
    /// be applied.
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

impl JsonCrawler {
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

impl<'a> JsonCrawlerGeneral for JsonCrawlerBorrowed<'a> {
    type BorrowTo<'b> = JsonCrawlerBorrowed<'b> where Self: 'b ;
    type IterMut<'b> = JsonCrawlerArrayIterMut<'b> where Self: 'b;
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
    // TODO: Reduce allocation, complete error, don't require Vec.
    fn take_value_pointers<T: DeserializeOwned>(
        &mut self,
        paths: Vec<&'static str>,
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        let Some((found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p).map(|v| (v.take(), p)))
        else {
            return Err(CrawlerError::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.to_string()).collect(),
            ));
        };
        path_clone.push(JsonPath::Pointer(path.to_string()));
        serde_json::from_value(found).map_err(|e| {
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

impl JsonCrawlerGeneral for JsonCrawler {
    type BorrowTo<'a> = JsonCrawlerBorrowed<'a> where Self: 'a;
    type IterMut<'a> = JsonCrawlerArrayIterMut<'a> where Self: 'a;
    type IntoIter = JsonCrawlerArrayIntoIter;
    fn try_into_iter(self) -> CrawlerResult<Self::IntoIter> {
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
    fn take_value_pointers<T: DeserializeOwned>(
        &mut self,
        paths: Vec<&'static str>,
    ) -> CrawlerResult<T> {
        let mut path_clone = self.path.clone();
        let Some((found, path)) = paths
            .iter()
            .find_map(|p| self.crawler.pointer_mut(p).map(|v| (v.take(), p)))
        else {
            return Err(CrawlerError::paths_not_found(
                path_clone,
                self.source.clone(),
                paths.iter().map(|s| s.to_string()).collect(),
            ));
        };
        path_clone.push(JsonPath::Pointer(path.to_string()));
        serde_json::from_value(found).map_err(|e| {
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
