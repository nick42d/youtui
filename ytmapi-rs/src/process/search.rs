use crate::{crawler::JsonCrawler, parse::ProcessedResult, query::*, Result};

use super::RawResult;

impl<'a, S: SearchType> RawResult<SearchQuery<'a, S>> {
    pub fn process(self) -> Result<ProcessedResult<SearchQuery<'a, S>>> {
        match self {
            RawResult { json, query } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}
