use crate::api::DynamicYtMusic;
use crate::Command;
use anyhow::bail;
use std::borrow::Borrow;
use std::fmt::Debug;
use ytmapi_rs::auth::noauth::NoAuthToken;
use ytmapi_rs::auth::{BrowserToken, OAuthToken};
use ytmapi_rs::common::{
    AlbumID, ArtistChannelID, BrowseParams, EpisodeID, FeedbackTokenAddToLibrary,
    FeedbackTokenRemoveFromHistory, LikeStatus, LyricsID, MoodCategoryParams, PlaylistID,
    PodcastChannelID, PodcastChannelParams, PodcastID, SetVideoID, SongTrackingUrl, TasteToken,
    TasteTokenImpression, TasteTokenSelection, UploadAlbumID, UploadArtistID, UploadEntityID,
    VideoID, YoutubeID,
};
use ytmapi_rs::continuations::ParseFromContinuable;
use ytmapi_rs::parse::ParseFrom;
use ytmapi_rs::process_json;
use ytmapi_rs::query::library::{GetLibraryChannelsQuery, GetLibraryPodcastsQuery};
use ytmapi_rs::query::playlist::GetPlaylistDetailsQuery;
use ytmapi_rs::query::rate::{RatePlaylistQuery, RateSongQuery};
use ytmapi_rs::query::search::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    PlaylistsFilter, PodcastsFilter, ProfilesFilter, SongsFilter, VideosFilter,
};
use ytmapi_rs::query::song::GetSongTrackingUrlQuery;
use ytmapi_rs::query::{
    AddHistoryItemQuery, AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery,
    DeleteUploadEntityQuery, EditPlaylistQuery, EditSongLibraryStatusQuery, GetAlbumQuery,
    GetArtistAlbumsQuery, GetArtistQuery, GetChannelEpisodesQuery, GetChannelQuery,
    GetContinuationsQuery, GetEpisodeQuery, GetHistoryQuery, GetLibraryAlbumsQuery,
    GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery, GetLibraryPlaylistsQuery,
    GetLibrarySongsQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
    GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
    GetLyricsIDQuery, GetLyricsQuery, GetMoodCategoriesQuery, GetMoodPlaylistsQuery,
    GetNewEpisodesQuery, GetPlaylistTracksQuery, GetPodcastQuery, GetSearchSuggestionsQuery,
    GetTasteProfileQuery, GetWatchPlaylistQuery, PostQuery, Query, RemoveHistoryItemsQuery,
    RemovePlaylistItemsQuery, SearchQuery, SetTasteProfileQuery, SubscribeArtistsQuery,
};

pub struct CliQuery {
    pub query_type: QueryType,
    pub show_source: bool,
}

pub enum QueryType {
    FromSourceFiles(Vec<String>),
    FromApi,
}

