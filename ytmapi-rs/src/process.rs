use crate::auth::AuthToken;
use crate::crawler::JsonCrawlerBorrowed;
use crate::parse::ProcessedResult;
use crate::query::Query;
use crate::Result;

// Should trait be Result?
/// The raw result of a query to the API.
#[derive(PartialEq, Debug)]
pub struct RawResult<'tok, Q, A>
where
    Q: Query,
    A: AuthToken,
{
    query: Q,
    token: &'tok A,
    json: String,
}

pub(crate) struct JsonCloner {
    string: String,
    json: serde_json::Value,
}
// TODO: Return local error.
impl JsonCloner {
    pub fn from_string(string: String) -> std::result::Result<Self, serde_json::Error> {
        Ok(Self {
            json: serde_json::from_str(string.as_ref())?,
            string,
        })
    }
    pub fn destructure(self) -> (String, serde_json::Value) {
        let Self { string, json } = self;
        (string, json)
    }
}

impl<'tok, Q: Query, A: AuthToken> RawResult<'tok, Q, A> {
    pub fn from_raw(json: String, query: Q, token: &'tok A) -> Self {
        Self { query, token, json }
    }
    pub fn get_query(&self) -> &Q {
        &self.query
    }
    pub fn get_json(&self) -> &str {
        &self.json
    }
    pub fn destructure_json(self) -> String {
        self.json
    }
    pub fn destructure(self) -> (String, Q) {
        (self.json, self.query)
    }
    pub fn process(self) -> Result<ProcessedResult<Q>> {
        A::serialize_json(self)
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
// In the python API this returns None if /text/runs doesn't exist, but we are not doing that here.
// Checking should instead be done by receiver.
pub fn process_flex_column_item<'a>(
    item: &'a mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<JsonCrawlerBorrowed<'a>> {
    let pointer = format!("/flexColumns/{col_idx}/musicResponsiveListItemFlexColumnRenderer");
    item.borrow_pointer(pointer)
}
