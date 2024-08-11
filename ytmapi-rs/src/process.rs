use crate::auth::AuthToken;
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

pub fn fixed_column_item_pointer(col_idx: usize) -> String {
    format!("/fixedColumns/{col_idx}/musicResponsiveListItemFixedColumnRenderer")
}

pub fn flex_column_item_pointer(col_idx: usize) -> String {
    format!("/flexColumns/{col_idx}/musicResponsiveListItemFlexColumnRenderer")
}
