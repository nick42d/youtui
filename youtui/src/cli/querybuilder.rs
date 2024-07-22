use ytmapi_rs::{
    auth::{AuthToken, BrowserToken, OAuthToken},
    common::{
        AlbumID, BrowseParams, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromHistory,
        PlaylistID, SetVideoID, UploadAlbumID, UploadArtistID, UploadEntityID, YoutubeID,
    },
    parse::{LikeStatus, ParseFrom},
    process_json,
    query::{
        rate::{RatePlaylistQuery, RateSongQuery},
        AddPlaylistItemsQuery, AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter,
        CreatePlaylistQuery, DeletePlaylistQuery, DeleteUploadEntityQuery, EditPlaylistQuery,
        EditSongLibraryStatusQuery, EpisodesFilter, FeaturedPlaylistsFilter, GetAlbumQuery,
        GetArtistAlbumsQuery, GetArtistQuery, GetHistoryQuery, GetLibraryAlbumsQuery,
        GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery, GetLibraryPlaylistsQuery,
        GetLibrarySongsQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
        GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
        GetPlaylistQuery, GetSearchSuggestionsQuery, PlaylistsFilter, PodcastsFilter,
        ProfilesFilter, Query, RemoveHistoryItemsQuery, RemovePlaylistItemsQuery, SearchQuery,
        SongsFilter, VideosFilter,
    },
    ChannelID, VideoID, YtMusic,
};

use crate::{api::DynamicYtMusic, Command};

pub struct CliQuery {
    pub query_type: QueryType,
    pub show_source: bool,
}

pub enum QueryType {
    FromSourceFile(String),
    FromApi,
}

