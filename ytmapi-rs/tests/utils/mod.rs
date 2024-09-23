use std::{env, path::Path};
use ytmapi_rs::{
    auth::{BrowserToken, OAuthToken}, Result, YtMusic,
};

pub const COOKIE_PATH: &str = "cookie.txt";
pub const EXPIRED_OAUTH_PATH: &str = "oauth.json";
// Cookie filled with nonsense values to test this case.
pub const INVALID_COOKIE: &str = "HSID=abc; SSID=abc; APISID=abc; SAPISID=abc; __Secure-1PAPISID=abc; __Secure-3PAPISID=abc; YSC=abc; LOGIN_INFO=abc; VISITOR_INFO1_LIVE=abc; _gcl_au=abc; PREF=tz=Australia.Perth&f6=40000000&f7=abc; VISITOR_PRIVACY_METADATA=abc; __Secure-1PSIDTS=abc; __Secure-3PSIDTS=abc; SID=abc; __Secure-1PSID=abc; __Secure-3PSID=abc; SIDCC=abc; __Secure-1PSIDCC=abc; __Secure-3PSIDCC=abc";
// Placeholder for future implementation.
// const INVALID_EXPIRED_OAUTH: &str = "
// {
//   \"token_type\": \"Bearer\",
//   \"access_token\": \"abc\",
//   \"refresh_token\": \"abc\",
//   \"expires_in\": 62609,
//   \"request_time\": {
//     \"secs_since_epoch\": 1702907669,
//     \"nanos_since_epoch\": 594642820
//   }
// }";

// It may be possible to put these inside a static, but last time I tried I kept
// getting web errors.
pub async fn new_standard_oauth_api() -> Result<YtMusic<OAuthToken>> {
    let oauth_token = if let Ok(tok) = env::var("youtui_test_oauth") {
        tok
    } else {
        tokio::fs::read_to_string(EXPIRED_OAUTH_PATH).await.unwrap()
    };
    Ok(YtMusic::from_oauth_token(
        serde_json::from_slice(oauth_token.as_bytes()).unwrap(),
    ))
}
// It may be possible to put these inside a static, but last time I tried I kept
// getting web errors.
pub async fn new_standard_api() -> Result<YtMusic<BrowserToken>> {
    if let Ok(cookie) = env::var("youtui_test_cookie") {
        YtMusic::from_cookie(cookie).await
    } else {
        YtMusic::from_cookie_file(Path::new(COOKIE_PATH)).await
    }
}

/// Macro to generate both oauth and browser tests for provided query.
/// May not really need a macro for this, could use a function.
// TODO: generalise
macro_rules! generate_query_test {
    ($fname:ident,$query:expr) => {
        #[tokio::test]
        async fn $fname() {
            let oauth_future = async {
                let mut api = crate::utils::new_standard_oauth_api().await.unwrap();
                // Don't stuff around trying the keep the local OAuth secret up to date, just
                // refresh it each time.
                api.refresh_token().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under oauth");
            };
            let browser_auth_future = async {
                let api = crate::utils::new_standard_api().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under browser auth");
            };
            tokio::join!(oauth_future, browser_auth_future);
        }
    };
}

/// Macro to generate both oauth and browser tests for provided query.
/// May not really need a macro for this, could use a function.
// TODO: generalise
macro_rules! generate_stream_test {
    ($fname:ident,$query:expr) => {
        #[tokio::test]
        async fn $fname() {
            use futures::stream::{StreamExt, TryStreamExt};
            let oauth_future = async {
                let mut api = crate::utils::new_standard_oauth_api().await.unwrap();
                // Don't stuff around trying the keep the local OAuth secret up to date, just
                // refresh it each time.
                api.refresh_token().await.unwrap();
                let query = $query;
                let stream = api.stream(&query);
                tokio::pin!(stream);
                stream
                    .try_collect::<Vec<_>>()
                    .await
                    .expect("Expected all results from oauth stream to suceed");
            };
            let browser_auth_future = async {
                let api = crate::utils::new_standard_api().await.unwrap();
                let query = $query;
                let stream = api.stream(&query);
                tokio::pin!(stream);
                stream
                    // limit test to 5 results to avoid overload
                    .take(5)
                    .try_collect::<Vec<_>>()
                    .await
                    .expect("Expected all results from browser stream to suceed");
            };
            tokio::join!(oauth_future, browser_auth_future);
        }
    };
}
