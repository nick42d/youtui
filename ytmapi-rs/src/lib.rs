// TODO Confirm if should be pub
pub mod auth;
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
use common::{
    browsing::Lyrics,
    library::{LibraryArtist, Playlist},
    watch::WatchPlaylist,
    SearchSuggestion, TextRun,
};
pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{AlbumParams, ArtistParams, SearchResult};
use process::RawResult;
use query::{
    continuations::GetContinuationsQuery, lyrics::GetLyricsQuery, watch::GetWatchPlaylistQuery,
    FilteredSearch, GetAlbumQuery, GetArtistAlbumsQuery, GetArtistQuery, GetLibraryArtistsQuery,
    GetLibraryPlaylistsQuery, GetSearchSuggestionsQuery, Query, SearchQuery, SearchType,
};
use reqwest::Client;
use utils::constants::{USER_AGENT, YTM_URL};

#[derive(Debug, Clone)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
pub struct YtMusic {
    // TODO: add language
    // TODO: add location
    client: Client,
    auth: Auth,
}

impl YtMusic {
    // TODO: Methods to create with a pre-existing client.
    pub fn get_auth_type(&self) -> &Auth {
        &self.auth
    }
    pub fn set_auth_type_oauth(&mut self, token: OAuthToken) {
        self.auth = Auth::OAuth(token)
    }
    pub async fn set_auth_type_header<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let token = BrowserToken::from_cookie_file(path, &self.client).await?;
        self.auth = Auth::Browser(token);
        Ok(())
    }
    /// Create a new API handle using an OAuthToken.
    pub fn from_oauth_token(token: OAuthToken) -> Self {
        let client = Client::new();
        let auth = Auth::OAuth(token);
        Self { client, auth }
    }
    /// Create a new API handle using a BrowserToken.
    pub fn from_browser_token(token: BrowserToken) -> Self {
        let client = Client::new();
        let auth = Auth::Browser(token);
        Self { client, auth }
    }
    /// Create a new API handle using browser authentication details saved to a file on disk.
    /// The file should contain the Cookie response from a real logged in browser interaction with YouTube Music.
    pub async fn from_cookie_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
        let auth = Auth::Browser(BrowserToken::from_cookie_file(path, &client).await?);
        Ok(Self { client, auth })
    }
    /// Create a new API handle using browser authentication details in a String.
    pub async fn from_cookie<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::new();
        let auth = Auth::Browser(BrowserToken::from_str(cookie.as_ref(), &client).await?);
        Ok(Self { client, auth })
    }
    async fn raw_query<Q: Query>(&self, query: Q) -> Result<RawResult<Q>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.auth.raw_query(&self.client, query).await
    }
    pub async fn json_query<Q: Query>(&self, query: Q) -> Result<serde_json::Value> {
        let json = self.raw_query(query).await?.destructure_json();
        Ok(json)
    }
    // TODO: add use statements to cleanup path.
    // XXX: Consider taking into<SearchQuery>
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
    pub async fn get_search_suggestions<'a, S: Into<GetSearchSuggestionsQuery<'a>>>(
        &self,
        query: S,
    ) -> Result<Vec<SearchSuggestion>> {
        self.raw_query(query.into()).await?.process()?.parse()
    }
    pub async fn get_library_playlists(&self) -> Result<Vec<Playlist>> {
        // TODO: investigate why returning empty array
        self.raw_query(GetLibraryPlaylistsQuery)
            .await?
            .process()?
            .parse()
    }
    pub async fn get_library_artists(
        // TODO: investigate why returning empty array
        // TODO: Better constructor for query
        &self,
        query: GetLibraryArtistsQuery,
    ) -> Result<Vec<LibraryArtist>> {
        self.raw_query(query).await?.process()?.parse()
    }
}
/// Generates a tuple containing fresh OAuthDeviceCode and corresponding url for you to authenticate yourself at.
/// (OAuthDeviceCode, URL)
pub async fn generate_oauth_code_and_url() -> Result<(OAuthDeviceCode, String)> {
    let client = Client::new();
    let code = OAuthTokenGenerator::new(&client).await?;
    let url = format!("{}?user_code={}", code.verification_url, code.user_code);
    Ok((code.device_code, url))
}

// TODO: Keep session alive after calling these methods.
/// Generates an OAuth Token when given an OAuthDeviceCode.
pub async fn generate_oauth_token(code: OAuthDeviceCode) -> Result<OAuthToken> {
    let client = Client::new();
    OAuthToken::from_code(&client, code).await
}
// TODO: Keep session alive after calling these methods.
/// Generates a Browser Token when given a browser cookie.
pub async fn generate_browser_token<S: AsRef<str>>(cookie: S) -> Result<BrowserToken> {
    let client = Client::new();
    BrowserToken::from_str(cookie.as_ref(), &client).await
}
