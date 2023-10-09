use crate::crawler::JsonCrawler;
use crate::query::continuations::GetContinuationsQuery;
use crate::Result;
use crate::{parse::ProcessedResult, query::*};

use super::RawResult;

impl<'a> RawResult<GetContinuationsQuery<SearchQuery<'a, FilteredSearch>>> {
    pub fn process(
        self,
    ) -> Result<ProcessedResult<GetContinuationsQuery<SearchQuery<'a, FilteredSearch>>>> {
        match self {
            RawResult { query, json } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}
