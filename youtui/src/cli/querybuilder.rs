use ytmapi_rs::{
    auth::AuthToken,
    common::{BrowseParams, PlaylistID, YoutubeID},
    query::{
        AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, DeletePlaylistQuery, EpisodesFilter,
        FeaturedPlaylistsFilter, GetArtistAlbumsQuery, GetArtistQuery, GetLibraryArtistsQuery,
        GetLibraryPlaylistsQuery, GetSearchSuggestionsQuery, PlaylistsFilter, PodcastsFilter,
        ProfilesFilter, Query, SearchQuery, SongsFilter, VideosFilter,
    },
    ChannelID, YtMusic,
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
        } => YtMusic::<A>::process_json(source,q)
            .map(|r| format!("{:#?}", r))
            .map_err(|e| e.into()),
    }
}