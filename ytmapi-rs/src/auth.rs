//! Available authorisation tokens.
use self::private::Sealed;
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::{process::RawResult, query::Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use reqwest::Client;

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
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q, Self>>;
    /// Process the result, by deserializing into JSON.
    /// Current implementations do error checking against expected responses for
    /// the token here too.
    fn deserialize_json<Q: Query>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}
