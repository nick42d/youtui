use super::private::Sealed;
use super::AuthToken;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::process::RawResult;
use crate::query::{GetQuery, PostQuery, Query};
use crate::utils::constants::{
    OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, OAUTH_CODE_URL, OAUTH_GRANT_URL, OAUTH_SCOPE,
    OAUTH_TOKEN_URL, OAUTH_USER_AGENT, USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY,
    YTM_URL,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

// The original reason for the two different structs was that we did not save
// the refresh token. But now we do, so consider simply making this only one
// struct. Otherwise the only difference is not including Scope which is not
// super relevant.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    async fn raw_query_post_json<'a, Q: PostQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, OAuthToken>> {
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
        } else {
            unreachable!("Body created in this function as an object")
        };
        let result = client
            .post_json_query(url, self.headers()?, &body, &query.params())
            .await?;
        let result = RawResult::from_raw(result, query);
        Ok(result)
    }
    async fn raw_query_get<'a, Q: GetQuery + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>> {
        let url = Url::parse_with_params(query.url(), query.params())
            .map_err(|e| Error::web(format!("{e}")))?;
        let result = client
            .get_query(url, self.headers()?, &query.params())
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

    async fn raw_query_post<'a, Q: crate::query::PostQueryCustom + Query<Self>>(
        &self,
        client: &Client,
        query: &'a Q,
    ) -> Result<RawResult<'a, Q, Self>> {
        todo!()
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
        let headers = [("User-Agent", OAUTH_USER_AGENT.into())];
        let result = client
            .post_json_query(OAUTH_TOKEN_URL, headers, &body, &())
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
        let headers = [("User-Agent", OAUTH_USER_AGENT.into())];
        let result = client
            .post_json_query(OAUTH_TOKEN_URL, headers, &body, &())
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
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<str>)>> {
        let request_time_unix = self.request_time.duration_since(UNIX_EPOCH)?.as_secs();
        let now_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        // TODO: Better handling for expiration case.
        if now_unix + 3600 > request_time_unix + self.expires_in as u64 {
            return Err(Error::oauth_token_expired(self));
        }
        Ok([
            // TODO: Confirm if parsing for expired user agent also relevant here.
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
            (
                "Authorization",
                format!("{} {}", self.token_type, self.access_token).into(),
            ),
            ("X-Goog-Request-Time", request_time_unix.to_string().into()),
        ])
    }
}

impl OAuthTokenGenerator {
    pub async fn new(client: &Client) -> Result<OAuthTokenGenerator> {
        let body = json!({
            "scope" : OAUTH_SCOPE,
            "client_id" : OAUTH_CLIENT_ID
        });
        let headers = [("User-Agent", OAUTH_USER_AGENT.into())];
        let result = client
            .post_json_query(OAUTH_CODE_URL, headers, &body, &())
            .await?;
        serde_json::from_str(&result).map_err(|_| Error::response(&result))
    }
}
// Don't use default Debug implementation for BrowserToken - contents are
// private
// TODO: Display some fields, such as time.
impl std::fmt::Debug for OAuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Private BrowserToken")
    }
}
