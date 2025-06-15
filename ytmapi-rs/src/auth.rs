//! Available authorisation tokens.
use crate::client::{Client, QueryResponse};
use crate::error::Result;
use crate::parse::ProcessedResult;
use crate::process::RawResult;
use crate::query::{GetQuery, PostQuery, Query};
use crate::utils::constants::{YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY};
use crate::Error;
pub use browser::BrowserToken;
use chrono::Utc;
pub use oauth::{OAuthToken, OAuthTokenGenerator};
use reqwest::Url;
use serde_json::json;
use std::borrow::Cow;

pub mod browser;
pub mod noauth;
pub mod oauth;

mod private {
    pub trait Sealed {}
}

pub trait AuthToken: Sized {
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<str>)>>;
    fn client_version(&self) -> Cow<str>;
    fn deserialize_response<Q: Query<Self>>(raw: RawResult<Q, Self>) -> Result<ProcessedResult<Q>>;
}

pub(crate) async fn raw_query_post<'a, A: AuthToken, Q: Query<A> + PostQuery>(
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

pub(crate) async fn raw_query_get<'a, Q: GetQuery + Query<A>, A: AuthToken>(
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
