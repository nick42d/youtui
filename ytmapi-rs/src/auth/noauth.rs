use super::private::Sealed;
use super::AuthToken;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::query::PostQuery;
use crate::{client, utils};
use crate::{
    process::RawResult,
    query::Query,
    utils::constants::{USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY, YTM_URL},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Debug;
use std::path::Path;
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct NoAuthToken {
    create_time: SystemTime,
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
        let result = client.get_query(YTM_URL, headers, &[]).await?;
        // Extract the parameter from inside the ytcfg.set() function.
        // Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L44
        let ytcfg_raw = result
            .split_once("ytcfg.set({")
            .unwrap()
            .1
            .split_once("})")
            .unwrap()
            .0;
        let ytcfg: serde_json::Value = serde_json::from_str(ytcfg_raw).unwrap();
        let visitor_id = ytcfg
            .as_object()
            .unwrap()
            .remove("VISITOR_DATA")
            .unwrap()
            // TODO: Remove allocation
            .as_str()
            .unwrap()
            .to_string();
        Ok(Self {
            create_time: std::time::SystemTime::now(),
            visitor_id,
        })
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
        let hash = utils::hash_sapisid(&self.sapisid);
        let headers = [
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("X-Goog-Visitor-Id", self.visitor_id),
            ("Content-Type", "application/json".into()),
        ];
        let result = client
            .post_query(url, headers, &body, &query.params())
            .await?;
        let result = RawResult::from_raw(result, query);
        Ok(result)
    }
    async fn raw_query_get<'a, Q: crate::query::GetQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>> {
        // COPY AND PASTE OF ABOVE.
        let hash = utils::hash_sapisid(&self.sapisid);
        let headers = [
            ("X-Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
            ("Authorization", format!("SAPISIDHASH {hash}").into()),
            ("Cookie", self.cookies.as_str().into()),
        ];
        let result = client
            .get_query(query.url(), headers, &query.params())
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
            match code {
                // Assuming Error:NotAuthenticated means browser token has expired.
                // May be incorrect - browser token may be invalid?
                // TODO: Investigate.
                401 => return Err(Error::browser_authentication_failed()),
                other => return Err(Error::other_code(other, message)),
            }
        }
        Ok(processed)
    }
}

/// Generate a dummy client version at the provided time.
/// Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L35C18-L35C31
fn fallback_client_version(time: &std::time::SystemTime) -> String {
    let time_formatted = "TODO";
    format!("1.{time_formatted}..01.00")
}
