use ytmapi_rs::query::Query;

use crate::Commands;

fn command_to_query(command: Commands) -> Box<dyn Query> {
    let q = match command {
        Commands::GetSearchSuggestions { query } => todo!(),
        Commands::GetArtist { channel_id } => todo!(),
        Commands::GetArtistAlbums {
            channel_id,
            browse_params,
        } => todo!(),
        Commands::GetLibraryPlaylists => todo!(),
        Commands::GetLibraryArtists => todo!(),
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
    Box::new(q);
}
