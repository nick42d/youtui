//! Iterators and extension for working with crawlers that are pointing to
//! arrays.
use crate::{
    CrawlerError, CrawlerResult, JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerGeneral, JsonPath,
    PathList,
};
use std::{slice::IterMut, sync::Arc, vec::IntoIter};

/// Iterator extension trait containing special methods for Json Crawler
/// iterators to help with error handling.
pub trait JsonCrawlerIterator: Iterator
where
    Self::Item: JsonCrawlerGeneral,
{
    /// Return the first crawler found at `path`, or error.
    fn find_path(self, path: impl AsRef<str>) -> CrawlerResult<Self::Item>;
    /// Consume self to return (`source`, `path`).
    fn get_context(self) -> (Arc<String>, String);
    /// Return the last item of the array, or return an error with context.
    fn try_last(self) -> CrawlerResult<Self::Item>;
}

pub struct JsonCrawlerArrayIterMut<'a> {
    pub(crate) source: Arc<String>,
    pub(crate) array: IterMut<'a, serde_json::Value>,
    pub(crate) path: PathList,
    pub(crate) cur_front: usize,
    pub(crate) cur_back: usize,
}

#[derive(Clone)]
pub struct JsonCrawlerArrayIntoIter {
    pub(crate) source: Arc<String>,
    pub(crate) array: IntoIter<serde_json::Value>,
    pub(crate) path: PathList,
    pub(crate) cur_front: usize,
    pub(crate) cur_back: usize,
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
        self.cur_back = self.cur_back.saturating_sub(1);
        out
    }
}

impl<'a> JsonCrawlerIterator for JsonCrawlerArrayIterMut<'a> {
    fn find_path(mut self, path: impl AsRef<str>) -> CrawlerResult<Self::Item> {
        self.find_map(|crawler| crawler.navigate_pointer(path.as_ref()).ok())
            .ok_or_else(|| {
                CrawlerError::path_not_found_in_array(self.path, self.source, path.as_ref())
            })
    }
    fn get_context(self) -> (Arc<String>, String) {
        let Self { source, path, .. } = self;
        (source, path.into())
    }
    fn try_last(self) -> CrawlerResult<Self::Item> {
        let Self {
            source,
            array,
            mut path,
            ..
        } = self;
        let len = array.len();
        path.push(JsonPath::IndexNum(len));
        let Some(last_item) = array.last() else {
            return Err(CrawlerError::array_size(path, source, 0));
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
    fn find_path(mut self, path: impl AsRef<str>) -> CrawlerResult<Self::Item> {
        self.find_map(|crawler| crawler.navigate_pointer(path.as_ref()).ok())
            .ok_or_else(|| {
                CrawlerError::path_not_found_in_array(self.path, self.source, path.as_ref())
            })
    }
    fn get_context(self) -> (Arc<String>, String) {
        let Self { source, path, .. } = self;
        (source, path.into())
    }
    fn try_last(self) -> CrawlerResult<Self::Item> {
        let Self {
            source,
            array,
            mut path,
            ..
        } = self;
        let len = array.len();
        path.push(JsonPath::IndexNum(len));
        let Some(last_item) = array.last() else {
            return Err(CrawlerError::array_size(path, source, 0));
        };
        Ok(Self::Item {
            source,
            crawler: last_item,
            path,
        })
    }
}
