//! Available authorisation tokens.
use self::private::Sealed;
use crate::client::Client;
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::process::RawResult;
use crate::query::{GetQuery, PostQuery, Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};

pub mod browser;
pub mod oauth;

mod private {
    pub trait Sealed {}
}

/// An authentication token into Youtube Music that can be used to query the
/// API. Currently sealed due to use of async, although this could become open
/// for implementation in future.
// Allow async_fn_in_trait required, as trait currently sealed.
#[allow(async_fn_in_trait)]
pub trait AuthToken: Sized + Sealed {
    // TODO: Continuations - as Stream?
    /// Run a post query that returns a raw json response.
    async fn raw_query_post<'a, Q: PostQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>>;
    /// Run a get query that returns a raw json response.
    async fn raw_query_get<'a, Q: GetQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>>;
    /// Process the result, by deserializing into JSON and checking for errors.
    fn deserialize_json<Q: Query<Self>>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}
