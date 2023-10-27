#![feature(async_fn_in_trait)]

mod utils;
mod auth {}
mod locales {}
pub mod common;
mod nav_consts;
// Consider if pub is correct for this
mod crawler;
mod error;
pub mod parse;
mod process;
pub mod query;

#[cfg(test)]
mod tests;

use common::{browsing::Lyrics, watch::WatchPlaylist, TextRun};
pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{AlbumParams, ArtistParams, SearchResult};
use process::RawResult;
use query::{
    continuations::GetContinuationsQuery, lyrics::GetLyricsQuery, watch::GetWatchPlaylistQuery,
    FilteredSearch, GetAlbumQuery, GetArtistAlbumsQuery, GetArtistQuery, GetSearchSuggestionsQuery,
    Query, SearchQuery, SearchType,
};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use utils::constants::{
    OAUTH_CODE_URL, OAUTH_TOKEN_URL, OAUTH_USER_AGENT, YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY,
    YTM_URL,
};

use crate::utils::constants::{OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, OAUTH_GRANT_URL, OAUTH_SCOPE};

/// An authentication token into Youtube Music that can be used to query the API.
trait AuthToken {
    // TODO: Continuations - as Stream?
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>>;
}

#[derive(Debug, Clone, Default)]
pub struct YtMusic {
    // TODO: add language
    // TODO: add location
    client: Client,
    auth: Auth,
}

#[derive(Debug, Clone, Default)]
enum Auth {
    OAuth(OAuthToken),
    Browser(BrowserToken),
    #[default]
    Unauthenticated,
}

#[derive(Debug, Clone)]
struct BrowserToken {
    sapisid: String,
    client_version: String,
    cookies: String,
}

#[derive(Debug, Clone)]
struct OAuthToken {
    // token_type: String,
    // access_token: String,
}

impl AuthToken for OAuthToken {
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>> {
        let result = client
            .post(OAUTH_CODE_URL)
            .header("User-Agent", OAUTH_USER_AGENT)
            .send()
            .await?
            .text()
            .await?;
        Err(Error::not_authenticated())
    }
}

impl AuthToken for Auth {
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>> {
        match self {
            Auth::OAuth(token) => token.raw_query(client, query).await,
            Auth::Browser(token) => token.raw_query(client, query).await,
            Auth::Unauthenticated => Err(Error::not_authenticated()),
        }
    }
}

impl AuthToken for BrowserToken {
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<RawResult<Q>> {
        // XXX: There is a test in here that I need to remove.
        let url = format!("{YTM_API_URL}{}{YTM_PARAMS}{YTM_PARAMS_KEY}", query.path());
        let test = json!({"test" : false});
        let mut body = json!({
            "context" : {
                "client" : {
                    "clientName" : "WEB_REMIX",
                    "clientVersion" : self.client_version,
                    "test" : test,
                }
            },
        });
        body.as_object_mut()
            .expect("I created body as an object")
            .append(&mut query.header());
        if let Some(q) = query.params() {
            body.as_object_mut()
                .expect("Body is an object")
                .insert("params".into(), q.into());
        }
        let hash = utils::hash_sapisid(&self.sapisid);
        let result = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("SAPISIDHASH {hash}"))
            .header("X-Origin", "https://music.youtube.com")
            .header("Cookie", &self.cookies)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        let result = RawResult::from_raw(
            // TODO: Better error
            serde_json::from_str(&result).map_err(|_| Error::response(&result))?,
            query,
        );
        Ok(result)
    }
}

impl BrowserToken {
    async fn from_header_file<P>(path: P, client: &Client) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let contents = tokio::fs::read_to_string(path).await.unwrap();
        let mut cookies = String::new();
        let mut user_agent = String::new();
        for l in contents.lines() {
            if let Some(c) = l.strip_prefix("Cookie:") {
                cookies = c.trim().to_string();
            }
            if let Some(u) = l.strip_prefix("User-Agent:") {
                user_agent = u.trim().to_string();
            }
        }
        let response = client
            .get(YTM_URL)
            .header(reqwest::header::COOKIE, &cookies)
            .header(reqwest::header::USER_AGENT, user_agent)
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
}

