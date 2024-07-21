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
use crate::common::{AlbumID, BrowseParams, LyricsID, SetVideoID};
use crate::parse::{
    AddPlaylistItem, AlbumParams, ApiSuccess, ArtistParams, GetArtistAlbums,
    GetLibraryArtistSubscription, GetPlaylist, LikeStatus, SearchResultAlbum, SearchResultArtist,
    SearchResultEpisode, SearchResultFeaturedPlaylist, SearchResultPlaylist, SearchResultPodcast,
    SearchResultProfile, SearchResultSong, SearchResultVideo, SearchResults, TableListItem,
    TableListSong,
};
use crate::process::RawResult;
use crate::query::DuplicateHandlingMode;
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
    /// yt.get_artist(&results[0].browse_id).await
    /// # };
    pub async fn get_artist<'a, T: Into<ChannelID<'a>>>(
        &self,
        channel_id: T,
    ) -> Result<ArtistParams> {
        let query = GetArtistQuery::new(channel_id.into());
        query.call(self).await
    }
    /// Gets a full list albums for an artist.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_artists("Beatles").await.unwrap();
    /// let artist_top_albums = yt.get_artist(&results[0].browse_id).await.unwrap().top_releases.albums.unwrap();
    /// yt.get_artist_albums(
    ///     artist_top_albums.browse_id.unwrap(),
    ///     artist_top_albums.params.unwrap(),
    /// ).await
    /// # };
    pub async fn get_artist_albums<'a, T: Into<ChannelID<'a>>, U: Into<BrowseParams<'a>>>(
        &self,
        channel_id: T,
        browse_params: U,
    ) -> Result<Vec<Album>> {
        let query = GetArtistAlbumsQuery::new(channel_id.into(), browse_params.into());
        query.call(self).await
    }
    /// Gets information about an album and its tracks.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_albums("Dark Side Of The Moon").await.unwrap();
    /// yt.get_album(&results[0].album_id).await
    /// # };
    pub async fn get_album<'a, T: Into<AlbumID<'a>>>(&self, album_id: T) -> Result<AlbumParams> {
        let query = GetAlbumQuery::new(album_id);
        query.call(self).await
    }
    /// Gets the information that's available when playing a song or playlist;
    /// upcoming tracks and lyrics.
    /// # Partially implemented
    /// Tracks are not implemented - empty vector always returned.
    /// See [`GetWatchPlaylistQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run
    /// a GetWatchPlaylistQuery.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetWatchPlaylistQuery]: crate::query::watch::GetWatchPlaylistQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// yt.get_watch_playlist_from_video_id(&results[0].video_id).await
    /// # };
    // NOTE: Could be generic across PlaylistID or VideoID using
    // Into<GetWatchPlaylistQuery>
    pub async fn get_watch_playlist_from_video_id<'a, S: Into<VideoID<'a>>>(
        &self,
        video_id: S,
    ) -> Result<WatchPlaylist> {
        let query = GetWatchPlaylistQuery::new_from_video_id(video_id.into());
        query.call(self).await
    }
    /// Gets song lyrics and the source.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// let watch_playlist = yt.get_watch_playlist_from_video_id(&results[0].video_id).await.unwrap();
    /// yt.get_lyrics(watch_playlist.lyrics_id).await
    /// # };
    pub async fn get_lyrics<'a, T: Into<LyricsID<'a>>>(&self, lyrics_id: T) -> Result<Lyrics> {
        let query = GetLyricsQuery::new(lyrics_id.into());
        query.call(self).await
    }
    /// Gets information about a playlist and its tracks.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_featured_playlists("Heavy metal").await.unwrap();
    /// yt.get_playlist(&results[0].playlist_id).await
    /// # };
    pub async fn get_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
    ) -> Result<GetPlaylist> {
        let query = GetPlaylistQuery::new(playlist_id.into());
        query.call(self).await
    }
    /// Gets search suggestions
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_search_suggestions("The Beat").await;
    /// # };
    pub async fn get_search_suggestions<'a, S: Into<GetSearchSuggestionsQuery<'a>>>(
        &self,
        query: S,
    ) -> Result<Vec<SearchSuggestion>> {
        let query = query.into();
        query.call(self).await
    }
    /// Gets a list of all playlists in your Library.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_library_playlists().await;
    /// # };
    pub async fn get_library_playlists(&self) -> Result<Vec<Playlist>> {
        let query = GetLibraryPlaylistsQuery;
        query.call(self).await
    }
    /// Gets a list of all artists in your Library.
    /// # Additional functionality
    /// See [`GetLibraryArtistsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryArtistsQuery]: crate::query::GetLibraryArtistsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_artists().await;
    /// # };
    pub async fn get_library_artists(&self) -> Result<Vec<LibraryArtist>> {
        let query = GetLibraryArtistsQuery::default();
        query.call(self).await
    }
    /// Gets a list of all songs in your Library.
    /// # Additional functionality
    /// See [`GetLibrarySongsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibrarySongsQuery]: crate::query::GetLibrarySongsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_songs().await;
    /// # };
    pub async fn get_library_songs(&self) -> Result<Vec<TableListSong>> {
        let query = GetLibrarySongsQuery::default();
        query.call(self).await
    }
    /// Gets a list of all albums in your Library.
    /// # Additional functionality
    /// See [`GetLibraryAlbumsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryAlbumsQuery]: crate::query::GetLibraryAlbumsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_albums().await;
    /// # };
    pub async fn get_library_albums(&self) -> Result<Vec<SearchResultAlbum>> {
        let query = GetLibraryAlbumsQuery::default();
        query.call(self).await
    }
    /// Gets a list of all artist subscriptions in your Library.
    /// # Additional functionality
    /// See [`GetLibraryArtistSubscriptionsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryArtistSubscriptionsQuery]: crate::query::GetLibraryArtistSubscriptionsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_artist_subscriptions().await;
    /// # };
    pub async fn get_library_artist_subscriptions(
        &self,
    ) -> Result<Vec<GetLibraryArtistSubscription>> {
        let query = GetLibraryArtistSubscriptionsQuery::default();
        query.call(self).await
    }
    /// Gets your recently played history.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_history().await;
    /// # };
    pub async fn get_history(&self) -> Result<Vec<TableListItem>> {
        let query = GetHistoryQuery;
        query.call(self).await
    }
    /// Removes a list of items from your recently played history.
    /// # Note
    /// The feedback tokens required to call this query are currently not
    /// generated by `get_history()`.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let history = yt.get_history().await.unwrap();
    /// todo!("FeedbackTokenRemoveFromHistory are not able to be accessed from history currently")
    /// # };
    pub async fn remove_history_items<'a>(
        &self,
        feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        let query = RemoveHistoryItemsQuery::new(feedback_tokens);
        query.call(self).await
    }
    // TODO: Docs / alternative constructors.
    pub async fn edit_song_library_status<'a>(
        &self,
        query: EditSongLibraryStatusQuery<'a>,
    ) -> Result<Vec<Result<ApiSuccess>>> {
        query.call(self).await
    }
    /// Sets the like status for a song.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// yt.rate_song(&results[0].video_id, ytmapi_rs::parse::LikeStatus::Liked).await
    /// # };
    pub async fn rate_song<'a, T: Into<VideoID<'a>>>(
        &self,
        video_id: T,
        rating: LikeStatus,
    ) -> Result<ApiSuccess> {
        let query = RateSongQuery::new(video_id.into(), rating);
        query.call(self).await
    }
    /// Sets the like status for a playlist.
    /// A 'Liked' playlist will be added to your library, an 'Indifferent' will
    /// be removed, and a 'Disliked' will reduce the chance of it appearing in
    /// your recommendations.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_featured_playlists("Heavy metal")
    ///     .await
    ///     .unwrap();
    /// yt.rate_playlist(
    ///     &results[0].playlist_id,
    ///     ytmapi_rs::parse::LikeStatus::Liked,
    /// ).await
    /// # };
    pub async fn rate_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        rating: LikeStatus,
    ) -> Result<ApiSuccess> {
        let query = RatePlaylistQuery::new(playlist_id.into(), rating);
        query.call(self).await
    }
    /// Deletes a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_playlists().await.unwrap();
    /// yt.delete_playlist(&results[0].playlist_id).await
    /// # };
    pub async fn delete_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
    ) -> Result<ApiSuccess> {
        let query = DeletePlaylistQuery::new(playlist_id.into());
        query.call(self).await
    }
    /// Creates a new playlist.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let playlists = yt.search_featured_playlists("Heavy metal")
    ///     .await
    ///     .unwrap();
    /// let query = ytmapi_rs::query::CreatePlaylistQuery::new(
    ///     "My heavy metal playlist",
    ///     None,
    ///     ytmapi_rs::query::PrivacyStatus::Public,
    /// )
    ///     .with_source(&playlists[0].playlist_id);
    /// yt.create_playlist(query).await
    /// # };
    pub async fn create_playlist<'a, T: CreatePlaylistType>(
        &self,
        query: CreatePlaylistQuery<'a, T>,
    ) -> Result<PlaylistID<'static>> {
        query.call(self).await
    }
    /// Adds video items to a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::common::library::Playlist { playlist_id, .. } =
    ///     yt.get_library_playlists().await.unwrap().pop().unwrap();
    /// let songs = yt.search_songs("Master of puppets").await.unwrap();
    /// yt.add_video_items_to_playlist(
    ///     playlist_id,
    ///     songs.iter().map(|s| (&s.video_id).into()).collect()
    /// ).await
    /// # };
    pub async fn add_video_items_to_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        video_ids: Vec<VideoID<'a>>,
    ) -> Result<Vec<AddPlaylistItem>> {
        let query = AddPlaylistItemsQuery::new_from_videos(
            playlist_id.into(),
            video_ids,
            DuplicateHandlingMode::default(),
        );
        query.call(self).await
    }
    /// Appends another playlist to a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::common::library::Playlist { playlist_id, .. } =
    ///     yt.get_library_playlists().await.unwrap().pop().unwrap();
    /// let source_playlist = yt.search_featured_playlists("Heavy metal")
    ///     .await
    ///     .unwrap();
    /// yt.add_playlist_to_playlist(
    ///     playlist_id,
    ///     &source_playlist[0].playlist_id
    /// ).await
    /// # };
    pub async fn add_playlist_to_playlist<'a, T: Into<PlaylistID<'a>>, U: Into<PlaylistID<'a>>>(
        &self,
        destination_playlist: T,
        source_playlist: U,
    ) -> Result<Vec<AddPlaylistItem>> {
        let query = AddPlaylistItemsQuery::new_from_playlist(
            destination_playlist.into(),
            source_playlist.into(),
        );
        query.call(self).await
    }
    /// Removes items from a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::common::library::Playlist { playlist_id, .. } =
    ///     yt.get_library_playlists().await.unwrap().pop().unwrap();
    /// let source_playlist = yt.search_featured_playlists("Heavy metal")
    ///     .await
    ///     .unwrap();
    /// let outcome = yt.add_playlist_to_playlist(
    ///     &playlist_id,
    ///     &source_playlist[0].playlist_id
    /// ).await.unwrap();
    /// yt.remove_playlist_items(
    ///     playlist_id,
    ///     outcome.iter().map(|o| (&o.set_video_id).into()).collect(),
    /// ).await
    /// # };
    pub async fn remove_playlist_items<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        video_items: Vec<SetVideoID<'a>>,
    ) -> Result<ApiSuccess> {
        let query = RemovePlaylistItemsQuery::new(playlist_id.into(), video_items);
        query.call(self).await
    }
    /// Makes changes to a playlist.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let playlists = yt.get_library_playlists()
    ///     .await
    ///     .unwrap();
    /// let query = ytmapi_rs::query::EditPlaylistQuery::new_title(
    ///     &playlists[0].playlist_id,
    ///     "Better playlist title",
    /// )
    ///     .with_new_description("Edited description");
    /// yt.edit_playlist(query).await
    /// # };
    pub async fn edit_playlist(&self, query: EditPlaylistQuery<'_>) -> Result<ApiSuccess> {
        query.call(self).await
    }
    /// Gets a list of all uploaded songs in your Library.
    /// # Additional functionality
    /// See [`GetLibraryUploadSongsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryUploadSongsQuery]: crate::query::GetLibraryUploadSongsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_library_upload_songs().await
    /// # };
    pub async fn get_library_upload_songs(
        &self,
    ) -> Result<<GetLibraryUploadSongsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadSongsQuery::default();
        query.call(self).await
    }
    /// Gets a list of all uploaded artists in your Library.
    /// # Additional functionality
    /// See [`GetLibraryUploadArtistsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryUploadArtistsQuery]: crate::query::GetLibraryUploadArtistsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_library_upload_artists().await
    /// # };
    pub async fn get_library_upload_artists(
        &self,
    ) -> Result<<GetLibraryUploadArtistsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadArtistsQuery::default();
        query.call(self).await
    }
    /// Gets a list of all uploaded albums in your Library.
    /// # Additional functionality
    /// See [`GetLibraryUploadAlbumsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryUploadAlbumsQuery]: crate::query::GetLibraryUploadAlbumsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_library_upload_albums().await
    /// # };
    pub async fn get_library_upload_albums(
        &self,
    ) -> Result<<GetLibraryUploadAlbumsQuery as Query<A>>::Output> {
        let query = GetLibraryUploadAlbumsQuery::default();
        query.call(self).await
    }
    /// Gets information and tracks for an uploaded album in your Library.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let albums = yt.get_library_upload_albums().await.unwrap();
    /// yt.get_library_upload_album(&albums[0].album_id).await
    /// # };
    pub async fn get_library_upload_album<'a, T: Into<UploadAlbumID<'a>>>(
        &self,
        upload_album_id: T,
    ) -> Result<<GetLibraryUploadAlbumQuery as Query<A>>::Output> {
        let query = GetLibraryUploadAlbumQuery::new(upload_album_id.into());
        query.call(self).await
    }
    /// Gets all tracks for an uploaded artist in your Library.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let artists = yt.get_library_upload_artists().await.unwrap();
    /// yt.get_library_upload_artist(&artists[0].artist_id).await
    /// # };
    pub async fn get_library_upload_artist<'a, T: Into<UploadArtistID<'a>>>(
        &self,
        upload_artist_id: T,
    ) -> Result<<GetLibraryUploadArtistQuery as Query<A>>::Output> {
        let query = GetLibraryUploadArtistQuery::new(upload_artist_id.into());
        query.call(self).await
    }
    /// Deletes an upload entity from your library - this is either a song or an
    /// album.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let albums = yt.get_library_upload_albums().await.unwrap();
    /// yt.delete_upload_entity(&albums[0].entity_id).await
    /// # };
    pub async fn delete_upload_entity<'a, T: Into<UploadEntityID<'a>>>(
        &self,
        upload_entity_id: T,
    ) -> Result<<DeleteUploadEntityQuery as Query<A>>::Output> {
        let query = DeleteUploadEntityQuery::new(upload_entity_id.into());
        query.call(self).await
    }
}
