use reqwest::Client;
use std::env;
use std::path::Path;
use ytmapi_rs::common::ChannelID;
use ytmapi_rs::common::{LyricsID, PlaylistID, TextRun, YoutubeID};
use ytmapi_rs::parse::LikeStatus;
use ytmapi_rs::query::*;
use ytmapi_rs::Error;
use ytmapi_rs::{auth::*, *};

const COOKIE_PATH: &str = "cookie.txt";
const EXPIRED_OAUTH_PATH: &str = "oauth.json";
// Cookie filled with nonsense values to test this case.
const INVALID_COOKIE: &str = "HSID=abc; SSID=abc; APISID=abc; SAPISID=abc; __Secure-1PAPISID=abc; __Secure-3PAPISID=abc; YSC=abc; LOGIN_INFO=abc; VISITOR_INFO1_LIVE=abc; _gcl_au=abc; PREF=tz=Australia.Perth&f6=40000000&f7=abc; VISITOR_PRIVACY_METADATA=abc; __Secure-1PSIDTS=abc; __Secure-3PSIDTS=abc; SID=abc; __Secure-1PSID=abc; __Secure-3PSID=abc; SIDCC=abc; __Secure-1PSIDCC=abc; __Secure-3PSIDCC=abc";
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

async fn new_standard_oauth_api() -> Result<YtMusic<OAuthToken>> {
    let oauth_token = if let Ok(tok) = env::var("youtui_test_oauth") {
        tok
    } else {
        tokio::fs::read_to_string(EXPIRED_OAUTH_PATH).await.unwrap()
    };
    Ok(YtMusic::from_oauth_token(
        serde_json::from_slice(oauth_token.as_bytes()).unwrap(),
    ))
}
async fn new_standard_api() -> Result<YtMusic<BrowserToken>> {
    if let Ok(cookie) = env::var("youtui_test_cookie") {
        YtMusic::from_cookie(cookie).await
    } else {
        YtMusic::from_cookie_file(Path::new(COOKIE_PATH)).await
    }
}
pub fn write_json(e: &Error) {
    if let Some((json, key)) = e.get_json_and_key() {
        std::fs::write("err.json", json)
            .unwrap_or_else(|_| eprintln!("Error writing json to err.json"));
        panic!("{key} not found, wrote to err.json");
    }
}

#[tokio::test]
async fn test_refresh_expired_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    api.refresh_token().await.unwrap();
}

#[tokio::test]
async fn test_get_oauth_code() {
    let client = Client::new();
    let _code = OAuthTokenGenerator::new(&client).await.unwrap();
}
#[test]
fn test() {
    assert!(1 == 2)
}