pub async fn command_to_query(
    command: Command,
    cli_query: CliQuery,
    yt: DynamicYtMusic,
) -> crate::Result<String> {
    match command {
        Command::GetSearchSuggestions { query } => {
            get_string_output_of_query(yt, GetSearchSuggestionsQuery::from(query), cli_query).await
        }
        Command::GetArtist { channel_id } => {
            get_string_output_of_query(
                yt,
                GetArtistQuery::new(ChannelID::from_raw(channel_id)),
                cli_query,
            )
            .await
        }
        Command::GetPlaylist { playlist_id } => {
            get_string_output_of_query(
                yt,
                GetPlaylistQuery::new(PlaylistID::from_raw(playlist_id)),
                cli_query,
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
                    ChannelID::from_raw(channel_id),
                    BrowseParams::from_raw(browse_params),
                ),
                cli_query,
            )
            .await
        }
        Command::GetLibraryPlaylists => {
            get_string_output_of_query(yt, GetLibraryPlaylistsQuery, cli_query).await
        }
        Command::GetLibraryArtists => {
            get_string_output_of_query(yt, GetLibraryArtistsQuery::default(), cli_query).await
        }
        Command::Search { query } => {
            get_string_output_of_query(yt, SearchQuery::new(query), cli_query).await
        }
        Command::SearchArtists { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(ArtistsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchAlbums { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(AlbumsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchSongs { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(SongsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchPlaylists { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(PlaylistsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchCommunityPlaylists { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(CommunityPlaylistsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchFeaturedPlaylists { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(FeaturedPlaylistsFilter),
                cli_query,
            )
            .await
        }
        Command::SearchVideos { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(VideosFilter),
                cli_query,
            )
            .await
        }
        Command::SearchEpisodes { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(EpisodesFilter),
                cli_query,
            )
            .await
        }
        Command::SearchProfiles { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(ProfilesFilter),
                cli_query,
            )
            .await
        }
        Command::SearchPodcasts { query } => {
            get_string_output_of_query(
                yt,
                SearchQuery::new(query).with_filter(PodcastsFilter),
                cli_query,
            )
            .await
        }
        Command::DeletePlaylist { playlist_id } => {
            get_string_output_of_query(
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
            get_string_output_of_query(
                yt,
                RemovePlaylistItemsQuery::new(
                    PlaylistID::from_raw(playlist_id),
                    set_video_ids.iter().map(SetVideoID::from_raw).collect(),
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
                    video_ids.iter().map(VideoID::from_raw).collect(),
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
        Command::GetLibrarySongs => {
            get_string_output_of_query(yt, GetLibrarySongsQuery::default(), cli_query).await
        }
        Command::GetLibraryAlbums => {
            get_string_output_of_query(yt, GetLibraryAlbumsQuery::default(), cli_query).await
        }
        Command::GetLibraryArtistSubscriptions => {
            get_string_output_of_query(yt, GetLibraryArtistSubscriptionsQuery::default(), cli_query)
                .await
        }
        Command::GetHistory => get_string_output_of_query(yt, GetHistoryQuery, cli_query).await,
        Command::RemoveHistoryItems { feedback_tokens } => {
            get_string_output_of_query(
                yt,
                RemoveHistoryItemsQuery::new(
                    feedback_tokens
                        .iter()
                        .map(FeedbackTokenRemoveFromHistory::from_raw)
                        .collect(),
                ),
                cli_query,
            )
            .await
        }
        Command::RateSong {
            video_id,
            like_status,
        } => {
            get_string_output_of_query(
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
            get_string_output_of_query(
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
            get_string_output_of_query(
                yt,
                // Internal knowledge: Even though the string tokens we are provided could be
                // either Add or Remove tokens, it's OK to just provide
                // FeedBackTokenAddToLibrary's, as the tokens themselves determine if they will add
                // or remove.
                EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(
                    feedback_tokens
                        .iter()
                        .map(FeedbackTokenAddToLibrary::from_raw)
                        .collect(),
                ),
                cli_query,
            )
            .await
        }
        Command::GetLibraryUploadSongs => {
            get_string_output_of_query(yt, GetLibraryUploadSongsQuery::default(), cli_query).await
        }
        Command::GetLibraryUploadArtists => {
            get_string_output_of_query(yt, GetLibraryUploadArtistsQuery::default(), cli_query).await
        }
        Command::GetLibraryUploadAlbums => {
            get_string_output_of_query(yt, GetLibraryUploadAlbumsQuery::default(), cli_query).await
        }
        Command::GetLibraryUploadArtist { upload_artist_id } => {
            get_string_output_of_query(
                yt,
                GetLibraryUploadArtistQuery::new(UploadArtistID::from_raw(upload_artist_id)),
                cli_query,
            )
            .await
        }
        Command::GetLibraryUploadAlbum { upload_album_id } => {
            get_string_output_of_query(
                yt,
                GetLibraryUploadAlbumQuery::new(UploadAlbumID::from_raw(upload_album_id)),
                cli_query,
            )
            .await
        }
        Command::DeleteUploadEntity { upload_entity_id } => {
            get_string_output_of_query(
                yt,
                DeleteUploadEntityQuery::new(UploadEntityID::from_raw(upload_entity_id)),
                cli_query,
            )
            .await
        }
    }
}

async fn get_string_output_of_query<Q, O>(
    yt: DynamicYtMusic,
    q: Q,
    cli_query: CliQuery,
) -> crate::Result<String>
where
    Q: Query<BrowserToken, Output = O>,
    Q: Query<OAuthToken, Output = O>,
    O: ParseFrom<Q>,
{
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => yt.query_source(q).await.map_err(|e| e.into()),
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: false,
        } => yt
            .query(q)
            .await
            .map(|r| format!("{:#?}", r))
            .map_err(|e| e.into()),
        CliQuery {
            query_type: QueryType::FromSourceFile(source),
            show_source: true,
        } => Ok(source),
        CliQuery {
            query_type: QueryType::FromSourceFile(source),
            show_source: false,
        } => {
            // Neat hack to ensure process_json utilises the same AuthType as was set in
            // config. This works as the config step sets the variant of
            // DynamicYtMusic.
            match yt {
                DynamicYtMusic::Browser(_) => process_json::<Q, BrowserToken>(source, q)
                    .map(|r| format!("{:#?}", r))
                    .map_err(|e| e.into()),
                DynamicYtMusic::OAuth(_) => process_json::<Q, OAuthToken>(source, q)
                    .map(|r| format!("{:#?}", r))
                    .map_err(|e| e.into()),
            }
        }
    }
}