impl OAuthToken {
    async fn get_code(client: &Client) -> Result<serde_json::Value> {
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
    // You get the device code from the web logon. Should make it type safe.
    async fn get_token_from_code(
        client: &Client,
        device_code: String,
    ) -> Result<serde_json::Value> {
        let body = json!({
            "client_secret" : OAUTH_CLIENT_SECRET,
            "grant_type" : OAUTH_GRANT_URL,
            "code": device_code,
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
        Ok(serde_json::from_str(&result).map_err(|_| Error::response(&result))?)
    }
}

//TODO - Typesafe public interface
impl YtMusic {
    // pub async fn from_oauth_json<P>(path: P) -> Result<Self>
    // where
    //     P: AsRef<Path>,
    // {
    //     let client = Client::new();
    //     Ok(Self { client, auth })
    // }
    pub async fn from_header_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
        let auth = Auth::Browser(BrowserToken::from_header_file(path, &client).await?);
        Ok(Self { client, auth })
    }
    pub async fn setup_oauth(&self) -> Result<()> {
        let code = OAuthToken::get_code(&self.client).await?;
        let verification_url = code
            .get("verification_url")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let user_code = code
            .get("user_code")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let device_code = code
            .get("device_code")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string();
        let url = format!("{verification_url}?user_code={user_code}");
        // Hack method to pause whilst I login
        println!("Go to {url}, finish the login flow, and press enter when done");
        let mut _buf = String::new();
        let _ = std::io::stdin().read_line(&mut _buf);
        let token = OAuthToken::get_token_from_code(&self.client, device_code).await;
        println!("{:#?}", token);
        Ok(())
    }
    async fn raw_query<Q: Query>(&self, query: Q) -> Result<RawResult<Q>> {
        self.auth.raw_query(&self.client, query).await
    }
    pub async fn json_query<Q: Query>(&self, query: Q) -> Result<serde_json::Value> {
        let json = self.raw_query(query).await?.destructure_json();
        Ok(json)
    }
    // TODO: add use statements to cleanup path.
    pub async fn search<'a, S: SearchType>(
        &self,
        query: SearchQuery<'a, S>,
    ) -> Result<Vec<SearchResult<'a>>> {
        self.raw_query(query).await?.process()?.parse()
    }
    #[deprecated = "In progress, not complete"]
    pub async fn get_continuations<S: SearchType>(
        &self,
        query: GetContinuationsQuery<SearchQuery<'_, FilteredSearch>>,
    ) -> Result<()> {
        self.raw_query(query).await?.process()?.parse()
    }
    pub async fn get_artist(&self, query: GetArtistQuery<'_>) -> Result<ArtistParams> {
        self.raw_query(query).await?.process()?.parse()
    }
    pub async fn get_artist_albums(&self, query: GetArtistAlbumsQuery<'_>) -> Result<Vec<Album>> {
        self.raw_query(query).await?.process()?.parse()
    }
    pub async fn get_album(&self, query: GetAlbumQuery<'_>) -> Result<AlbumParams> {
        self.raw_query(query).await?.process()?.parse()
    }
    pub async fn get_lyrics(&self, query: GetLyricsQuery<'_>) -> Result<Lyrics> {
        self.raw_query(query).await?.process()?.parse()
    }
    pub async fn get_watch_playlist<'a, S: Into<GetWatchPlaylistQuery<VideoID<'a>>>>(
        &self,
        query: S,
    ) -> Result<WatchPlaylist> {
        self.raw_query(query.into()).await?.process()?.parse()
    }
    // TODO: Implement detailed runs function that highlights some parts of text bold.
    pub async fn get_search_suggestions<'a, S: Into<GetSearchSuggestionsQuery<'a>>>(
        &self,
        query: S,
    ) -> Result<Vec<Vec<TextRun>>> {
        self.raw_query(query.into()).await?.process()?.parse()
    }
}
