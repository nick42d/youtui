use super::private::Sealed;
use super::AuthToken;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::{
    process::RawResult,
    query::Query,
    utils::constants::{
        OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, OAUTH_CODE_URL, OAUTH_GRANT_URL, OAUTH_SCOPE,
        OAUTH_TOKEN_URL, OAUTH_USER_AGENT, USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY,
        YTM_URL,
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

// The original reason for the two different structs was that we did not save
// the refresh token. But now we do, so consider simply making this only one
// struct. Otherwise the only difference is not including Scope which is not
// super relevant.
#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    token_type: String,
    access_token: String,
    refresh_token: String,
    expires_in: usize,
    request_time: SystemTime,
}
// TODO: Lock down construction of this type.
#[derive(Clone, Deserialize)]
pub struct OAuthDeviceCode(String);

#[derive(Clone, Deserialize)]
struct GoogleOAuthToken {
    pub access_token: String,
    pub expires_in: usize,
    pub refresh_token: String,
    // Unused currently - for future use
    #[allow(dead_code)]
    pub scope: String,
    pub token_type: String,
}
#[derive(Clone, Deserialize)]
struct GoogleOAuthRefreshToken {
    pub access_token: String,
    pub expires_in: usize,
    // Unused currently - for future use
    #[allow(dead_code)]
    pub scope: String,
    pub token_type: String,
}
#[derive(Clone, Deserialize)]
pub struct OAuthTokenGenerator {
    pub device_code: OAuthDeviceCode,
    pub expires_in: usize,
    pub interval: usize,
    pub user_code: String,
    pub verification_url: String,
}

impl OAuthToken {
    fn from_google_refresh_token(
        google_token: GoogleOAuthRefreshToken,
        request_time: SystemTime,
        refresh_token: String,
    ) -> Self {
        // See comment above on OAuthToken
        let GoogleOAuthRefreshToken {
            access_token,
            expires_in,
            token_type,
            ..
        } = google_token;
        Self {
            token_type,
            refresh_token,
            access_token,
            request_time,
            expires_in,
        }
    }
    fn from_google_token(google_token: GoogleOAuthToken, request_time: SystemTime) -> Self {
        // See comment above on OAuthToken
        let GoogleOAuthToken {
            access_token,
            expires_in,
            token_type,
            refresh_token,
            ..
        } = google_token;
        Self {
            token_type,
            refresh_token,
            access_token,
            request_time,
            expires_in,
        }
    }
}

impl OAuthDeviceCode {
    pub fn new(code: String) -> Self {
        Self(code)
    }
    pub fn get_code(&self) -> &str {
        &self.0
    }
}

impl Sealed for OAuthToken {}
impl AuthToken for OAuthToken {
    async fn raw_query<Q: Query>(
        &self,
        client: &Client,
        query: Q,
    ) -> Result<RawResult<Q, OAuthToken>> {
        // TODO: Functionize - used for Browser Auth as well.
        let url = format!("{YTM_API_URL}{}{YTM_PARAMS}{YTM_PARAMS_KEY}", query.path());
        let now_datetime: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let client_version = format!("1.{}.01.00", now_datetime.format("%Y%m%d"));
        let mut body = json!({
            "context" : {
                "client" : {
                    "clientName" : "WEB_REMIX",
                    "clientVersion" : client_version,
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
        let request_time_unix = self
            .request_time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::other("Error calculating time since unix epoch"))?
            .as_secs();
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::other("Error calculating time since unix epoch"))?
            .as_secs();
        // TODO: Better handling for expiration case.
        if now_unix + 3600 > request_time_unix + self.expires_in as u64 {
            return Err(Error::oauth_token_expired());
        }
        let result = client
            // Could include gzip deflation in headers - may improve performance?
            .post(&url)
            // TODO: Confirm if parsing for expired user agent also relevant here.
            .header("User-Agent", USER_AGENT)
            .header("X-Origin", YTM_URL)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("{} {}", self.token_type, self.access_token),
            )
            .header("X-Goog-Request-Time", request_time_unix)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        let result = RawResult::from_raw(result, query);
        Ok(result)
    }
    fn deserialize_json<Q: Query>(
        raw: RawResult<Q, Self>,
    ) -> Result<crate::parse::ProcessedResult<Q>> {
        let (json, query) = raw.destructure();
        ProcessedResult::from_raw(json, query)
    }
}

impl OAuthToken {
    pub async fn from_code(client: &Client, code: OAuthDeviceCode) -> Result<OAuthToken> {
        let body = json!({
            "client_secret" : OAUTH_CLIENT_SECRET,
            "grant_type" : OAUTH_GRANT_URL,
            "code": code.get_code(),
            "client_id" : OAUTH_CLIENT_ID
        });
        let result = client
            .post(OAUTH_TOKEN_URL)
            .header("User-Agent", OAUTH_USER_AGENT)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        let google_token: GoogleOAuthToken =
            serde_json::from_str(&result).map_err(|_| Error::response(&result))?;
        Ok(OAuthToken::from_google_token(
            google_token,
            SystemTime::now(),
        ))
    }
    pub async fn refresh(&self, client: &Client) -> Result<OAuthToken> {
        let body = json!({
            "client_secret" : OAUTH_CLIENT_SECRET,
            "grant_type" : "refresh_token",
            "refresh_token" : self.refresh_token,
            "client_id" : OAUTH_CLIENT_ID,
        });
        let result = client
            .post(OAUTH_TOKEN_URL)
            .header("User-Agent", OAUTH_USER_AGENT)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        let google_token: GoogleOAuthRefreshToken = serde_json::from_str(&result)
            .map_err(|e| Error::unable_to_serialize_oauth(&result, e))?;
        Ok(OAuthToken::from_google_refresh_token(
            google_token,
            SystemTime::now(),
            // TODO: Remove clone.
            self.refresh_token.clone(),
        ))
    }
}

impl OAuthTokenGenerator {
    pub async fn new(client: &Client) -> Result<OAuthTokenGenerator> {
        let body = json!({
            "scope" : OAUTH_SCOPE,
            "client_id" : OAUTH_CLIENT_ID
        });
        let result = client
            .post(OAUTH_CODE_URL)
            .header("User-Agent", OAUTH_USER_AGENT)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(serde_json::from_str(&result).map_err(|_| Error::response(&result))?)
    }
}
