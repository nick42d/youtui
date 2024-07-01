use crate::query::{AddHistoryItemQuery, GetHistoryQuery, RemoveHistoryItemsQuery};

use super::ParseFrom;

impl ParseFrom<GetHistoryQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetHistoryQuery>,
    ) -> crate::Result<<GetHistoryQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl ParseFrom<AddHistoryItemQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<AddHistoryItemQuery>,
    ) -> crate::Result<<AddHistoryItemQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<RemoveHistoryItemsQuery<'a>> for () {
    fn parse_from(
        p: super::ProcessedResult<RemoveHistoryItemsQuery>,
    ) -> crate::Result<<RemoveHistoryItemsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
