use crate::config::Config;
use crate::get_api;
use crate::Cli;
use crate::Commands;
use crate::Result;
use crate::RuntimeInfo;
use std::path::PathBuf;
use ytmapi_rs::common::BrowseParams;
use ytmapi_rs::common::PlaylistID;
use ytmapi_rs::parse::ProcessedResult;
use ytmapi_rs::query::AlbumsFilter;
use ytmapi_rs::query::ArtistsFilter;
use ytmapi_rs::query::CommunityPlaylistsFilter;
use ytmapi_rs::query::DeletePlaylistQuery;
use ytmapi_rs::query::EpisodesFilter;
use ytmapi_rs::query::FeaturedPlaylistsFilter;
use ytmapi_rs::query::GetArtistAlbumsQuery;
use ytmapi_rs::query::GetLibraryArtistsQuery;
use ytmapi_rs::query::GetLibraryPlaylistsQuery;
use ytmapi_rs::query::PlaylistsFilter;
use ytmapi_rs::query::PodcastsFilter;
use ytmapi_rs::query::ProfilesFilter;
use ytmapi_rs::query::Query;
use ytmapi_rs::query::SearchQuery;
use ytmapi_rs::query::SongsFilter;
use ytmapi_rs::query::VideosFilter;
use ytmapi_rs::{
    common::YoutubeID,
    generate_oauth_code_and_url, generate_oauth_token,
    query::{GetArtistQuery, GetSearchSuggestionsQuery},
    ChannelID,
};

pub async fn handle_cli_command(cli: Cli, rt: RuntimeInfo) -> Result<()> {
    let config = rt.config;
    match cli {
        // TODO: Block this action using type system.
        Cli { command: None, .. } => println!("Show source requires an associated API command"),
        Cli {
            input_json: Some(path),
            command:
                Some(Commands::GetArtistAlbums {
                    channel_id,
                    browse_params,
                }),
            ..
        } => {
            let file = tokio::fs::read_to_string(path).await?;
            let res = ProcessedResult::from_string(
                file,
                GetArtistAlbumsQuery::new(
                    ChannelID::from_raw(channel_id),
                    BrowseParams::from_raw(browse_params),
                ),
            )
            .parse()?;
            println!("{:#?}", res);
        }
        Cli {
            input_json: Some(path),
            ..
        } => todo!(),
        Cli {
            command: Some(Commands::GetLibraryArtists),
            show_source: true,
            ..
        } => print_library_artists_json(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryArtists),
            show_source: false,
            ..
        } => print_library_artists(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryPlaylists),
            show_source: true,
            ..
        } => print_library_playlists_json(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryPlaylists),
            show_source: false,
            ..
        } => print_library_playlists(&config).await?,
        Cli {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: false,
            ..
        } => print_search_suggestions(&config, query).await?,
        Cli {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: true,
            ..
        } => print_search_suggestions_json(&config, query).await?,
        Cli {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: false,
            ..
        } => print_artist(&config, channel_id).await?,
        Cli {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: true,
            ..
        } => print_artist_json(&config, channel_id).await?,
        Cli {
            command: Some(Commands::SearchArtists { query }),
            show_source: false,
            ..
        } => search_artists(&config, query).await?,
        Cli {
            command: Some(Commands::SearchArtists { query }),
            show_source: true,
            ..
        } => search_artists_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchAlbums { query }),
            show_source: false,
            ..
        } => search_albums(&config, query).await?,
        Cli {
            command: Some(Commands::SearchAlbums { query }),
            show_source: true,
            ..
        } => search_albums_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchSongs { query }),
            show_source: false,
            ..
        } => search_songs(&config, query).await?,
        Cli {
            command: Some(Commands::SearchSongs { query }),
            show_source: true,
            ..
        } => search_songs_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchPlaylists { query }),
            show_source: false,
            ..
        } => search_playlists(&config, query).await?,
        Cli {
            command: Some(Commands::SearchPlaylists { query }),
            show_source: true,
            ..
        } => search_playlists_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchEpisodes { query }),
            show_source: false,
            ..
        } => search_episodes(&config, query).await?,
        Cli {
            command: Some(Commands::SearchEpisodes { query }),
            show_source: true,
            ..
        } => search_episodes_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchPodcasts { query }),
            show_source: false,
            ..
        } => search_podcasts(&config, query).await?,
        Cli {
            command: Some(Commands::SearchPodcasts { query }),
            show_source: true,
            ..
        } => search_podcasts_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchCommunityPlaylists { query }),
            show_source: false,
            ..
        } => search_community_playlists(&config, query).await?,
        Cli {
            command: Some(Commands::SearchCommunityPlaylists { query }),
            show_source: true,
            ..
        } => search_community_playlists_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchFeaturedPlaylists { query }),
            show_source: false,
            ..
        } => search_featured_playlists(&config, query).await?,
        Cli {
            command: Some(Commands::SearchFeaturedPlaylists { query }),
            show_source: true,
            ..
        } => search_featured_playlists_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchProfiles { query }),
            show_source: false,
            ..
        } => search_profiles(&config, query).await?,
        Cli {
            command: Some(Commands::SearchProfiles { query }),
            show_source: true,
            ..
        } => search_profiles_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchVideos { query }),
            show_source: false,
            ..
        } => search_videos(&config, query).await?,
        Cli {
            command: Some(Commands::SearchVideos { query }),
            show_source: true,
            ..
        } => search_videos_json(&config, query).await?,
        Cli {
            command: Some(Commands::Search { query }),
            show_source: false,
            ..
        } => search(&config, query).await?,
        Cli {
            command: Some(Commands::Search { query }),
            show_source: true,
            ..
        } => search_json(&config, query).await?,
        Cli {
            command: Some(Commands::DeletePlaylist { playlist_id }),
            show_source: true,
            ..
        } => unimplemented!(),
        Cli {
            command: Some(Commands::DeletePlaylist { playlist_id }),
            show_source: false,
            ..
        } => delete_json(&config, playlist_id).await?,
        _ => todo!(),
    }
    Ok(())
}
pub async fn get_and_output_oauth_token(file_name: Option<PathBuf>) -> Result<()> {
    let token_str = get_oauth_token().await?;
    if let Some(file_name) = file_name {
        tokio::fs::write(&file_name, token_str).await?;
        println!("Wrote Oauth token to {}", file_name.display());
    } else {
        println!("{token_str}");
    }
    Ok(())
}
async fn get_oauth_token() -> Result<String> {
    let (code, url) = generate_oauth_code_and_url().await?;
    // Hack to wait for input
    println!("Go to {url}, finish the login flow, and press enter when done");
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
    let token = generate_oauth_token(code).await?;
    Ok(serde_json::to_string_pretty(&token)?)
}

