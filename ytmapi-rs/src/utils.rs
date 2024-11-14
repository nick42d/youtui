pub mod constants {
    use const_format::concatcp;

    pub const USER_AGENT: &str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0";
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
// Consider if this should take origin from headers instead of using the
// constant YTM_URL which I have modified.
pub fn hash_sapisid(sapisid: &str) -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime::now() should always be ahead of UNIX_EPOCH")
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

/// Macro to generate the boilerplate code that allows implementation of
/// YoutubeID for a simple struct. In addition implements a convenient From
/// implementation.
macro_rules! impl_youtube_id {
    ($t:ty) => {
        impl<'a> YoutubeID<'a> for $t {
            fn get_raw(&self) -> &str {
                &self.0
            }
            fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self {
                Self(raw_str.into())
            }
        }
        impl<'a> From<&'a $t> for $t {
            fn from(value: &'a $t) -> Self {
                let core = &value.0;
                Self(core.as_ref().into())
            }
        }
    };
}

/// Macro to generate a parsing test based on the following values:
/// May not really need a macro for this, could use a function.
/// Input file, output file, query, token
/// Note, this is async due to use of tokio::fs
#[cfg(test)]
macro_rules! parse_test {
    ($in:expr,$out:expr,$query:expr,$token:ty) => {
        let source_path = std::path::Path::new($in);
        let expected_path = std::path::Path::new($out);
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        let output = crate::process_json::<_, $token>(source, $query).unwrap();
        let output = format!("{:#?}", output);
        pretty_assertions::assert_eq!(expected, output);
    };
}

/// Macro to test that an input file succesfully parses directly against the
/// expected output value.
/// Input file, output value, query, token
/// Note, this is async due to use of tokio::fs
#[cfg(test)]
macro_rules! parse_test_value {
    ($in:expr,$out:expr,$query:expr,$token:ty) => {
        let source_path = std::path::Path::new($in);
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let parsed = crate::process_json::<_, $token>(source, $query).unwrap();
        pretty_assertions::assert_eq!(parsed, $out);
    };
}

/// Macro to generate a parsing test for continuations based on the following
/// values: May not really need a macro for this, could use a function.
/// Input file, output file, query, token
/// Note, this is async due to use of tokio::fs
#[cfg(test)]
macro_rules! parse_continuations_test {
    ($in:expr,$out:expr,$query:expr,$token:ty) => {
        let source_path = std::path::Path::new($in);
        let expected_path = std::path::Path::new($out);
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        let query = $query;
        let continuations_query = crate::query::GetContinuationsQuery::new_mock_unchecked(&query);
        let output = crate::process_json::<_, $token>(source, continuations_query).unwrap();
        let output = format!("{:#?}", output);
        pretty_assertions::assert_eq!(expected, output);
    };
}
