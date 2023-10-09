use crate::query::{continuations::GetContinuationsQuery, FilteredSearch, SearchQuery};
use crate::Result;

use super::ProcessedResult;

impl<'a> ProcessedResult<GetContinuationsQuery<SearchQuery<'a, FilteredSearch>>> {
    pub fn parse(self) -> Result<()> {
        Ok(())
    }
}