pub async fn print_artist(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config)
        .await?
        .get_artist(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await?;
    println!("{:#?}", res);
    Ok(())
}

pub async fn print_artist_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

pub async fn print_search_suggestions(config: &Config, query: String) -> Result<()> {
    // TODO: remove unwrap
    let res = get_api(&config)
        .await?
        .get_search_suggestions(query)
        .await?;
    println!("{:#?}", res);
    Ok(())
}

pub async fn print_search_suggestions_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(GetSearchSuggestionsQuery::from(query))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

pub async fn print_library_playlists(config: &Config) -> Result<()> {
    let res = get_api(&config).await?.get_library_playlists().await?;
    println!("{:#?}", res);
    Ok(())
}

pub async fn print_library_playlists_json(config: &Config) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(GetLibraryPlaylistsQuery)
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_artists(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_artists(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_artists_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(ArtistsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_albums(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_albums(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_albums_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(AlbumsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_songs(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_songs(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_songs_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(SongsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_playlists(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_playlists(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_playlists_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(PlaylistsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_featured_playlists(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config)
        .await?
        .search_featured_playlists(query)
        .await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_featured_playlists_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(FeaturedPlaylistsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_community_playlists(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config)
        .await?
        .search_community_playlists(query)
        .await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_community_playlists_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(CommunityPlaylistsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_episodes(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_episodes(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_episodes_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(EpisodesFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_podcasts(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_podcasts(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_podcasts_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(PodcastsFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn search_profiles(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_profiles(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_profiles_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(ProfilesFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
pub async fn delete_json(config: &Config, playlist_id: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(DeletePlaylistQuery::new(PlaylistID::from_raw(playlist_id)))
        .await?;
    println!("{}", json);
    Ok(())
}
pub async fn search_videos(config: &Config, query: String) -> Result<()> {
    let res = get_api(&config).await?.search_videos(query).await?;
    println!("{:#?}", res);
    Ok(())
}
pub async fn search_videos_json(config: &Config, query: String) -> Result<()> {
    let json = get_api(&config)
        .await?
        .json_query(SearchQuery::new(query).with_filter(VideosFilter))
        .await?;
    let json: serde_json::Value = serde_json::from_str(json.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

pub async fn print_library_artists(config: &Config) -> Result<()> {
    // TODO: Allow sorting
    let res = get_api(&config)
        .await?
        .get_library_artists(GetLibraryArtistsQuery::default())
        .await?;
    println!("{:#?}", res);
    Ok(())
}

pub async fn print_library_artists_json(config: &Config) -> Result<()> {
    // TODO: Allow sorting
    let json = get_api(&config)
        .await?
        .json_query(GetLibraryArtistsQuery::default())
        .await?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
