//! This module contains the implementation for more convenient ways to call the
//! API, in many cases without the need of building Query structs.
//! This module contains purely additional implementations for YtMusic. To see
//! the documentation, refer to the [`YtMusic`] documentation itself.
//! # Optional
//! To enable this module, feature `simplified-queries` must be enabled (enabled
//! by default)
use crate::auth::{AuthToken, LoggedIn};
use crate::common::{
    AlbumID, ApiOutcome, ArtistChannelID, BrowseParams, EpisodeID, FeedbackTokenRemoveFromHistory,
    LikeStatus, LyricsID, MoodCategoryParams, PlaylistID, PodcastChannelID, PodcastChannelParams,
    PodcastID, SearchSuggestion, SetVideoID, SongTrackingUrl, TasteToken, UploadAlbumID,
    UploadArtistID, UploadEntityID, VideoID,
};
use crate::parse::{
    AddPlaylistItem, ArtistParams, GetAlbum, GetArtistAlbumsAlbum, GetPlaylistDetails,
    HistoryPeriod, LibraryArtist, LibraryArtistSubscription, LibraryPlaylist, Lyrics, PlaylistItem,
    SearchResultAlbum, SearchResultArtist, SearchResultEpisode, SearchResultFeaturedPlaylist,
    SearchResultPlaylist, SearchResultPodcast, SearchResultProfile, SearchResultSong,
    SearchResultVideo, SearchResults, WatchPlaylistTrack,
};
use crate::query::playlist::{CreatePlaylistType, DuplicateHandlingMode, GetPlaylistDetailsQuery};
use crate::query::rate::{RatePlaylistQuery, RateSongQuery};
use crate::query::search::filteredsearch::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    FilteredSearch, PlaylistsFilter, PodcastsFilter, ProfilesFilter, SongsFilter, VideosFilter,
};
use crate::query::search::BasicSearch;
use crate::query::song::{GetLyricsQuery, GetSongTrackingUrlQuery};
use crate::query::{
    AddHistoryItemQuery, AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery,
    DeleteUploadEntityQuery, EditPlaylistQuery, EditSongLibraryStatusQuery, GetAlbumQuery,
    GetArtistAlbumsQuery, GetArtistQuery, GetChannelEpisodesQuery, GetChannelQuery,
    GetEpisodeQuery, GetHistoryQuery, GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery,
    GetLibraryArtistsQuery, GetLibraryChannelsQuery, GetLibraryPlaylistsQuery,
    GetLibraryPodcastsQuery, GetLibrarySongsQuery, GetLibraryUploadAlbumQuery,
    GetLibraryUploadAlbumsQuery, GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery,
    GetLibraryUploadSongsQuery, GetLyricsIDQuery, GetMoodCategoriesQuery, GetMoodPlaylistsQuery,
    GetNewEpisodesQuery, GetPlaylistTracksQuery, GetPodcastQuery, GetSearchSuggestionsQuery,
    GetTasteProfileQuery, GetWatchPlaylistQuery, Query, RemoveHistoryItemsQuery,
    RemovePlaylistItemsQuery, SearchQuery, SetTasteProfileQuery, SubscribeArtistsQuery,
    UnsubscribeArtistsQuery,
};
use crate::{Result, YtMusic};

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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
    }
    /// Gets information about an artist and their top releases.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_artists("Beatles").await.unwrap();
    /// yt.get_artist(&results[0].browse_id).await
    /// # };
    pub async fn get_artist<'a>(
        &self,
        query: impl Into<GetArtistQuery<'a>>,
    ) -> Result<ArtistParams> {
        self.query(query.into()).await
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
    pub async fn get_artist_albums<'a, T: Into<ArtistChannelID<'a>>, U: Into<BrowseParams<'a>>>(
        &self,
        channel_id: T,
        browse_params: U,
    ) -> Result<Vec<GetArtistAlbumsAlbum>> {
        let query = GetArtistAlbumsQuery::new(channel_id.into(), browse_params.into());
        self.query(query).await
    }
    /// Gets information about an album and its tracks.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_albums("Dark Side Of The Moon").await.unwrap();
    /// yt.get_album(&results[0].album_id).await
    /// # };
    pub async fn get_album<'a, T: Into<AlbumID<'a>>>(&self, album_id: T) -> Result<GetAlbum> {
        let query = GetAlbumQuery::new(album_id);
        self.query(query).await
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
    ) -> Result<Vec<WatchPlaylistTrack>> {
        let query = GetWatchPlaylistQuery::new_from_video_id(video_id.into());
        self.query(query).await
    }
    /// Gets the `LyricsID` required to get lyrics.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// yt.get_lyrics_id(&results[0].video_id).await
    /// # };
    pub async fn get_lyrics_id<'a, T: Into<VideoID<'a>>>(
        &self,
        video_id: T,
    ) -> Result<LyricsID<'static>> {
        let query = GetLyricsIDQuery::new(video_id.into());
        self.query(query).await
    }
    /// Gets song lyrics and the source.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// let lyrics_id = yt.get_lyrics_id(&results[0].video_id).await.unwrap();
    /// yt.get_lyrics(lyrics_id).await
    /// # };
    pub async fn get_lyrics<'a, T: Into<LyricsID<'a>>>(&self, lyrics_id: T) -> Result<Lyrics> {
        let query = GetLyricsQuery::new(lyrics_id.into());
        self.query(query).await
    }
    /// Gets a playlists tracks.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_featured_playlists("Heavy metal").await.unwrap();
    /// yt.get_playlist_tracks(&results[0].playlist_id).await
    /// # };
    pub async fn get_playlist_tracks<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
    ) -> Result<Vec<PlaylistItem>> {
        let query = GetPlaylistTracksQuery::new(playlist_id.into());
        self.query(query).await
    }
    /// Gets information about a playlist.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_featured_playlists("Heavy metal").await.unwrap();
    /// yt.get_playlist_details(&results[0].playlist_id).await
    /// # };
    pub async fn get_playlist_details<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
    ) -> Result<GetPlaylistDetails> {
        let query = GetPlaylistDetailsQuery::new(playlist_id.into());
        self.query(query).await
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
        self.query(query).await
    }
    /// Fetches suggested artists from taste profile
    /// <https://music.youtube.com/tasteprofile>.
    /// Tasteprofile allows users to pick artists to update their
    /// recommendations.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_taste_profile().await
    /// # };
    pub async fn get_taste_profile(&self) -> Result<<GetTasteProfileQuery as Query<A>>::Output> {
        self.query(GetTasteProfileQuery).await
    }
    /// Sets artists as favourites to influence your recommendations.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_taste_profile().await.unwrap();
    /// yt.set_taste_profile(results.into_iter()
    ///     .take(5)
    ///     .map(|r| r.taste_tokens))
    ///     .await
    /// # };
    pub async fn set_taste_profile<'a>(
        &self,
        taste_tokens: impl IntoIterator<Item = TasteToken<'a>>,
    ) -> Result<<SetTasteProfileQuery<'a> as Query<A>>::Output> {
        self.query(SetTasteProfileQuery::new(taste_tokens)).await
    }
    /// Fetches 'Moods & Genres' categories.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_mood_categories().await
    /// # };
    pub async fn get_mood_categories(
        &self,
    ) -> Result<<GetMoodCategoriesQuery as Query<A>>::Output> {
        self.query(GetMoodCategoriesQuery).await
    }
    /// Returns a list of playlists for a given mood category.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_mood_categories().await.unwrap();
    /// yt.get_mood_playlists(&results[0].mood_categories[0].params).await
    /// # };
    pub async fn get_mood_playlists<'a, T: Into<MoodCategoryParams<'a>>>(
        &self,
        mood_params: T,
    ) -> Result<<GetMoodPlaylistsQuery as Query<A>>::Output> {
        self.query(GetMoodPlaylistsQuery::new(mood_params.into()))
            .await
    }
    /// Get the 'SongTrackingUrl' for a song. This is used to add items to
    /// history using `add_history_item()`.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let song = yt.search_songs("While My Guitar Gently Weeps")
    ///     .await
    ///     .unwrap()
    ///     .into_iter()
    ///     .next()
    ///     .unwrap();
    /// yt.get_song_tracking_url(song.video_id).await
    /// # };
    pub async fn get_song_tracking_url<'a, T: Into<VideoID<'a>>>(
        &self,
        video_id: T,
    ) -> Result<SongTrackingUrl<'static>> {
        let query = GetSongTrackingUrlQuery::new(video_id.into())?;
        self.query(query).await
    }
    /// Gets information about a Channel of Podcasts.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let podcasts = yt.search_podcasts("Rustacean").await.unwrap();
    /// let podcast = yt.get_podcast(&podcasts[0].podcast_id).await.unwrap();
    /// yt.get_channel(podcast.channels[0].id.as_ref().unwrap()).await
    /// # };
    pub async fn get_channel(
        &self,
        channel_id: impl Into<PodcastChannelID<'_>>,
    ) -> Result<<GetChannelQuery as Query<A>>::Output> {
        self.query(GetChannelQuery::new(channel_id)).await
    }
    /// Gets a list of all Episodes for a Channel. Note, if GetPodcastChannel
    /// doesn't contain `episode_params`, you can be sure that all episodes are
    /// already included at `episodes` key.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let podcasts = yt.search_podcasts("Rustacean").await.unwrap();
    /// let podcast = yt.get_podcast(&podcasts[0].podcast_id).await.unwrap();
    /// let channel_id = podcast.channels[0].id.as_ref().unwrap();
    /// let channel = yt.get_channel(channel_id).await.unwrap();
    /// match channel.episode_params {
    ///     Some(p) => yt.get_channel_episodes(channel_id, p).await,
    ///     None => Ok(channel.episodes),
    /// }
    /// # };
    pub async fn get_channel_episodes<'a>(
        &self,
        channel_id: impl Into<PodcastChannelID<'a>>,
        podcast_channel_params: impl Into<PodcastChannelParams<'a>>,
    ) -> Result<<GetChannelEpisodesQuery as Query<A>>::Output> {
        self.query(GetChannelEpisodesQuery::new(
            channel_id,
            podcast_channel_params,
        ))
        .await
    }
    /// Gets information about a Podcast, including Episodes.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let podcasts = yt.search_podcasts("Rustacean").await.unwrap();
    /// yt.get_podcast(&podcasts[0].podcast_id).await
    /// # };
    pub async fn get_podcast(
        &self,
        podcast_id: impl Into<PodcastID<'_>>,
    ) -> Result<<GetPodcastQuery as Query<A>>::Output> {
        self.query(GetPodcastQuery::new(podcast_id)).await
    }
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let episodes = yt.search_episodes("Ratatui").await.unwrap();
    /// yt.get_episode(&episodes[0].episode_id).await
    /// # };
    pub async fn get_episode(
        &self,
        episode_id: impl Into<EpisodeID<'_>>,
    ) -> Result<<GetEpisodeQuery as Query<A>>::Output> {
        self.query(GetEpisodeQuery::new(episode_id)).await
    }
    /// Gets the special 'New Episodes' playlist.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_new_episodes().await
    /// # };
    pub async fn get_new_episodes(&self) -> Result<<GetNewEpisodesQuery as Query<A>>::Output> {
        self.query(GetNewEpisodesQuery).await
    }
}

