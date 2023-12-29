//! Available authorisation tokens.
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::{process::RawResult, query::Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use reqwest::Client;

pub mod browser;
pub mod oauth;

// TODO: Seal and ignore warning.
/// An authentication token into Youtube Music that can be used to query the API.
pub trait AuthToken: Sized {
    // TODO: Continuations - as Stream?
    async fn raw_query<'a, Q: Query>(
        &'a self,
        client: &Client,
        query: Q,
    ) -> Result<RawResult<'a, Q, Self>>;
    fn serialize_json<Q: Query>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}
