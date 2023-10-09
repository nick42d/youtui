use crate::crawler::JsonCrawler;
use crate::nav_consts::*;
use crate::parse::ProcessedResult;
use crate::query::*;
use crate::Result;
use const_format::concatcp;

use super::RawResult;
impl<'a> RawResult<GetArtistQuery<'a>> {
    pub fn process(self) -> Result<ProcessedResult<GetArtistQuery<'a>>> {
        match self {
            RawResult { query, json } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}
impl<'a> RawResult<GetArtistAlbumsQuery<'a>> {
    // Not sure if this is correct and should take mut self or not. It could instead involve
    // references but then will require lifetimes.
    pub fn process(self) -> Result<ProcessedResult<GetArtistAlbumsQuery<'a>>> {
        let RawResult { json, query } = self;
        let json_crawler = JsonCrawler::from_json(json).navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        Ok(ProcessedResult::from_raw(json_crawler, query))
    }
}
