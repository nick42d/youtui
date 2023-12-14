use crate::auth::{AuthToken, BrowserToken, OAuthToken};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::parse::ProcessedResult;
use crate::query::Query;
use crate::{error, Result};

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
}
impl<'tok, Q: Query> RawResult<'tok, Q, OAuthToken> {
    pub fn process(self) -> Result<ProcessedResult<Q>> {
        match self {
            // TODO: error handling here
            RawResult { query, json, .. } => {
                let json_cloner = JsonCloner::from_string(json)
                    .map_err(|_| error::Error::response("Error deserializing"))?;
                Ok(ProcessedResult::from_raw(
                    JsonCrawler::from_json_cloner(json_cloner),
                    query,
                ))
            }
        }
    }
}
impl<'tok, Q: Query> RawResult<'tok, Q, BrowserToken> {
    pub fn process(self) -> Result<ProcessedResult<Q>> {
        match self {
            // TODO: error handling here
            // // todo: better error
            // let result: serde_json::value =
            //     serde_json::from_str(&result).map_err(|_| error::response(&result))?;
            // // guard against error codes in json response.
            // // todo: can we determine if this is because the cookie has expired?
            // // todo: add a test for this
            // if let some(error) = result.get("error") {
            //     let some(code) = error.get("code").and_then(|code| code.as_u64()) else {
            //         return err(error::other(
            //             "error message received from server, but doesn't have an error code",
            //         ));
            //     };
            //     match code {
            //         401 => return err(error::not_authenticated()),
            //         other => return err(error::other_code(other)),
            //     }
            // }
            RawResult { query, json, .. } => {
                let json_cloner = JsonCloner::from_string(json)
                    .map_err(|_| error::Error::response("Error serializing"))?;
                Ok(ProcessedResult::from_raw(
                    JsonCrawler::from_json_cloner(json_cloner),
                    query,
                ))
            }
        }
    }
}
