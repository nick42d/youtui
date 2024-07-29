//! Available authorisation tokens.
use self::private::Sealed;
use crate::client::Client;
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::query::{GetQuery, PostQuery};
use crate::{process::RawResult, query::Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};

pub mod browser;
pub mod oauth;

// Seal AuthToken for now, due to instability of async trait currently.
mod private {
    pub trait Sealed {}
}
/// An authentication token into Youtube Music that can be used to query the
/// API.
// Allow async_fn_in_trait, as trait currently sealed.
#[allow(async_fn_in_trait)]
pub trait AuthToken: Sized + Sealed {
    // TODO: Continuations - as Stream?
    /// Run a post query that returns a raw json response.
    async fn raw_query_post<Q: PostQuery + Query<Self>>(
        &self,
        client: &Client,
        query: Q,
    ) -> Result<RawResult<Q, Self>>;
    /// Run a get query that returns a raw json response.
    async fn raw_query_get<Q: GetQuery + Query<Self>>(
        &self,
        client: &Client,
        query: Q,
    ) -> Result<RawResult<Q, Self>>;
    /// Process the result, by deserializing into JSON and checking for errors.
    fn deserialize_json<Q: Query<Self>>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}
