use super::RawResult;
use crate::crawler::JsonCrawler;
use crate::parse::ProcessedResult;
use crate::query::{GetLibraryArtistsQuery, GetLibraryPlaylistQuery};
use crate::Result;

impl<'a> RawResult<GetLibraryPlaylistQuery> {
    pub fn process(self) -> Result<ProcessedResult<GetLibraryPlaylistQuery>> {
        match self {
            RawResult { query, json } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}

impl<'a> RawResult<GetLibraryArtistsQuery> {
    pub fn process(self) -> Result<ProcessedResult<GetLibraryArtistsQuery>> {
        match self {
            RawResult { query, json } => Ok(ProcessedResult::from_raw(
                JsonCrawler::from_json(json),
                query,
            )),
        }
    }
}
