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

pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{AlbumParams, ArtistParams, SearchResult};
use process::RawResult;
use query::{
    continuations::GetContinuationsQuery, FilteredSearch, GetAlbumQuery, GetArtistAlbumsQuery,
    GetArtistQuery, Query, SearchQuery, SearchType,
};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use utils::constants::{YTM_API_URL, YTM_PARAMS, YTM_PARAMS_KEY, YTM_URL};

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
    pub async fn from_header_file(path: &Path) -> Result<Self> {
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
    async fn raw_query<T: Query>(&self, query: T) -> Result<RawResult<T>> {
        // TODO: Handle errors
        // TODO: Continuations - as Stream?
        let url = format!("{YTM_API_URL}{}{YTM_PARAMS}{YTM_PARAMS_KEY}", query.path());
        let mut body = json!({
            "context" : {
                "client" : {
                    "clientName" : "WEB_REMIX",
                    "clientVersion" : self.client_version,
                }
            },
            query.header().key : query.header().value,
        });
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
            serde_json::from_str(&result).map_err(|_| Error::response(&result))?,
            query,
        );
        Ok(result)
    }
    // TODO: add use statements to cleanup path.
    pub async fn search<'a, S: SearchType>(
        &self,
        query: SearchQuery<'a, S>,
    ) -> Result<Vec<SearchResult<'a>>> {
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
    pub async fn get_continuations<S: SearchType>(
        &self,
        query: GetContinuationsQuery<SearchQuery<'_, FilteredSearch>>,
    ) -> Result<()> {
        self.raw_query(query).await?.process()?.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::common::{BrowseID, ChannelID};
    use super::query::*;
    use super::*;
    use crate::common::{AlbumID, YoutubeID};
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
