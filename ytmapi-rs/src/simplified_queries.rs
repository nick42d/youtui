//! This module contains the implementation for more convenient ways to call the
//! API, in many cases without the need of building Query structs.
//! # Optional
//! To enable this module, feature `simplified-queries` must be enabled (enabled
//! by default)
use crate::auth::AuthToken;
use crate::common::{
    browsing::Lyrics,
    library::{LibraryArtist, Playlist},
    watch::WatchPlaylist,
    FeedbackTokenRemoveFromHistory, PlaylistID, SearchSuggestion, UploadAlbumID, UploadArtistID,
};
use crate::common::{AlbumID, BrowseParams};
use crate::parse::{
    AddPlaylistItem, AlbumParams, ApiSuccess, ArtistParams, GetArtistAlbums,
    GetLibraryArtistSubscription, GetPlaylist, LikeStatus, SearchResultAlbum, SearchResultArtist,
    SearchResultEpisode, SearchResultFeaturedPlaylist, SearchResultPlaylist, SearchResultPodcast,
    SearchResultProfile, SearchResultSong, SearchResultVideo, SearchResults, TableListItem,
    TableListSong,
};
pub use crate::process::RawResult;
use crate::query::{
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
    GetLibrarySongsQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
    GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
    GetPlaylistQuery, GetSearchSuggestionsQuery, Query, RemoveHistoryItemsQuery,
    RemovePlaylistItemsQuery, SearchQuery,
};
use crate::{common::UploadEntityID, query::DeleteUploadEntityQuery};
use crate::{Album, ChannelID, Result, VideoID, YtMusic};

impl<A: AuthToken> YtMusic<A> {
    /// API Search Query that returns results for each category if available.
    /// # Usage
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search("Beatles").await
    /// # };
    pub async fn search<'a, Q: Into<SearchQuery<'a, BasicSearch>>>(
        &self,
        query: Q,
    ) -> Result<SearchResults> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Artists only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_artists("Beatles").await
    /// # };
    pub async fn search_artists<'a, Q: Into<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultArtist>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Albums only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_albums("Beatles").await
    /// # };
    pub async fn search_albums<'a, Q: Into<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultAlbum>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Songs only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_songs("Beatles").await
    /// # };
    pub async fn search_songs<'a, Q: Into<SearchQuery<'a, FilteredSearch<SongsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultSong>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Playlists only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_playlists("Beatles").await
    /// # };
    pub async fn search_playlists<'a, Q: Into<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPlaylist>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Community Playlists only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_community_playlists("Beatles").await
    /// # };
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
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_featured_playlists("Beatles").await
    /// # };
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
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_episodes("Beatles").await
    /// # };
    pub async fn search_episodes<'a, Q: Into<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultEpisode>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Podcasts only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_podcasts("Beatles").await
    /// # };
    pub async fn search_podcasts<'a, Q: Into<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultPodcast>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Videos only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_videos("Beatles").await
    /// # };
    pub async fn search_videos<'a, Q: Into<SearchQuery<'a, FilteredSearch<VideosFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultVideo>> {
        let query = query.into();
        query.call(self).await
    }
    /// API Search Query for Profiles only.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.search_profiles("Beatles").await
    /// # };
    pub async fn search_profiles<'a, Q: Into<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>>(
        &self,
        query: Q,
    ) -> Result<Vec<SearchResultProfile>> {
        let query = query.into();
        query.call(self).await
    }
    /// Gets information about an artist and their top releases.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_artists("Beatles").await.unwrap();
    /// yt.get_artist((&results[0].browse_id).into()).await
    /// # };
    pub async fn get_artist(&self, channel_id: ChannelID<'_>) -> Result<ArtistParams> {
        let query = GetArtistQuery::new(channel_id);
        query.call(self).await
    }
    /// Gets a full list albums for an artist.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_artists("Beatles").await.unwrap();
    /// let artist_top_albums = yt.get_artist((&results[0].browse_id).into()).await.unwrap().top_releases.albums.unwrap();
    /// yt.get_artist_albums(
    ///     artist_top_albums.browse_id.unwrap(),
    ///     artist_top_albums.params.unwrap(),
    /// ).await
    /// # };
    pub async fn get_artist_albums(
        &self,
        channel_id: ChannelID<'_>,
        browse_params: BrowseParams<'_>,
    ) -> Result<Vec<Album>> {
        let query = GetArtistAlbumsQuery::new(channel_id, browse_params);
        query.call(self).await
    }
    /// Gets information about an album and its tracks.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_albums("Dark Side Of The Moon").await.unwrap();
    /// yt.get_album(&results[0].album_id).await
    /// # };
    // NOTE: Approach to use Into<AlbumID> means we can take AlbumID or &AlbumID.
    // Should implement this for more queries.
    pub async fn get_album<'a, T: Into<AlbumID<'a>>>(&self, album_id: T) -> Result<AlbumParams> {
        let query = GetAlbumQuery::new(album_id);
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
        let query = GetHistoryQuery;
        query.call(self).await
    }
    pub async fn remove_history_items<'a>(
        &self,
        feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        let query = RemoveHistoryItemsQuery::new(feedback_tokens);
        query.call(self).await
    }
    pub async fn edit_song_library_status<'a>(
        &self,
        query: EditSongLibraryStatusQuery<'a>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        query.call(self).await
    }
    pub async fn rate_song(&self, video_id: VideoID<'_>, rating: LikeStatus) -> Result<ApiSuccess> {
        let query = RateSongQuery::new(video_id, rating);
        query.call(self).await
    }
    pub async fn rate_playlist(
        &self,
        playlist_id: PlaylistID<'_>,
        rating: LikeStatus,
    ) -> Result<ApiSuccess> {
        let query = RatePlaylistQuery::new(playlist_id, rating);
        query.call(self).await
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
    pub async fn get_library_upload_songs(
        &self,
    ) -> Result<<GetLibraryUploadSongsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadSongsQuery::default();
        query.call(self).await
    }
    pub async fn get_library_upload_artists(
        &self,
    ) -> Result<<GetLibraryUploadArtistsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadArtistsQuery::default();
        query.call(self).await
    }
    pub async fn get_library_upload_albums(
        &self,
    ) -> Result<<GetLibraryUploadAlbumsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadAlbumsQuery::default();
        query.call(self).await
    }
    pub async fn get_library_upload_album(
        &self,
        upload_album_id: UploadAlbumID<'_>,
    ) -> Result<<GetLibraryUploadAlbumQuery as Query<A>>::Output> {
        let query = GetLibraryUploadAlbumQuery::new(upload_album_id);
        query.call(self).await
    }
    pub async fn get_library_upload_artist(
        &self,
        upload_artist_id: UploadArtistID<'_>,
    ) -> Result<<GetLibraryUploadArtistQuery as Query<A>>::Output> {
        let query = GetLibraryUploadArtistQuery::new(upload_artist_id);
        query.call(self).await
    }
    pub async fn delete_upload_entity(
        &self,
        upload_entity_id: UploadEntityID<'_>,
    ) -> Result<<DeleteUploadEntityQuery as Query<A>>::Output> {
        let query = DeleteUploadEntityQuery::new(upload_entity_id);
        query.call(self).await
    }
}
