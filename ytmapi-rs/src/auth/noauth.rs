use super::{AuthToken, RawResult, fallback_client_version};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::utils::constants::{USER_AGENT, YTM_URL};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoAuthToken {
    create_time: chrono::DateTime<Utc>,
    visitor_id: String,
}

impl NoAuthToken {
    pub async fn new(client: &Client) -> Result<Self> {
        let initial_headers = [
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
        ];
        let result_text = client.get_query(YTM_URL, initial_headers, &()).await?.text;
        // Extract the parameter from inside the ytcfg.set() function.
        // Original implementation: https://github.com/sigma67/ytmusicapi/blob/459bc40e4ce31584f9d87cf75838a1f404aa472d/ytmusicapi/helpers.py#L44
        let ytcfg_raw = result_text
            .split_once("ytcfg.set({")
            .ok_or_else(|| Error::ytcfg(&result_text))?
            .1
            .split_once("})")
            .ok_or_else(|| Error::ytcfg(&result_text))?
            .0
            .trim();
        let mut ytcfg: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&format!("{{{ytcfg_raw}}}"))
                .map_err(|_| Error::ytcfg(ytcfg_raw))?;
        let visitor_id = serde_json::from_value(
            ytcfg
                .remove("VISITOR_DATA")
                .ok_or_else(Error::no_visitor_data)?,
        )
        .map_err(|_| Error::no_visitor_data())?;
        Ok(Self {
            create_time: Utc::now(),
            visitor_id,
        })
    }
}

impl AuthToken for NoAuthToken {
    fn client_version(&self) -> Cow<'_, str> {
        fallback_client_version(&self.create_time).into()
    }
    fn deserialize_response<Q>(
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
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<'_, str>)>> {
        Ok([
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("X-Goog-Visitor-Id", (&self.visitor_id).into()),
            ("Content-Type", "application/json".into()),
        ])
    }
}
