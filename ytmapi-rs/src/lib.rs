//! # ytmapi_rs
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
//!     println!("Go to {url}, fhe login flow, and press enter when done");
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
//! ## Optional Features
//! ### TLS
//! NOTE: To use an alternative TLS, you will need to specify `default-features
//! = false`. As reqwest preferentially uses default-tls when multiple TLS
//! features are enabled. See reqwest docs for more information.
//! <https://docs.rs/reqwest/latest/reqwest/tls/index.html>
//! - **default-tls** *(enabled by default)*: Utilises the default TLS from
//!   reqwest - at the time of writing is native-tls.
//! - **native-tls**: This feature forces use of the the native-tls crate,
//!   reliant on vendors tls.
//! - **rustls-tls**: This feature forces use of the rustls crate, written in
//!   rust.
use auth::{
    browser::BrowserToken, oauth::OAuthDeviceCode, AuthToken, OAuthToken, OAuthTokenGenerator,
};
use common::{
    browsing::Lyrics,
    library::{LibraryArtist, Playlist},
    watch::WatchPlaylist,
    FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromHistory, PlaylistID, SearchSuggestion,
};
pub use common::{Album, BrowseID, ChannelID, Thumbnail, VideoID};
pub use error::{Error, Result};
use parse::{
    AddPlaylistItem, AlbumParams, ApiSuccess, ArtistParams, GetLibraryArtistSubscription,
    GetPlaylist, LikeStatus, ParseFrom, ProcessedResult, SearchResultAlbum, SearchResultArtist,
    SearchResultEpisode, SearchResultFeaturedPlaylist, SearchResultPlaylist, SearchResultPodcast,
    SearchResultProfile, SearchResultSong, SearchResultVideo, SearchResults, TableListItem,
    TableListSong,
};
pub use process::RawResult;
use query::{
    filteredsearch::{
        AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter,
        FeaturedPlaylistsFilter, FilteredSearch, PlaylistsFilter, PodcastsFilter, ProfilesFilter,
        SongsFilter, VideosFilter,
    },
    lyrics::GetLyricsQuery,
    rate::{RatePlaylistQuery, RateSongQuery},
    watch::GetWatchPlaylistQuery,
    AddPlaylistItemsQuery, AddVideosToPlaylist, BasicSearch, CreatePlaylistQuery,
    CreatePlaylistType, DeletePlaylistQuery, EditPlaylistQuery, EditSongLibraryStatusQuery,
    GetAlbumQuery, GetArtistAlbumsQuery, GetArtistQuery, GetHistoryQuery, GetLibraryAlbumsQuery,
    GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery, GetLibraryPlaylistsQuery,
    GetLibrarySongsQuery, GetPlaylistQuery, GetSearchSuggestionsQuery, Query,
    RemoveHistoryItemsQuery, RemovePlaylistItemsQuery, SearchQuery,
};
use reqwest::Client;
use std::path::Path;

