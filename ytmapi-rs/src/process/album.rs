use crate::crawler::JsonCrawler;
use crate::Result;
use crate::{parse::ProcessedResult, query::*};

use super::RawResult;

impl<'a> RawResult<GetAlbumQuery<'a>> {
    pub fn process(self) -> Result<ProcessedResult<GetAlbumQuery<'a>>> {
        match self {
            RawResult { query, json } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}
