use super::private::Sealed;
use super::AuthToken;
use crate::client;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::query::PostQuery;
use crate::{
    process::RawResult,
    query::Query,
    utils::constants::{USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY, YTM_URL},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoAuthToken {
    create_time: chrono::DateTime<Utc>,
    visitor_id: String,
}

impl NoAuthToken {
    pub async fn new(client: &Client) -> Result<Self> {
        // COPY AND PASTE OF RAW_QUERY_GET.
        let headers = [
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
        ];
        let result = client.get_query(YTM_URL, headers, &()).await?;
        // Extract the parameter from inside the ytcfg.set() function.
        // Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L44
        let ytcfg_raw = result
            .split_once("ytcfg.set({")
            .unwrap()
            .1
            .split_once("})")
            .unwrap()
            .0
            .trim();
        let mut ytcfg: serde_json::Value = serde_json::from_str(&format!("{{{}}}", ytcfg_raw))
            .unwrap_or_else(|e| panic!("{{{ytcfg_raw}}} error {e}"));
        let visitor_id = ytcfg
            .as_object_mut()
            .unwrap()
            .remove("VISITOR_DATA")
            .unwrap()
            .as_str()
            .unwrap()
            // TODO: Remove allocation
            .to_string();
        Ok(Self {
            create_time: Utc::now(),
            visitor_id,
        })
    }
    fn headers(&self) -> impl IntoIterator<Item = (&str, Cow<str>)> {
        [
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("X-Goog-Visitor-Id", (&self.visitor_id).into()),
            ("Content-Type", "application/json".into()),
        ]
    }
}

impl Sealed for NoAuthToken {}
impl AuthToken for NoAuthToken {
    async fn raw_query_post<'a, Q: PostQuery + Query<Self>>(
        &self,
        client: &client::Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, NoAuthToken>> {
        // TODO: Functionize - used for OAuth as well.
        let url = format!("{YTM_API_URL}{}{YTM_PARAMS}{YTM_PARAMS_KEY}", query.path());
        let mut body = json!({
            "context" : {
                "client" : {
                    "clientName" : "WEB_REMIX",
                    "clientVersion" : fallback_client_version(&self.create_time),
                    "user" : {}
                },
            },
        });
        if let Some(body) = body.as_object_mut() {
            body.append(&mut query.header());
        } else {
            unreachable!("Body created in this function as an object")
        };
        let result = client
            .post_query(url, self.headers(), &body, &query.params())
            .await?;
        let result = RawResult::from_raw(result, query);
        Ok(result)
    }
    async fn raw_query_get<'a, Q: crate::query::GetQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>> {
        let result = client
            .get_query(query.url(), self.headers(), &query.params())
            .await?;
        let result = RawResult::from_raw(result, query);
        Ok(result)
    }
    fn deserialize_json<Q: Query<Self>>(
        raw: RawResult<Q, Self>,
    ) -> Result<crate::parse::ProcessedResult<Q>> {
        let processed = ProcessedResult::try_from(raw)?;
        // Guard against error codes in json response.
        // TODO: Add a test for this
        if let Some(error) = processed.get_json().pointer("/error") {
            let Some(code) = error.pointer("/code").and_then(|v| v.as_u64()) else {
                // TODO: Better error.
                return Err(Error::response("API reported an error but no code"));
            };
            let message = error
                .pointer("/message")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            return Err(Error::other_code(code, message));
        }
        Ok(processed)
    }
}

/// Generate a dummy client version at the provided time.
/// Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L35C18-L35C31
fn fallback_client_version(time: &chrono::DateTime<Utc>) -> String {
    format!("1.{}.01.00", time.format("%Y%m%d"))
}