// TODO: Confirm if auth should be pub
pub mod auth;
#[macro_use]
mod utils;
mod locales {}
mod nav_consts;
// Consider if pub is correct for this
pub mod common;
mod crawler;
pub mod error;
pub mod parse;
mod process;
pub mod query;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
// XXX: Consider wrapping auth in reference counting for cheap cloning.
/// A handle to the YouTube Music API, wrapping a reqwest::Client.
/// Generic over AuthToken, as different AuthTokens may allow different queries
/// to be executed.
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
    /// Create a new API handle using a real browser authentication cookie saved
    /// to a file on disk.
    pub async fn from_cookie_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let client = Client::new();
        let token = BrowserToken::from_cookie_file(path, &client).await?;
        Ok(Self { client, token })
    }
    /// Create a new API handle using a real browser authentication cookie in a
    /// String.
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
    /// Refresh the internal oauth token, and return a clone of it (for user to
    /// store locally, e.g).
    pub async fn refresh_token(&mut self) -> Result<OAuthToken> {
        let refreshed_token = self.token.refresh(&self.client).await?;
        self.token = refreshed_token.clone();
        Ok(refreshed_token)
    }
}
impl<A: AuthToken> YtMusic<A> {
    //TODO: Usage examples
    /// Return a raw result from YouTube music for query Q that requires further
    /// processing.
    pub async fn raw_query<Q: Query>(&self, query: Q) -> Result<RawResult<Q, A>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.token.raw_query(&self.client, query).await
    }
    /// Return a result from YouTube music that has had errors removed and been
    /// processed into parsable JSON.
    pub async fn processed_query<Q: Query>(&self, query: Q) -> Result<ProcessedResult<Q>> {
        // TODO: Check for a response the reflects an expired Headers token
        self.token.raw_query(&self.client, query).await?.process()
    }
    /// Return the raw JSON returned by YouTube music for Query Q.
    pub async fn json_query<Q: Query>(&self, query: Q) -> Result<String> {
        // TODO: Remove allocation
        let json = self.raw_query(query).await?.process()?.clone_json();
        Ok(json)
    }
    pub async fn query<Q: Query>(&self, query: Q) -> Result<Q::Output> {
        query.call(self).await
    }
    /// Process a string of JSON as if it had been directly received from the
    /// api for a query. Note that this is generic across AuthToken.
    /// NOTE: Potentially can be removed from impl
    pub fn process_json<Q: Query>(json: String, query: Q) -> Result<Q::Output> {
        Q::Output::parse_from(RawResult::<Q, A>::from_raw(json, query).process()?)
    }
    /// API Search Query that returns results for each category if available.
    pub async fn search<'a, Q: Into<SearchQuery<'a, BasicSearch>>>(
        &self,
        query: Q,
    ) -> Result<SearchResults> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Artists only.
    pub async fn search_artists<'a, Q: Into<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultArtist>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Albums only.
    pub async fn search_albums<'a, Q: Into<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultAlbum>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Songs only.
    pub async fn search_songs<'a, Q: Into<SearchQuery<'a, FilteredSearch<SongsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultSong>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Playlists only.
    pub async fn search_playlists<'a, Q: Into<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPlaylist>> {
        let query = query.into();
        query.call(self).await
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
        query.call(self).await
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
        query.call(self).await
    }
    /// API Search Query for Episodes only.
    pub async fn search_episodes<'a, Q: Into<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultEpisode>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Podcasts only.
    pub async fn search_podcasts<'a, Q: Into<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPodcast>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Videos only.
    pub async fn search_videos<'a, Q: Into<SearchQuery<'a, FilteredSearch<VideosFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultVideo>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Profiles only.
    pub async fn search_profiles<'a, Q: Into<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultProfile>> {
        let query = query.into();
        query.call(self).await
    }
    pub async fn get_artist(&self, query: GetArtistQuery<'_>) -> Result<ArtistParams> {
        query.call(self).await
    }
    pub async fn get_artist_albums(&self, query: GetArtistAlbumsQuery<'_>) -> Result<Vec<Album>> {
        query.call(self).await
    }
    pub async fn get_album(&self, query: GetAlbumQuery<'_>) -> Result<AlbumParams> {
        query.call(self).await
    }
    pub async fn get_lyrics(&self, query: GetLyricsQuery<'_>) -> Result<Lyrics> {
        query.call(self).await
    }
    // TODO: Implement for other cases of query.
    pub async fn get_watch_playlist<'a, S: Into<GetWatchPlaylistQuery<VideoID<'a>>>>(
        &self,
        query: S,
    ) -> Result<WatchPlaylist> {
        let query = query.into();
        query.call(self).await
    }
    // TODO: Implement for other cases of query.
    pub async fn get_playlist<'a, S: Into<GetPlaylistQuery<'a>>>(
        &self,
        query: S,
    ) -> Result<GetPlaylist> {
        let query = query.into();
        query.call(self).await
    }
    pub async fn get_search_suggestions<'a, S: Into<GetSearchSuggestionsQuery<'a>>>(
        &self,
        query: S,
    ) -> Result<Vec<SearchSuggestion>> {
        let query = query.into();
        query.call(self).await
    }
    pub async fn get_library_playlists(&self) -> Result<Vec<Playlist>> {
        // TODO: investigate why returning empty array
        let query = GetLibraryPlaylistsQuery;
        query.call(self).await
    }
    pub async fn get_library_artists(
        // TODO: investigate why returning empty array
        // TODO: Better constructor for query
        &self,
        query: GetLibraryArtistsQuery,
    ) -> Result<Vec<LibraryArtist>> {
        query.call(self).await
    }
    pub async fn get_library_songs(
        &self,
        query: GetLibrarySongsQuery,
    ) -> Result<Vec<TableListSong>> {
        query.call(self).await
    }
    pub async fn get_library_albums(
        &self,
        query: GetLibraryAlbumsQuery,
    ) -> Result<Vec<SearchResultAlbum>> {
        query.call(self).await
    }
    pub async fn get_library_artist_subscriptions(
        &self,
        query: GetLibraryArtistSubscriptionsQuery,
    ) -> Result<Vec<GetLibraryArtistSubscription>> {
        query.call(self).await
    }
    pub async fn get_history(&self) -> Result<Vec<TableListItem>> {
        self.query(GetHistoryQuery).await
    }
    pub async fn remove_history_items<'a>(
        &self,
        feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        let query = RemoveHistoryItemsQuery::new(feedback_tokens);
        self.query(query).await
    }
    pub async fn edit_song_library_status<'a>(
        &self,
        feedback_tokens: Vec<FeedbackTokenAddToLibrary<'a>>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        let query = EditSongLibraryStatusQuery::new(feedback_tokens);
        self.query(query).await
    }
    pub async fn rate_song(&self, video_id: VideoID<'_>, rating: LikeStatus) -> Result<ApiSuccess> {
        let query = RateSongQuery::new(video_id, rating);
        self.query(query).await
    }
    pub async fn rate_playlist(
        &self,
        playlist_id: PlaylistID<'_>,
        rating: LikeStatus,
    ) -> Result<ApiSuccess> {
        let query = RatePlaylistQuery::new(playlist_id, rating);
        self.query(query).await
    }
    pub async fn delete_playlist<'a, Q: Into<DeletePlaylistQuery<'a>>>(
        &self,
        query: Q,
    ) -> Result<ApiSuccess> {
        query.into().call(self).await
    }
    pub async fn create_playlist<'a, Q: Into<CreatePlaylistQuery<'a, C>>, C: CreatePlaylistType>(
        &self,
        query: Q,
    ) -> Result<PlaylistID<'static>> {
        query.into().call(self).await
    }
    pub async fn remove_playlist_items<'a, Q: Into<RemovePlaylistItemsQuery<'a>>>(
        &self,
        query: Q,
    ) -> Result<ApiSuccess> {
        query.into().call(self).await
    }
    pub async fn add_playlist_video_items<
        'a,
        Q: Into<AddPlaylistItemsQuery<'a, AddVideosToPlaylist<'a>>>,
    >(
        &self,
        query: Q,
    ) -> Result<Vec<AddPlaylistItem>> {
        query.into().call(self).await
    }
    pub async fn edit_playlist<'a, Q: Into<EditPlaylistQuery<'a>>>(
        &self,
        query: Q,
    ) -> Result<ApiSuccess> {
        query.into().call(self).await
    }
}
// TODO: Keep session alive after calling these methods.
/// Generates a tuple containing fresh OAuthDeviceCode and corresponding url for
/// you to authenticate yourself at. (OAuthDeviceCode, URL)
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
