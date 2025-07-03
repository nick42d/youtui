#![allow(clippy::unwrap_used)]
use std::env::{self, VarError};
use std::path::Path;
use tokio::sync::OnceCell;
use ytmapi_rs::auth::{BrowserToken, OAuthToken};
use ytmapi_rs::{Client, Result, YtMusic};

pub const COOKIE_PATH: &str = "cookie.txt";
pub const EXPIRED_OAUTH_PATH: &str = "oauth.json";
// Cookie filled with nonsense values to test this case.
// pub const INVALID_COOKIE: &str = "HSID=abc; SSID=abc; APISID=abc; SAPISID=abc; __Secure-1PAPISID=abc; __Secure-3PAPISID=abc; YSC=abc; LOGIN_INFO=abc; VISITOR_INFO1_LIVE=abc; _gcl_au=abc; PREF=tz=Australia.Perth&f6=40000000&f7=abc; VISITOR_PRIVACY_METADATA=abc; __Secure-1PSIDTS=abc; __Secure-3PSIDTS=abc; SID=abc; __Secure-1PSID=abc; __Secure-3PSID=abc; SIDCC=abc; __Secure-1PSIDCC=abc; __Secure-3PSIDCC=abc";
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

/// To avoid refreshing OAuthToken on every API call, it's refreshed on
/// initialization and stored here.
static OAUTH_TOKEN: OnceCell<OAuthToken> = OnceCell::const_new();

/// (client_id, client_secret)
pub fn get_oauth_client_id_and_secret() -> std::result::Result<(String, String), VarError> {
    let client_id = std::env::var("youtui_client_id")?;
    let client_secret = std::env::var("youtui_client_secret")?;
    Ok((client_id, client_secret))
}

// It may be possible to put these inside a static, but last time I tried I kept
// getting web errors.
// The cause of the web errors is that each tokio::test has its own runtime.
// To resolve this, we'll need a shared runtime as well as a static containing
// the API.
pub async fn new_standard_oauth_api() -> Result<YtMusic<OAuthToken>> {
    let oauth_token = OAUTH_TOKEN
        .get_or_init(|| async {
            let tok_str = if let Ok(tok) = env::var("youtui_test_oauth") {
                tok
            } else {
                tokio::fs::read_to_string(EXPIRED_OAUTH_PATH).await.unwrap()
            };
            let tok: OAuthToken = serde_json::from_slice(tok_str.as_bytes()).unwrap();
            let client = Client::new_rustls_tls().unwrap();
            tok.refresh(&client).await.unwrap();
            tok
        })
        .await;
    let mut api = YtMusic::from_auth_token(oauth_token.clone());
    api.refresh_token().await.unwrap();
    Ok(api)
}
// It may be possible to put these inside a static, but last time I tried I kept
// getting web errors.
// The cause of the web errors is that each tokio::test has its own runtime.
// To resolve this, we'll need a shared runtime as well as a static containing
// the API.
pub async fn new_standard_api() -> Result<YtMusic<BrowserToken>> {
    if let Ok(cookie) = env::var("youtui_test_cookie") {
        YtMusic::from_cookie(cookie).await
    } else {
        YtMusic::from_cookie_file(Path::new(COOKIE_PATH)).await
    }
}

/// Macro to generate both oauth and browser tests for provided query.
/// Attributes like #[ignore] can be passed as the optional first argument.
/// NOTE: Oauth not handled due to oauth issues.
///
/// https://github.com/nick42d/youtui/issues/179
macro_rules! generate_query_test_logged_in {
    ($(#[$m:meta])*
    $fname:ident,$query:expr) => {
        paste::paste! {
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _browser>]() {
                let api = crate::utils::new_standard_api().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under browser auth");
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _oauth>]() {
                let api = crate::utils::new_standard_oauth_api().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under oauth");
            }
        }
    };
}

/// Macro to generate noauth, oauth and browser tests for provided query.
/// Attributes like #[ignore] can be passed as the optional first argument.
/// NOTE: Oauth not handled due to oauth issues.
///
/// https://github.com/nick42d/youtui/issues/179
macro_rules! generate_query_test {
    ($(#[$m:meta])*
    $fname:ident,$query:expr) => {
        paste::paste! {
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _browser>]() {
                let api = crate::utils::new_standard_api().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under browser auth");
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _noauth>]() {
                let api = YtMusic::new_unauthenticated().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully without auth");
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _oauth>]() {
                let api = crate::utils::new_standard_oauth_api().await.unwrap();
                api.query($query)
                    .await
                    .expect("Expected query to run succesfully under oauth");
            }
        }
    };
}

/// Macro to generate noauth, oauth and browser tests for provided stream.
/// Attributes like #[ignore] can be passed as the optional first argument.
/// NOTE: Oauth not handled due to oauth issues.
///
/// https://github.com/nick42d/youtui/issues/179
macro_rules! generate_stream_test {
    ($(#[$m:meta])*
    $fname:ident,$query:expr) => {
        paste::paste! {
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _browser>]() {
                use futures::stream::{StreamExt, TryStreamExt};
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
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _noauth>]() {
                use futures::stream::{StreamExt, TryStreamExt};
                let api = YtMusic::new_unauthenticated().await.unwrap();
                let query = $query;
                let stream = api.stream(&query);
                tokio::pin!(stream);
                stream
                    // limit test to 5 results to avoid overload
                    .take(5)
                    .try_collect::<Vec<_>>()
                    .await
                    .expect("Expected all results from stream to succeed without auth");
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _oauth>]() {
                use futures::stream::{StreamExt, TryStreamExt};
                let api = crate::utils::new_standard_oauth_api().await.unwrap();
                let query = $query;
                let stream = api.stream(&query);
                tokio::pin!(stream);
                stream
                    // limit test to 5 results to avoid overload
                    .take(5)
                    .try_collect::<Vec<_>>()
                    .await
                    .expect("Expected all results from oauth stream to suceed");
            }
        }
    };
}

/// Macro to generate both oauth and browser tests for provided stream.
/// Attributes like #[ignore] can be passed as the optional first argument.
/// NOTE: Oauth not handled due to oauth issues.
///
/// https://github.com/nick42d/youtui/issues/179
macro_rules! generate_stream_test_logged_in {
    ($(#[$m:meta])*
    $fname:ident,$query:expr) => {
        paste::paste! {
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _browser>]() {
                use futures::stream::{StreamExt, TryStreamExt};
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
            }
            $(#[$m])*
            #[tokio::test]
            async fn [<$fname _oauth>]() {
                use futures::stream::{StreamExt, TryStreamExt};
                let api = crate::utils::new_standard_oauth_api().await.unwrap();
                let query = $query;
                let stream = api.stream(&query);
                tokio::pin!(stream);
                stream
                    // limit test to 5 results to avoid overload
                    .take(5)
                    .try_collect::<Vec<_>>()
                    .await
                    .expect("Expected all results from oauth stream to suceed");
            }
        }
    };
}
