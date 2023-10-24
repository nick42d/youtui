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

// TODO: Remove allocation requirement.
// TODO: Remove clone. Is just there for a hack in ui.rs
#[derive(Debug, Clone)]
pub struct YtMusic {
    // TODO: add language
    // TODO: add location
    client: Client, // XXX: rename to avoid confusion
    sapisid: String,
    client_version: String,
    cookies: String,
}

enum Auth {
    Oauth(OAuthToken),
    Browser,
}

struct OAuthToken {}

impl OAuthToken {
    async fn raw_query<Q: Query>(&self, client: &Client, query: Q) -> Result<()> {
        let result = client
            .post(OAUTH_CODE_URL)
            .header("User-Agent", OAUTH_USER_AGENT)
            .send()
            .await?
            .text()
            .await?;
        Ok(())
    }
    async fn get_code(&self, client: &Client) -> Result<serde_json::Value> {
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
        &self,
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

async fn setup_oauth() {
    let client = Client::new();
}

//TODO - Typesafe public interface
impl YtMusic {
    pub fn from_raw_parts(client_version: String, cookies: String, sapisid: String) -> Self {
        let client = Client::new();
        Self {
            client,
            client_version,
            cookies,
            sapisid,
        }
    }
    // TODO: Use OAuth
    // TODO: Handle errors
    // TODO: Path should be impl into path
    pub async fn from_header_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
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
            client,
            client_version,
            cookies,
        })
    }
    async fn raw_query<Q: Query>(&self, query: Q) -> Result<RawResult<Q>> {
        // TODO: Handle errors
        // TODO: Continuations - as Stream?
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
        let result = self
            .client
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

#[cfg(test)]
mod tests {
    use super::common::{BrowseID, ChannelID};
    use super::query::*;
    use super::*;
    use crate::common::{AlbumID, LyricsID, PlaylistID, YoutubeID};
    use crate::Error;

    #[tokio::test]
    async fn test_new() {
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn test_search() {
        let now = std::time::Instant::now();
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        println!("API took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let query = SearchQuery::new("Beatles")
            .set_filter(Filter::Artists)
            .set_spelling_mode(SpellingMode::ExactMatch);
        let res = api.search(query).await.unwrap();
        println!("Search took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        println!("Parse search took {} ms", now.elapsed().as_millis());
    }
    #[tokio::test]
    async fn test_watch_playlist() {
        // TODO: Make more generic
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        let res = api
            .get_watch_playlist(GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(
                "9mWr4c_ig54",
            )))
            .await
            .unwrap();
        let example = WatchPlaylist {
            _tracks: Vec::new(),
            playlist_id: Some(PlaylistID::from_raw("RDAMVM9mWr4c_ig54")),
            lyrics_id: LyricsID("MPLYt_C8aRK1qmsDJ-1".into()),
        };
        assert_eq!(res, example)
    }
    #[tokio::test]
    async fn test_get_lyrics() {
        // TODO: Make more generic
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        let res = api
            .get_watch_playlist(GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(
                "9mWr4c_ig54",
            )))
            .await
            .unwrap();
        let res = api
            .get_lyrics(GetLyricsQuery::new(res.lyrics_id))
            .await
            .unwrap();
        let example = Lyrics {
            lyrics: "You're my lesson I had to learn\nAnother page I'll have to turn\nI got one more message, always tryna be heard\nBut you never listen to a word\n\nHeaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nJust give me one second and I'll be fine\nJust let me catch my breath and come back to life\nI finally get the message, you were never meant to be mine\nCouldn't see the truth, I was blind (meant to be mine)\n\nWhoa, heaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nLetting go, we came so close (how foolish of me)\nOh, I'm letting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me".into(),
            source: "Source: Musixmatch".into(),
        };
        assert_eq!(res, example)
    }
    #[tokio::test]
    async fn test_search_suggestions() {
        // TODO: Make more generic
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        let res = api.get_search_suggestions("faded").await.unwrap();
        let example = vec![
            vec![TextRun::Bold("faded".into())],
            vec![
                TextRun::Bold("faded".into()),
                TextRun::Normal(" alan walker".into()),
            ],
            vec![
                TextRun::Bold("faded".into()),
                TextRun::Normal(" zhu".into()),
            ],
            vec![
                TextRun::Bold("faded".into()),
                TextRun::Normal(" kerser".into()),
            ],
            vec![
                TextRun::Bold("faded".into()),
                TextRun::Normal(" remix".into()),
            ],
        ];
        assert_eq!(res, example)
    }
    #[tokio::test]
    async fn test_get_artist() {
        let now = std::time::Instant::now();
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        println!("API took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = api
            .raw_query(GetArtistQuery::new(ChannelID::from_raw(
                "UC2XdaAVUannpujzv32jcouQ",
            )))
            .await
            .unwrap();
        println!("Get artist took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = res.process().unwrap();
        let res = res.parse().unwrap();
        println!("Parse artist took {} ms", now.elapsed().as_millis());
    }
    #[tokio::test]
    async fn test_get_artist_albums() {
        let now = std::time::Instant::now();
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        println!("API took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = api
            .raw_query(GetArtistQuery::new(ChannelID::from_raw(
                // Metallica
                "UCGexNm_Kw4rdQjLxmpb2EKw",
            )))
            .await
            .unwrap();
        println!("Get artist took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        // TODO: fix temporary value dropped while borrowed error.
        // This won't compile:
        // let res = res.process().unwrap().parse().unwrap();
        let res = res.process().unwrap();
        let res = res.parse().unwrap();
        println!("Parse artist took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let albums = res.top_releases.albums.unwrap();
        let params = albums.params.unwrap();
        // For some reason the params is wrong. needs investigation.
        let channel_id = &albums.browse_id.unwrap();
        let q = GetArtistAlbumsQuery::new(ChannelID::from_raw(channel_id.get_raw()), params);
        api.get_artist_albums(q).await.unwrap();
        let now = std::time::Instant::now();
        println!("Get albums took {} ms", now.elapsed().as_millis());
    }
    #[tokio::test]
    async fn test_get_oauth_code() {
        let client = Client::new();
        let oauth = OAuthToken {};
        let code = oauth.get_code(&client).await.unwrap();
        assert_eq!(json!({"hello": "world"}), code);
    }
    #[tokio::test]
    async fn test_get_oauth_token() {
        let client = Client::new();
        let oauth = OAuthToken {};
        let code = oauth.get_code(&client).await.unwrap();
        let token = oauth
            .get_token_from_code(
                &client,
                code.get("device_code")
                    .and_then(|s| serde_json::to_string(s).ok())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(json!({"hello": "world"}), token);
    }
    #[tokio::test]
    async fn test_get_artist_album_songs() {
        let now = std::time::Instant::now();
        let api = YtMusic::from_header_file(Path::new("headers.txt"))
            .await
            .unwrap();
        println!("API took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = api
            .raw_query(GetArtistQuery::new(ChannelID::from_raw(
                "UCGexNm_Kw4rdQjLxmpb2EKw",
            )))
            .await
            .unwrap();
        println!("Get artist took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        // TODO: fix temporary value dropped while borrowed error.
        // This won't compile:
        // let res = res.process().unwrap().parse().unwrap();
        let res = res.process().unwrap();
        let res = res.parse().unwrap();
        println!("Parse artist took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let albums = res.top_releases.albums.unwrap();
        let params = albums.params.unwrap();
        let channel_id = &albums.browse_id.unwrap();
        let res = api
            .raw_query(GetArtistAlbumsQuery::new(
                ChannelID::from_raw(channel_id.get_raw()),
                params,
            ))
            .await
            .unwrap();
        println!("Get albums took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = res.process().unwrap();
        let res = res.parse().unwrap();
        println!("Process albums took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let browse_id = AlbumID::from_raw(&res[0].browse_id);
        let res = api.raw_query(GetAlbumQuery::new(browse_id)).await.unwrap();
        println!("Get album took {} ms", now.elapsed().as_millis());
        let now = std::time::Instant::now();
        let res = res.process().map_err(|e| write_json(&e)).unwrap();
        let res = res.parse().unwrap();
        println!("Process album took {} ms", now.elapsed().as_millis());
    }
    pub fn write_json(e: &Error) {
        if let Some((json, key)) = e.get_json_and_key() {
            std::fs::write("err.json", json)
                .unwrap_or_else(|_| eprintln!("Error writing json to err.json"));
            panic!("{key} not found, wrote to err.json");
        }
    }
}