impl<A: LoggedIn> YtMusic<A> {
    /// Removes items from a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::parse::LibraryPlaylist { playlist_id, .. } =
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
    ///     outcome.iter().map(|o| (&o.set_video_id).into()),
    /// ).await
    /// # };
    pub async fn remove_playlist_items<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        video_items: impl IntoIterator<Item = SetVideoID<'a>>,
    ) -> Result<()> {
        let query = RemovePlaylistItemsQuery::new(playlist_id.into(), video_items);
        self.query(query).await
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
    pub async fn edit_playlist(&self, query: EditPlaylistQuery<'_>) -> Result<ApiOutcome> {
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
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
        self.query(query).await
    }
    /// Removes a list of items from your recently played history.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let history = yt.get_history().await.unwrap();
    /// let first_history_token = match history.first().unwrap().items.first().unwrap() {
    ///     ytmapi_rs::parse::HistoryItem::Song(i) => &i.feedback_token_remove,
    ///     ytmapi_rs::parse::HistoryItem::Video(i) => &i.feedback_token_remove,
    ///     ytmapi_rs::parse::HistoryItem::Episode(i) => &i.feedback_token_remove,
    ///     ytmapi_rs::parse::HistoryItem::UploadSong(i) => &i.feedback_token_remove,
    /// }.into();
    /// yt.remove_history_items(vec![first_history_token]).await
    /// # };
    pub async fn remove_history_items(
        &self,
        feedback_tokens: impl IntoIterator<Item = FeedbackTokenRemoveFromHistory<'_>>,
    ) -> Result<Vec<ApiOutcome>> {
        let query = RemoveHistoryItemsQuery::new(feedback_tokens);
        self.query(query).await
    }
    // TODO: Docs / alternative constructors.
    pub async fn edit_song_library_status(
        &self,
        query: EditSongLibraryStatusQuery<'_>,
    ) -> Result<Vec<ApiOutcome>> {
        self.query(query).await
    }
    /// Sets the like status for a song.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.search_songs("While My Guitar Gently Weeps").await.unwrap();
    /// yt.rate_song(&results[0].video_id, ytmapi_rs::common::LikeStatus::Liked).await
    /// # };
    pub async fn rate_song<'a, T: Into<VideoID<'a>>>(
        &self,
        video_id: T,
        rating: LikeStatus,
    ) -> Result<()> {
        let query = RateSongQuery::new(video_id.into(), rating);
        self.query(query).await
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
    ///     ytmapi_rs::common::LikeStatus::Liked,
    /// ).await
    /// # };
    pub async fn rate_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        rating: LikeStatus,
    ) -> Result<()> {
        let query = RatePlaylistQuery::new(playlist_id.into(), rating);
        self.query(query).await
    }
    /// Deletes a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_playlists().await.unwrap();
    /// yt.delete_playlist(&results[0].playlist_id).await
    /// # };
    pub async fn delete_playlist<'a, T: Into<PlaylistID<'a>>>(&self, playlist_id: T) -> Result<()> {
        let query = DeletePlaylistQuery::new(playlist_id.into());
        self.query(query).await
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
    ///     ytmapi_rs::query::playlist::PrivacyStatus::Public,
    /// )
    ///     .with_source(&playlists[0].playlist_id);
    /// yt.create_playlist(query).await
    /// # };
    pub async fn create_playlist<T: CreatePlaylistType>(
        &self,
        query: CreatePlaylistQuery<'_, T>,
    ) -> Result<PlaylistID<'static>> {
        self.query(query).await
    }
    /// Adds video items to a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::parse::LibraryPlaylist { playlist_id, .. } =
    ///     yt.get_library_playlists().await.unwrap().pop().unwrap();
    /// let songs = yt.search_songs("Master of puppets").await.unwrap();
    /// yt.add_video_items_to_playlist(
    ///     playlist_id,
    ///     songs.iter().map(|s| (&s.video_id).into())
    /// ).await
    /// # };
    pub async fn add_video_items_to_playlist<'a, T: Into<PlaylistID<'a>>>(
        &self,
        playlist_id: T,
        video_ids: impl IntoIterator<Item = VideoID<'a>>,
    ) -> Result<Vec<AddPlaylistItem>> {
        let query = AddPlaylistItemsQuery::new_from_videos(
            playlist_id.into(),
            video_ids,
            DuplicateHandlingMode::default(),
        );
        self.query(query).await
    }
    /// Appends another playlist to a playlist you own.
    ///  ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let ytmapi_rs::parse::LibraryPlaylist { playlist_id, .. } =
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
        self.query(query).await
    }
    /// Gets a list of all playlists in your Library.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// yt.get_library_playlists().await;
    /// # };
    pub async fn get_library_playlists(&self) -> Result<Vec<LibraryPlaylist>> {
        let query = GetLibraryPlaylistsQuery;
        self.query(query).await
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
        self.query(query).await
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
    pub async fn get_library_songs(&self) -> Result<<GetLibrarySongsQuery as Query<A>>::Output> {
        let query = GetLibrarySongsQuery::default();
        self.query(query).await
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
        self.query(query).await
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
    pub async fn get_library_artist_subscriptions(&self) -> Result<Vec<LibraryArtistSubscription>> {
        let query = GetLibraryArtistSubscriptionsQuery::default();
        self.query(query).await
    }
    /// Gets a list of all podcasts in your Library.
    /// # Additional functionality
    /// See [`GetLibraryPodcastsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryPodcastsQuery]: crate::query::GetLibraryPodcastsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_podcasts().await;
    /// # };
    pub async fn get_library_podcasts(
        &self,
    ) -> Result<<GetLibraryPodcastsQuery as Query<A>>::Output> {
        let query = GetLibraryPodcastsQuery::default();
        self.query(query).await
    }
    /// Gets a list of all channels in your Library.
    /// # Additional functionality
    /// See [`GetLibraryChannelsQuery`] and [`YtMusic.query()`]
    /// for more ways to construct and run.
    ///
    /// [`YtMusic.query()`]: crate::YtMusic::query
    /// [GetLibraryChannelsQuery]: crate::query::GetLibraryChannelsQuery
    ///
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_library_channels().await;
    /// # };
    pub async fn get_library_channels(
        &self,
    ) -> Result<<GetLibraryChannelsQuery as Query<A>>::Output> {
        let query = GetLibraryChannelsQuery::default();
        self.query(query).await
    }
    /// Gets your recently played history.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let results = yt.get_history().await;
    /// # };
    pub async fn get_history(&self) -> Result<Vec<HistoryPeriod>> {
        let query = GetHistoryQuery;
        self.query(query).await
    }
    /// Adds an item to the accounts history.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// let song = yt.search_songs("While My Guitar Gently Weeps")
    ///     .await
    ///     .unwrap()
    ///     .into_iter()
    ///     .next()
    ///     .unwrap();
    /// let url = yt.get_song_tracking_url(song.video_id).await.unwrap();
    /// yt.add_history_item(url).await
    /// # };
    pub async fn add_history_item<'a, T: Into<SongTrackingUrl<'a>>>(
        &self,
        song_url: T,
    ) -> Result<<AddHistoryItemQuery<'a> as Query<A>>::Output> {
        self.query(AddHistoryItemQuery::new(song_url.into())).await
    }
    /// Gets information about an artist and their top releases.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// fixme
    /// # };
    pub async fn subscribe_artists<'a>(
        &self,
        channels: impl IntoIterator<Item = ArtistChannelID<'a>>,
    ) -> Result<()> {
        self.query(SubscribeArtistsQuery::new(channels)).await
    }
    /// Gets information about an artist and their top releases.
    /// ```no_run
    /// # async {
    /// let yt = ytmapi_rs::YtMusic::from_cookie("FAKE COOKIE").await.unwrap();
    /// fixme
    /// # };
    pub async fn unsubscribe_artists<'a>(
        &self,
        channels: impl IntoIterator<Item = ArtistChannelID<'a>>,
    ) -> Result<()> {
        self.query(UnsubscribeArtistsQuery::new(channels)).await
    }
}
