#![feature(async_fn_in_trait)]

mod auth;
mod utils;
mod locales {}
mod nav_consts;
// Consider if pub is correct for this
pub mod common;
mod crawler;
mod error;
pub mod parse;
mod process;
pub mod query;

#[cfg(test)]
mod tests;

use std::path::Path;

use auth::{
    browser::BrowserToken, oauth::OAuthDeviceCode, Auth, AuthToken, OAuthToken, OAuthTokenGenerator,
};
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

#[derive(Debug, Clone, Default)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
pub struct YtMusic {
    // TODO: add language
    // TODO: add location
    client: Client,
    auth: Auth,
}

impl YtMusic {
    /// Create a new API handle using an OAuthToken.
    pub async fn from_oauth_token(token: OAuthToken) -> Result<Self> {
        let client = Client::new();
        let auth = Auth::OAuth(token);
        Ok(Self { client, auth })
    }
    /// Create a new API handle using browser authentication details saved to a file on disk.
    /// The file should contain the Cookie response from a real logged in browser interaction with YouTube Music.
    pub async fn from_header_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
        let auth = Auth::Browser(BrowserToken::from_header_file(path, &client).await?);
        Ok(Self { client, auth })
    }
    /// Generates a tuple containing fresh OAuthDeviceCode and corresponding url for you to authenticate yourself at.
    /// (OAuthDeviceCode, URL)
    pub async fn generate_oauth_code_and_url(&self) -> Result<(OAuthDeviceCode, String)> {
        let code = OAuthTokenGenerator::new(&self.client).await?;
        let url = format!("{}?user_code={}", code.verification_url, code.user_code);
        Ok((code.device_code, url))
    }
    pub async fn generate_oauth_token(&self, code: OAuthDeviceCode) -> Result<OAuthToken> {
        OAuthToken::from_code(&self.client, code).await
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
