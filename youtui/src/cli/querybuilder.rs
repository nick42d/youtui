use ytmapi_rs::{
    auth::AuthToken,
    common::{
        AlbumID, BrowseParams, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromHistory,
        PlaylistID, SetVideoID, YoutubeID,
    },
    parse::LikeStatus,
    query::{
        rate::{RatePlaylistQuery, RateSongQuery},
        AddPlaylistItemsQuery, AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter,
        CreatePlaylistQuery, DeletePlaylistQuery, EditPlaylistQuery, EditSongLibraryStatusQuery,
        EpisodesFilter, FeaturedPlaylistsFilter, GetAlbumQuery, GetArtistAlbumsQuery,
        GetArtistQuery, GetHistoryQuery, GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery,
        GetLibraryArtistsQuery, GetLibraryPlaylistsQuery, GetLibrarySongsQuery, GetPlaylistQuery,
        GetSearchSuggestionsQuery, PlaylistsFilter, PodcastsFilter, ProfilesFilter, Query,
        RemoveHistoryItemsQuery, RemovePlaylistItemsQuery, SearchQuery, SongsFilter, VideosFilter,
    },
    ChannelID, VideoID, YtMusic,
};

use crate::Command;

pub struct CliQuery {
    pub query_type: QueryType,
    pub show_source: bool,
}

pub enum QueryType {
    FromSourceFile(String),
    FromApi,
}

pub async fn command_to_query<A: AuthToken>(
    command: Command,
    cli_query: CliQuery,
    yt: &YtMusic<A>,
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
        Command::EditLibraryStatus { feedback_tokens } => {
            get_string_output_of_query(
                yt,
                EditSongLibraryStatusQuery::new(
                    feedback_tokens
                        .iter()
                        .map(|s| FeedbackTokenAddToLibrary::from_raw(s))
                        .collect(),
                ),
                cli_query,
            )
            .await
        }
    }
}

async fn get_string_output_of_query<Q: Query, A: AuthToken>(
    yt: &YtMusic<A>,
    q: Q,
    cli_query: CliQuery,
) -> crate::Result<String> {
    match cli_query {
        CliQuery {
            query_type: QueryType::FromApi,
            show_source: true,
        } => yt
            .raw_query(q)
            .await
            .map(|r| r.destructure_json())
            .map_err(|e| e.into()),
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
        } => YtMusic::<A>::process_json(source, q)
            .map(|r| format!("{:#?}", r))
            .map_err(|e| e.into()),
    }
}
