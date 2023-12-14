use crate::error::{Error, Result};
use crate::utils;
use crate::{
    process::RawResult,
    query::Query,
    utils::constants::{USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY, YTM_URL},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

use super::AuthToken;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToken {
    sapisid: String,
    client_version: String,
    cookies: String,
}

impl AuthToken for BrowserToken {
    async fn raw_query<'a, Q: Query>(
        &'a self,
        client: &Client,
        query: Q,
    ) -> Result<RawResult<Q, BrowserToken>> {
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
            if let Some(q) = query.params() {
                body.insert("params".into(), q.into());
            }
        } else {
            unreachable!("Body created in this function as an object")
        };
        let hash = utils::hash_sapisid(&self.sapisid);
        let result = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("SAPISIDHASH {hash}"))
            .header("X-Origin", YTM_URL)
            .header("Cookie", &self.cookies)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        let result = RawResult::from_raw(result, query, self);
        Ok(result)
    }
}

impl BrowserToken {
    pub async fn from_str(cookie_str: &str, client: &Client) -> Result<Self> {
        let cookies = cookie_str.trim().to_string();
        let response = client
            .get(YTM_URL)
            .header(reqwest::header::COOKIE, &cookies)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .text()
            .await?;
        let client_version = response
            .split_once("INNERTUBE_CLIENT_VERSION\":\"")
            .ok_or(Error::header())?
            .1
            .split_once("\"")
            .ok_or(Error::header())?
            .0
            .to_string();
        let sapisid = cookies
            .split_once("SAPISID=")
            .ok_or(Error::header())?
            .1
            .split_once(";")
            .ok_or(Error::header())?
            .0
            .to_string();
        Ok(Self {
            sapisid,
            client_version,
            cookies,
        })
    }
    pub async fn from_cookie_file<P>(path: P, client: &Client) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let contents = tokio::fs::read_to_string(path).await.unwrap();
        BrowserToken::from_str(&contents, client).await
    }
}
