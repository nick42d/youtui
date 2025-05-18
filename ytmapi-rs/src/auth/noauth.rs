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

#[derive(Clone, Serialize, Deserialize)]
pub struct NoAuthToken;

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
                    "clientVersion" : self.client_version,
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
            ("X-Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
            ("Authorization", format!("SAPISIDHASH {hash}").into()),
            ("Cookie", self.cookies.as_str().into()),
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
