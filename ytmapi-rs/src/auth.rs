//! Available authorisation tokens.
use self::private::Sealed;
use crate::client::{Client, QueryResponse};
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::process::RawResult;
use crate::query::{GetQuery, PostQuery, PostQueryCustom, Query};
pub use browser::BrowserToken;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use serde_json::json;
use std::borrow::Cow;

pub mod browser;
pub mod noauth;
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
    /// Run a post query with json as the body that returns a raw json response.
    async fn raw_query_post_json<'a, Q: PostQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>>;
    /// Run a post query with a file as the body that returns a raw json
    /// response.
    async fn raw_query_post<'a, Q: PostQueryCustom + Query<Self>>(
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

pub trait AuthToken2 {
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<str>)>>;
    fn client_version(&self) -> Cow<str>;
    // TODO: Should be generic across Self not BrowserToken.
    fn process_response<Q: Query<BrowserToken>>(
        raw: RawResult<Q, BrowserToken>,
    ) -> Result<ProcessedResult<Q>>;
}

async fn run_query<A: AuthToken2, Q: Query<BrowserToken> + PostQuery>(
    q: Q,
    tok: A,
    c: Client,
) -> Result<Q::Output> {
    let url = format!("TODO");
    let mut body = json!({
        "context" : {
            "client" : {
                "clientName" : "WEB_REMIX",
                "clientVersion" : tok.client_version(),
            },
        },
    });
    if let Some(body) = body.as_object_mut() {
        body.append(&mut q.header());
    } else {
        unreachable!("Body created in this function as an object")
    };
    let QueryResponse {
        text,
        status_code,
        headers,
    } = c
        .post_json_query(url, tok.headers().unwrap(), &body, &q.params())
        .await?;
    let result = RawResult::from_raw(text, &q);
    BrowserToken::deserialize_json(result).unwrap();
    Ok(result)
}

/// Marker trait to mark an AuthToken as LoggedIn
/// To allow Query implementors to write like
/// `impl<A: LoggedIn> Query<A> for AddSongToPlaylistQuery`
/// Since AuthToken is sealed, no-one else can implement this.
pub trait LoggedIn: AuthToken {}

impl LoggedIn for BrowserToken {}
impl LoggedIn for OAuthToken {}
