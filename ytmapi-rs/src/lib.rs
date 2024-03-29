//! Library into YouTube Music's internal API.
//! ## Examples
//! Basic usage with a pre-created cookie file :
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let cookie_path = std::path::Path::new("./cookie.txt");
//!     let yt = ytmapi_rs::YtMusic::from_cookie_file(cookie_path).await?;
//!     yt.get_search_suggestions("Beatles").await?;
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
//! Basic usage - oauth:
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let (code, url) = ytmapi_rs::generate_oauth_code_and_url().await?;
//!     println!("Go to {url}, finish the login flow, and press enter when done");
//!     let mut _buf = String::new();
//!     let _ = std::io::stdin().read_line(&mut _buf);
//!     let token = ytmapi_rs::generate_oauth_token(code).await?;
//!     // NOTE: The token can be re-used until it expires, and refreshed once it has,
//!     // so it's recommended to save it to a file here.
//!     let yt = ytmapi_rs::YtMusic::from_oauth_token(token);
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
use auth::{
    browser::BrowserToken, oauth::OAuthDeviceCode, AuthToken, OAuthToken, OAuthTokenGenerator,
};
use common::{
    browsing::Lyrics,
    library::{LibraryArtist, Playlist},
    watch::WatchPlaylist,
    SearchSuggestion,
};
pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{
    AlbumParams, ArtistParams, Parse, SearchResultAlbum, SearchResultArtist, SearchResultEpisode,
    SearchResultFeaturedPlaylist, SearchResultPlaylist, SearchResultPodcast, SearchResultProfile,
    SearchResultSong, SearchResultVideo, SearchResults,
};
use process::RawResult;
use query::{
    lyrics::GetLyricsQuery, watch::GetWatchPlaylistQuery, AlbumsFilter, ArtistsFilter, BasicSearch,
    CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter, FilteredSearch,
    GetAlbumQuery, GetArtistAlbumsQuery, GetArtistQuery, GetLibraryArtistsQuery,
    GetLibraryPlaylistsQuery, GetSearchSuggestionsQuery, PlaylistsFilter, PodcastsFilter,
    ProfilesFilter, Query, SearchQuery, SongsFilter, VideosFilter,
};
use reqwest::Client;
use std::path::Path;

// TODO: Confirm if auth should be pub
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

#[derive(Debug, Clone)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
/// A handle to the YouTube Music API, wrapping a reqwest::Client.
/// Generic over AuthToken, as different AuthTokens may allow different queries to be executed.
pub struct YtMusic<A: AuthToken> {
    // TODO: add language
    // TODO: add location
    client: Client,
    token: A,
}

impl YtMusic<BrowserToken> {
    /// Create a new API handle using a BrowserToken.
    pub fn from_browser_token(token: BrowserToken) -> YtMusic<BrowserToken> {
        let client = Client::new();
        YtMusic { client, token }
    }
    /// Create a new API handle using a real browser authentication cookie saved to a file on disk.
    pub async fn from_cookie_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a String.
    pub async fn from_cookie<S: AsRef<str>>(cookie: S) -> Result<Self> {
        let client = Client::new();
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(Self { client, token })
    }
}
impl YtMusic<OAuthToken> {
    /// Create a new API handle using an OAuthToken.
    pub fn from_oauth_token(token: OAuthToken) -> YtMusic<OAuthToken> {
        let client = Client::new();
        YtMusic { client, token }
    }
    /// Refresh the internal oauth token, and return a clone of it (for user to store locally, e.g).
    pub async fn refresh_token(&mut self) -> Result<OAuthToken> {
        let refreshed_token = self.token.refresh(&self.client).await?;
        self.token = refreshed_token.clone();
        Ok(refreshed_token)
    }
}
impl<A: AuthToken> YtMusic<A> {
    async fn raw_query<Q: Query>(&self, query: Q) -> Result<RawResult<Q, A>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.token.raw_query(&self.client, query).await
    }
    /// Return the raw JSON returned by YouTube music for Query Q.
    pub async fn json_query<Q: Query>(&self, query: Q) -> Result<String> {
        // TODO: Remove allocation
        let json = self.raw_query(query).await?.process()?.clone_json();
        Ok(json)
    }
    /// API Search Query that returns results for each category if available.
    pub async fn search<'a, Q: Into<SearchQuery<'a, BasicSearch>>>(
        &self,
        query: Q,
    ) -> Result<SearchResults> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Artists only.
    pub async fn search_artists<'a, Q: Into<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultArtist>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Albums only.
    pub async fn search_albums<'a, Q: Into<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultAlbum>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Songs only.
    pub async fn search_songs<'a, Q: Into<SearchQuery<'a, FilteredSearch<SongsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultSong>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Playlists only.
    pub async fn search_playlists<'a, Q: Into<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPlaylist>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Community Playlists only.
    pub async fn search_community_playlists<
        'a,
        Q: Into<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>,
    >(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPlaylist>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Featured Playlists only.
    pub async fn search_featured_playlists<
        'a,
        Q: Into<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>,
    >(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultFeaturedPlaylist>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Episodes only.
    pub async fn search_episodes<'a, Q: Into<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultEpisode>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Podcasts only.
    pub async fn search_podcasts<'a, Q: Into<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPodcast>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Videos only.
    pub async fn search_videos<'a, Q: Into<SearchQuery<'a, FilteredSearch<VideosFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultVideo>> {
        let query = query.into();
        self.raw_query(query).await?.process()?.parse()
    }
    /// API Search Query for Profiles only.
    pub async fn search_profiles<'a, Q: Into<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultProfile>> {
        let query = query.into();
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
    // TODO: Implement for other cases of query.
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
// TODO: Keep session alive after calling these methods.
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
