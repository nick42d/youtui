pub mod constants {
    use const_format::concatcp;

    pub const USER_AGENT: &str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0";
    pub const YTM_URL: &str = "https://music.youtube.com";
    pub const YTM_API_URL: &str = "https://music.youtube.com/youtubei/v1/";
    pub const YTM_PARAMS: &str = "?alt=json&prettyPrint=false";
    pub const YTM_PARAMS_KEY: &str = "&key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";
    pub const OAUTH_CLIENT_ID: &str =
        "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com";
    pub const OAUTH_CLIENT_SECRET: &str = "SboVhoG9s0rNafixCSGGKXAT";
    pub const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/youtube";
    pub const OAUTH_CODE_URL: &str = "https://www.youtube.com/o/oauth2/device/code";
    pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
    pub const OAUTH_USER_AGENT: &str = concatcp!(USER_AGENT, " Cobalt/Version");
    pub const OAUTH_GRANT_URL: &str = "http://oauth.net/grant_type/device/1.0";
}
use constants::YTM_URL;
use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};
/// Calculates the Authorization hash from Google's SAPISID.
/// https://stackoverflow.com/a/32065323/5726546
/// Returns "{elapsed_since_epoch}_{hashed_sapisid}"
// TODO: Add Doctest
// TODO: Modify to be testable.
// Consider if this should take origin from headers instead of using the constant YTM_URL which
// I have modified.
pub fn hash_sapisid(sapisid: &str) -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime::now() is ahead of UNIX_EPOCH")
        .as_secs();
    let mut hasher = Sha1::new();
    hasher.update(format!("{elapsed} {sapisid} {YTM_URL}"));
    let result = hasher.finalize();
    let mut hex = String::new();
    for b in result {
        hex.push_str(&format!("{b:02x}"));
    }
    format!("{elapsed}_{hex}")
}
