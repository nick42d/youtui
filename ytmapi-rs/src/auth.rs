use crate::error::Result;
use crate::{process::RawResult, query::Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use reqwest::Client;

pub mod browser;
pub mod oauth;

/// An authentication token into Youtube Music that can be used to query the API.
pub(crate) trait AuthToken {
    // TODO: Continuations - as Stream?
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>>;
}

#[derive(Debug, Clone)]
pub enum Auth {
    OAuth(OAuthToken),
    Browser(BrowserToken),
}

impl AuthToken for Auth {
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>> {
        match self {
            Auth::OAuth(token) => token.raw_query(client, query).await,
            Auth::Browser(token) => token.raw_query(client, query).await,
        }
    }
}
