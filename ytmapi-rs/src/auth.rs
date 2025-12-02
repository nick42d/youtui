//! Available authorisation tokens.
use crate::Error;
use crate::client::{Client, QueryResponse};
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::query::{GetQuery, PostQuery};
use crate::utils::constants::{YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY};
pub use browser::BrowserToken;
use chrono::Utc;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use reqwest::Url;
use serde_json::json;
use std::borrow::Cow;
use std::marker::PhantomData;

pub mod browser;
pub mod noauth;
pub mod oauth;

/// An AuthToken is required to use the API.
/// AuthToken is reponsible for HTTP request headers, client_version and
/// performing the initial error checking and processing prior to parsing.
pub trait AuthToken: Sized {
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<'_, str>)>>;
    fn client_version(&self) -> Cow<'_, str>;
    fn deserialize_response<Q>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}

/// The raw result of a query to the API.
// NOTE: The reason this is exposed in the public API, is that it is required to implement
// AuthToken.
#[derive(PartialEq, Debug)]
pub struct RawResult<'a, Q, A>
where
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

impl<'a, Q, A: AuthToken> RawResult<'a, Q, A> {
    pub(crate) fn from_raw(json: String, query: &'a Q) -> Self {
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
        A::deserialize_response(self)
    }
}

pub(crate) async fn raw_query_post<'a, A: AuthToken, Q: PostQuery>(
    q: &'a Q,
    tok: &A,
    c: &Client,
) -> Result<RawResult<'a, Q, A>> {
    let url = format!("{YTM_API_URL}{}{YTM_PARAMS}{YTM_PARAMS_KEY}", q.path());
    let mut body = json!({
        "context" : {
            "client" : {
                "clientName" : "WEB_REMIX",
                "clientVersion" : tok.client_version(),
                "user" : {},
            },
        },
    });
    if let Some(body) = body.as_object_mut() {
        body.append(&mut q.header());
    } else {
        unreachable!("Body created in this function as an object")
    };
    let QueryResponse { text, .. } = c
        .post_json_query(url, tok.headers()?, &body, &q.params())
        .await?;
    Ok(RawResult::from_raw(text, q))
}

pub(crate) async fn raw_query_get<'a, Q: GetQuery, A: AuthToken>(
    tok: &A,
    client: &Client,
    query: &'a Q,
) -> Result<RawResult<'a, Q, A>> {
    let url = Url::parse_with_params(query.url(), query.params())
        .map_err(|e| Error::web(format!("{e}")))?;
    let result = client
        .get_query(url, tok.headers()?, &query.params())
        .await?;
    let result = RawResult::from_raw(result.text, query);
    Ok(result)
}

/// Marker trait to mark an AuthToken as LoggedIn
/// To allow Query implementors to write like
/// `impl<A: LoggedIn> Query<A> for AddSongToPlaylistQuery`
/// Since AuthToken is sealed, no-one else can implement this.
pub trait LoggedIn: AuthToken {}

impl LoggedIn for BrowserToken {}
impl LoggedIn for OAuthToken {}

/// Generate a dummy client version at the provided time.
/// Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L35C18-L35C31
fn fallback_client_version(time: &chrono::DateTime<Utc>) -> String {
    format!("1.{}.01.00", time.format("%Y%m%d"))
}
