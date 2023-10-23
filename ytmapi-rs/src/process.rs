use crate::crawler::JsonCrawlerBorrowed;
use crate::query::Query;

use crate::{Error, Result};
pub use album::*;
pub use artist::*;
pub use search::*;
mod album;
mod artist;
mod search;

mod continuations;
pub use continuations::*;
// Could return FixedColumnItem
// consider if should be optional / error also.
pub fn process_fixed_column_item<'a>(
    item: &'a mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<JsonCrawlerBorrowed<'a>> {
    let pointer = format!("/fixedColumns/{col_idx}/musicResponsiveListItemFixedColumnRenderer");
    item.borrow_pointer(pointer)
}

// Consider if this should return a FlexColumnItem
// In the python API this returns None if /text/runs doesn't exist, but we are not doing that here.
// Checking should instead be done by receiver.
pub fn process_flex_column_item<'a>(
    item: &'a mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<JsonCrawlerBorrowed<'a>> {
    let pointer = format!("/flexColumns/{col_idx}/musicResponsiveListItemFlexColumnRenderer");
    item.borrow_pointer(pointer)
}

// Should trait be Result?
#[derive(PartialEq, Debug)]
pub struct RawResult<T>
where
    T: Query,
{
    query: T,
    json: serde_json::Value,
}

impl<T: Query> RawResult<T> {
    pub fn from_raw(json: serde_json::Value, query: T) -> Self {
        Self { query, json }
    }
    pub fn get_query(&self) -> &T {
        &self.query
    }
    pub fn get_json(&self) -> &serde_json::Value {
        &self.json
    }
    pub fn destructure_json(self) -> serde_json::Value {
        self.json
    }
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::query::SearchQuery;

    use super::*;

    #[tokio::test]
    async fn test_all_raw_impl() {
        let query = SearchQuery::new("Beatles");
        let json = json!({"name": "John Doe"});
        let raw = RawResult::from_raw(json.clone(), query.clone());
        assert_eq!(&query, raw.get_query());
        assert_eq!(&json, raw.get_json());
    }
}

pub mod lyrics {

    use crate::crawler::JsonCrawler;
    use crate::parse::ProcessedResult;
    use crate::query::lyrics::GetLyricsQuery;
    use crate::Result;

    use super::RawResult;

    impl<'a> RawResult<GetLyricsQuery<'a>> {
        pub fn process(self) -> Result<ProcessedResult<GetLyricsQuery<'a>>> {
            match self {
                RawResult { query, json } => Ok(ProcessedResult::from_raw(
                    JsonCrawler::from_json(json),
                    query,
                )),
            }
        }
    }
}

pub mod watch {
    use crate::{
        crawler::JsonCrawler,
        parse::ProcessedResult,
        query::{watch::GetWatchPlaylistQuery, Query},
        Result,
    };

    use super::RawResult;
    impl<T> RawResult<GetWatchPlaylistQuery<T>>
    where
        GetWatchPlaylistQuery<T>: Query,
    {
        pub fn process(self) -> Result<ProcessedResult<GetWatchPlaylistQuery<T>>> {
            match self {
                RawResult { query, json } => Ok(ProcessedResult::from_raw(
                    JsonCrawler::from_json(json),
                    query,
                )),
            }
        }
    }
}