pub async fn command_to_query(
    command: Command,
    cli_query: CliQuery,
    yt: DynamicYtMusic,
) -> anyhow::Result<String> {
    match command {
        Command::GetSearchSuggestions { query } => {
            get_string_output_of_query(yt, GetSearchSuggestionsQuery::from(query), cli_query).await
        }
        Command::GetArtist { channel_id } => {
            get_string_output_of_query(
                yt,
                GetArtistQuery::new(ArtistChannelID::from_raw(channel_id)),
                cli_query,
            )
            .await
        }
        Command::SubscribeArtists { channel_ids } => {
            get_string_output_of_query(yt, SubscribeArtistsQuery::new(channel_ids), cli_query).await
        }
        Command::UnsubscribeArtists { channel_ids } => {
            get_string_output_of_query(yt, UnsubscribeArtistsQuery::new(channel_ids), cli_query)
                .await
        }
        Command::GetPlaylistDetails { playlist_id } => {
            get_string_output_of_query(
                yt,
                GetPlaylistDetailsQuery::new(PlaylistID::from_raw(playlist_id)),
                cli_query,
            )
            .await
        }
        Command::GetPlaylistTracks {
            playlist_id,
            max_pages,
        } => {
            get_string_output_of_streaming_query(
                yt,
                GetPlaylistTracksQuery::new(PlaylistID::from_raw(playlist_id)),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetArtistAlbums {
            channel_id,
            browse_params,
        } => {
            get_string_output_of_query(
                yt,
                GetArtistAlbumsQuery::new(
                    ArtistChannelID::from_raw(channel_id),
                    BrowseParams::from_raw(browse_params),
                ),
                cli_query,
            )
            .await
        }
        Command::Search { query } => {
            get_string_output_of_query(yt, SearchQuery::new(query), cli_query).await
        }
        Command::SearchArtists { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(ArtistsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchAlbums { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(AlbumsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchSongs { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(SongsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchPlaylists { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(PlaylistsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchCommunityPlaylists { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(CommunityPlaylistsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchFeaturedPlaylists { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(FeaturedPlaylistsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchVideos { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(VideosFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchEpisodes { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(EpisodesFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchProfiles { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(ProfilesFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::SearchPodcasts { query, max_pages } => {
            get_string_output_of_streaming_query(
                yt,
                SearchQuery::new(query).with_filter(PodcastsFilter),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::DeletePlaylist { playlist_id } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                DeletePlaylistQuery::new(PlaylistID::from_raw(playlist_id)),
                cli_query,
            )
            .await
        }
        Command::GetAlbum { browse_id } => {
            get_string_output_of_query(
                yt,
                GetAlbumQuery::new(AlbumID::from_raw(browse_id)),
                cli_query,
            )
            .await
        }
        Command::CreatePlaylist { title, description } => {
            get_string_output_of_query(
                yt,
                CreatePlaylistQuery::new(
                    title.as_str(),
                    description.as_deref(),
                    Default::default(),
                ),
                cli_query,
            )
            .await
        }
        Command::RemovePlaylistItems {
            playlist_id,
            video_ids: set_video_ids,
        } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                RemovePlaylistItemsQuery::new(
                    PlaylistID::from_raw(playlist_id),
                    set_video_ids.iter().map(SetVideoID::from_raw),
                ),
                cli_query,
            )
            .await
        }
        Command::AddVideosToPlaylist {
            playlist_id,
            video_ids,
        } => {
            get_string_output_of_query(
                yt,
                AddPlaylistItemsQuery::new_from_videos(
                    PlaylistID::from_raw(playlist_id),
                    video_ids.iter().map(VideoID::from_raw),
                    Default::default(),
                ),
                cli_query,
            )
            .await
        }
        Command::EditPlaylistTitle {
            playlist_id,
            new_title,
        } => {
            get_string_output_of_query(
                yt,
                EditPlaylistQuery::new_title(PlaylistID::from_raw(playlist_id), new_title),
                cli_query,
            )
            .await
        }
        Command::AddPlaylistToPlaylist {
            playlist_id,
            from_playlist_id,
        } => {
            get_string_output_of_query(
                yt,
                AddPlaylistItemsQuery::new_from_playlist(
                    PlaylistID::from_raw(playlist_id),
                    PlaylistID::from_raw(from_playlist_id),
                ),
                cli_query,
            )
            .await
        }
        Command::GetLibraryPlaylists { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryPlaylistsQuery,
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryArtists { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryArtistsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibrarySongs { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibrarySongsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryAlbums { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryAlbumsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryArtistSubscriptions { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryArtistSubscriptionsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryPodcasts { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryPodcastsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryChannels { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryChannelsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetHistory => {
            get_string_output_of_query_browser_or_oauth(yt, GetHistoryQuery, cli_query).await
        }
        Command::RemoveHistoryItems { feedback_tokens } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                RemoveHistoryItemsQuery::new(
                    feedback_tokens
                        .iter()
                        .map(FeedbackTokenRemoveFromHistory::from_raw),
                ),
                cli_query,
            )
            .await
        }
        Command::RateSong {
            video_id,
            like_status,
        } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                RateSongQuery::new(
                    VideoID::from_raw(video_id),
                    match like_status.as_str() {
                        "Like" => LikeStatus::Liked,
                        "Dislike" => LikeStatus::Disliked,
                        "Indifferent" => LikeStatus::Indifferent,
                        other => panic!("Unhandled like status <{other}>"),
                    },
                ),
                cli_query,
            )
            .await
        }
        Command::RatePlaylist {
            playlist_id,
            like_status,
        } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                RatePlaylistQuery::new(
                    PlaylistID::from_raw(playlist_id),
                    match like_status.as_str() {
                        "Like" => LikeStatus::Liked,
                        "Dislike" => LikeStatus::Disliked,
                        "Indifferent" => LikeStatus::Indifferent,
                        other => panic!("Unhandled like status <{other}>"),
                    },
                ),
                cli_query,
            )
            .await
        }
        Command::EditSongLibraryStatus { feedback_tokens } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                // Internal knowledge: Even though the string tokens we are provided could be
                // either Add or Remove tokens, it's OK to just provide
                // FeedBackTokenAddToLibrary's, as the tokens themselves determine if they will add
                // or remove.
                EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(
                    feedback_tokens
                        .iter()
                        .map(FeedbackTokenAddToLibrary::from_raw),
                ),
                cli_query,
            )
            .await
        }
        Command::GetLibraryUploadSongs { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryUploadSongsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryUploadArtists { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryUploadArtistsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryUploadAlbums { max_pages } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryUploadAlbumsQuery::default(),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryUploadArtist {
            upload_artist_id,
            max_pages,
        } => {
            get_string_output_of_streaming_query_browser_or_oauth(
                yt,
                GetLibraryUploadArtistQuery::new(UploadArtistID::from_raw(upload_artist_id)),
                cli_query,
                max_pages,
            )
            .await
        }
        Command::GetLibraryUploadAlbum { upload_album_id } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                GetLibraryUploadAlbumQuery::new(UploadAlbumID::from_raw(upload_album_id)),
                cli_query,
            )
            .await
        }
        Command::DeleteUploadEntity { upload_entity_id } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                DeleteUploadEntityQuery::new(UploadEntityID::from_raw(upload_entity_id)),
                cli_query,
            )
            .await
        }
        Command::GetTasteProfile => {
            get_string_output_of_query(yt, GetTasteProfileQuery, cli_query).await
        }
        Command::SetTasteProfile {
            impression_token,
            selection_token,
        } => {
            get_string_output_of_query(
                yt,
                SetTasteProfileQuery::new([TasteToken {
                    impression_value: TasteTokenImpression::from_raw(impression_token),
                    selection_value: TasteTokenSelection::from_raw(selection_token),
                }]),
                cli_query,
            )
            .await
        }
        Command::GetMoodCategories => {
            get_string_output_of_query(yt, GetMoodCategoriesQuery, cli_query).await
        }
        Command::GetMoodPlaylists {
            mood_category_params,
        } => {
            get_string_output_of_query(
                yt,
                GetMoodPlaylistsQuery::new(MoodCategoryParams::from_raw(mood_category_params)),
                cli_query,
            )
            .await
        }
        Command::AddHistoryItem {
            song_tracking_url: song_url,
        } => {
            get_string_output_of_query_browser_or_oauth(
                yt,
                AddHistoryItemQuery::new(SongTrackingUrl::from_raw(song_url)),
                cli_query,
            )
            .await
        }
        Command::GetSongTrackingUrl { video_id } => {
            get_string_output_of_query(
                yt,
                GetSongTrackingUrlQuery::new(VideoID::from_raw(video_id))?,
                cli_query,
            )
            .await
        }
        Command::GetChannel { channel_id } => {
            get_string_output_of_query(
                yt,
                GetChannelQuery::new(PodcastChannelID::from_raw(channel_id)),
                cli_query,
            )
            .await
        }
        Command::GetChannelEpisodes {
            channel_id,
            podcast_channel_params,
        } => {
            get_string_output_of_query(
                yt,
                GetChannelEpisodesQuery::new(
                    PodcastChannelID::from_raw(channel_id),
                    PodcastChannelParams::from_raw(podcast_channel_params),
                ),
                cli_query,
            )
            .await
        }
        Command::GetPodcast { podcast_id } => {
            get_string_output_of_query(
                yt,
                GetPodcastQuery::new(PodcastID::from_raw(podcast_id)),
                cli_query,
            )
            .await
        }
        Command::GetEpisode { video_id } => {
            get_string_output_of_query(
                yt,
                GetEpisodeQuery::new(EpisodeID::from_raw(video_id)),
                cli_query,
            )
            .await
        }
        Command::GetNewEpisodes => {
            get_string_output_of_query(yt, GetNewEpisodesQuery, cli_query).await
        }
        Command::GetLyricsID { video_id } => {
            get_string_output_of_query(
                yt,
                GetLyricsIDQuery::new(VideoID::from_raw(video_id)),
                cli_query,
            )
            .await
        }
        Command::GetLyrics { lyrics_id } => {
            get_string_output_of_query(
                yt,
                GetLyricsQuery::new(LyricsID::from_raw(lyrics_id)),
                cli_query,
            )
            .await
        }
        Command::GetWatchPlaylist {
            video_id,
            max_pages,
        } => {
            get_string_output_of_streaming_query(
                yt,
                GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(video_id)),
                cli_query,
                max_pages,
            )
            .await
        }
    }
}

async fn get_string_output_of_query<Q, O>(
    yt: DynamicYtMusic,
    q: impl Borrow<Q>,
    cli_query: CliQuery,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    Q: Query<NoAuthToken, Output = O>,
    O: ParseFrom<Q>,
{
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => yt.query_source(q).await,
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: false,
        } => yt.query(q).await.map(|r| format!("{r:#?}")),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: true,
        } => Ok(sources.into_iter().next().unwrap_or_default()),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: false,
        } => {
            // Note - if multiple sources are provided, only the first is processed - the
            // rest are ignored.
            if let Some(first_source) = sources.into_iter().next() {
                process_json_based_on_dyn_api(&yt, first_source, q)
            } else {
                Ok(String::new())
            }
        }
    }
}

async fn get_string_output_of_query_browser_or_oauth<Q, O>(
    yt: DynamicYtMusic,
    q: impl Borrow<Q>,
    cli_query: CliQuery,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    O: ParseFrom<Q>,
{
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => yt.query_source_browser_or_oauth(q).await,
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: false,
        } => yt
            .query_browser_or_oauth(q)
            .await
            .map(|r| format!("{r:#?}")),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: true,
        } => Ok(sources.into_iter().next().unwrap_or_default()),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: false,
        } => {
            // Note - if multiple sources are provided, only the first is processed - the
            // rest are ignored.
            if let Some(first_source) = sources.into_iter().next() {
                process_json_based_on_dyn_api_browser_or_oauth(&yt, first_source, q)
            } else {
                Ok(String::new())
            }
        }
    }
}

async fn get_string_output_of_streaming_query<Q, O>(
    yt: DynamicYtMusic,
    q: impl Borrow<Q>,
    cli_query: CliQuery,
    max_pages: usize,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    Q: Query<NoAuthToken, Output = O>,
    Q: PostQuery,
    O: ParseFromContinuable<Q>,
{
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => yt._stream_source(q.borrow(), max_pages).await,
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: false,
        } => yt._stream(q, max_pages).await.map(|r| format!("{r:#?}")),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: true,
        } => {
            Ok(
                // Replace with standard library method once stabilised.
                itertools::intersperse(sources.into_iter().take(max_pages), "\n".to_string())
                    .collect(),
            )
        }
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: false,
        } => {
            let mut output_arr = vec![];
            let mut sources_iter = sources.into_iter().take(max_pages);
            if let Some(first_source) = sources_iter.next() {
                output_arr.push(process_json_based_on_dyn_api::<Q, O>(
                    &yt,
                    first_source,
                    q.borrow(),
                )?)
            }
            for source in sources_iter {
                let continuation_query = GetContinuationsQuery::new_mock_unchecked(q.borrow());
                output_arr.push(process_json_based_on_dyn_api(
                    &yt,
                    source,
                    continuation_query,
                )?)
            }
            Ok(output_arr.join("\n"))
        }
    }
}

async fn get_string_output_of_streaming_query_browser_or_oauth<Q, O>(
    yt: DynamicYtMusic,
    q: impl Borrow<Q>,
    cli_query: CliQuery,
    max_pages: usize,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    Q: PostQuery,
    O: ParseFromContinuable<Q>,
{
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => {
            yt.stream_source_browser_or_oauth(q.borrow(), max_pages)
                .await
        }
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: false,
        } => yt
            .stream_browser_or_oauth(q, max_pages)
            .await
            .map(|r| format!("{r:#?}")),
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: true,
        } => {
            Ok(
                // Replace with standard library method once stabilised.
                itertools::intersperse(sources.into_iter().take(max_pages), "\n".to_string())
                    .collect(),
            )
        }
        CliQuery {
            query_type: QueryType::FromSourceFiles(sources),
            show_source: false,
        } => {
            let mut output_arr = vec![];
            let mut sources_iter = sources.into_iter().take(max_pages);
            if let Some(first_source) = sources_iter.next() {
                output_arr.push(process_json_based_on_dyn_api_browser_or_oauth::<Q, O>(
                    &yt,
                    first_source,
                    q.borrow(),
                )?)
            }
            for source in sources_iter {
                let continuation_query = GetContinuationsQuery::new_mock_unchecked(q.borrow());
                output_arr.push(process_json_based_on_dyn_api_browser_or_oauth(
                    &yt,
                    source,
                    continuation_query,
                )?)
            }
            Ok(output_arr.join("\n"))
        }
    }
}

fn process_json_based_on_dyn_api<Q, O>(
    yt: &DynamicYtMusic,
    source: String,
    query: impl Borrow<Q>,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    Q: Query<NoAuthToken, Output = O>,
    O: Debug,
{
    // The matching on yt is a neat hack to ensure process_json utilises the same
    // AuthType as was set in config. This works as the config step sets
    // the variant of DynamicYtMusic.
    match yt {
        DynamicYtMusic::Browser(_) => process_json::<Q, BrowserToken>(source, query)
            .map(|r| format!("{r:#?}"))
            .map_err(|e| e.into()),
        DynamicYtMusic::OAuth(_) => process_json::<Q, OAuthToken>(source, query)
            .map(|r| format!("{r:#?}"))
            .map_err(|e| e.into()),
        DynamicYtMusic::NoAuth(_) => process_json::<Q, NoAuthToken>(source, query)
            .map(|r| format!("{r:#?}"))
            .map_err(|e| e.into()),
    }
}

fn process_json_based_on_dyn_api_browser_or_oauth<Q, O>(
    yt: &DynamicYtMusic,
    source: String,
    query: impl Borrow<Q>,
) -> anyhow::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    O: Debug,
{
    // The matching on yt is a neat hack to ensure process_json utilises the same
    // AuthType as was set in config. This works as the config step sets
    // the variant of DynamicYtMusic.
    match yt {
        DynamicYtMusic::Browser(_) => process_json::<Q, BrowserToken>(source, query)
            .map(|r| format!("{r:#?}"))
            .map_err(|e| e.into()),
        DynamicYtMusic::OAuth(_) => process_json::<Q, OAuthToken>(source, query)
            .map(|r| format!("{r:#?}"))
            .map_err(|e| e.into()),
        DynamicYtMusic::NoAuth(_) => {
            bail!("Tried to process a query that doesnt support not being authenticated")
        }
    }
}
