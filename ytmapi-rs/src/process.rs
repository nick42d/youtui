use crate::auth::AuthToken;
use crate::crawler::JsonCrawlerBorrowed;
use crate::parse::ProcessedResult;
use crate::query::Query;
use crate::Result;
use std::marker::PhantomData;

/// The raw result of a query to the API.
// NOTE: The reason this is exposed in the public API, is that it is required to implement
// AuthToken.
#[derive(PartialEq, Debug)]
pub struct RawResult<'a, Q, A>
where
    Q: Query<A>,
    A: AuthToken,
{
    // A PhantomData is held to ensure token is processed correctly depending on the AuthToken that
    // generated it.
    token: PhantomData<A>,
    /// The query that generated this RawResult.
    pub query: &'a Q,
    /// The raw string output returned from the web request to YouTube.
    pub json: String,
}

impl<'a, Q: Query<A>, A: AuthToken> RawResult<'a, Q, A> {
    pub fn from_raw(json: String, query: &'a Q) -> Self {
        Self {
            query,
            token: PhantomData,
            json,
        }
    }
    pub fn destructure_json(self) -> String {
        self.json
    }
    pub fn process(self) -> Result<ProcessedResult<'a, Q>> {
        A::deserialize_json(self)
    }
}
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
// In the python API this returns None if /text/runs doesn't exist, but we are
// not doing that here. Checking should instead be done by receiver.
pub fn process_flex_column_item<'a>(
    item: &'a mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<JsonCrawlerBorrowed<'a>> {
    let pointer = format!("/flexColumns/{col_idx}/musicResponsiveListItemFlexColumnRenderer");
    item.borrow_pointer(pointer)
}
