use ytmapi_rs::{
    auth::AuthToken,
    query::{GetLibraryArtistsQuery, Query},
    YtMusic,
};

use crate::Commands;

enum QueryType {
    Source,
    Formatted,
}

fn command_to_query<A: AuthToken>(
    command: Commands,
    query_type: QueryType,
    yt: &YtMusic<A>,
) -> String {
    let q = match command {
        Commands::GetSearchSuggestions { query } => todo!(),
        Commands::GetArtist { channel_id } => todo!(),
        Commands::GetArtistAlbums {
            channel_id,
            browse_params,
        } => todo!(),
        Commands::GetLibraryPlaylists => todo!(),
        Commands::GetLibraryArtists => call_cmd(yt, GetLibraryArtistsQuery::default(), query_type),
        Commands::Search { query } => todo!(),
        Commands::SearchArtists { query } => todo!(),
        Commands::SearchAlbums { query } => todo!(),
        Commands::SearchSongs { query } => todo!(),
        Commands::SearchPlaylists { query } => todo!(),
        Commands::SearchCommunityPlaylists { query } => todo!(),
        Commands::SearchFeaturedPlaylists { query } => todo!(),
        Commands::SearchVideos { query } => todo!(),
        Commands::SearchEpisodes { query } => todo!(),
        Commands::SearchProfiles { query } => todo!(),
        Commands::SearchPodcasts { query } => todo!(),
        Commands::DeletePlaylist { playlist_id } => todo!(),
    };
    todo!()
}

async fn call_cmd<Q: Query, A: AuthToken>(yt: &YtMusic<A>, q: Q, query_type: QueryType) -> String {
    match query_type {
        QueryType::Source => yt.json_query(q).await.unwrap(),
        QueryType::Formatted => format!("{:#?}", yt.query(q).await.unwrap()),
    }
}
