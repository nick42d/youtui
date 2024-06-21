use crate::config::Config;
use crate::get_api;
use crate::Cli;
use crate::Command;
use crate::Result;
use crate::RuntimeInfo;
use querybuilder::command_to_query;
use querybuilder::CliQuery;
use querybuilder::QueryType;
use std::path::PathBuf;
use ytmapi_rs::common::PlaylistID;
use ytmapi_rs::query::AlbumsFilter;
use ytmapi_rs::query::ArtistsFilter;
use ytmapi_rs::query::CommunityPlaylistsFilter;
use ytmapi_rs::query::DeletePlaylistQuery;
use ytmapi_rs::query::EpisodesFilter;
use ytmapi_rs::query::FeaturedPlaylistsFilter;
use ytmapi_rs::query::GetLibraryArtistsQuery;
use ytmapi_rs::query::GetLibraryPlaylistsQuery;
use ytmapi_rs::query::PlaylistsFilter;
use ytmapi_rs::query::PodcastsFilter;
use ytmapi_rs::query::ProfilesFilter;
use ytmapi_rs::query::SearchQuery;
use ytmapi_rs::query::SongsFilter;
use ytmapi_rs::query::VideosFilter;
use ytmapi_rs::{
    common::YoutubeID,
    generate_oauth_code_and_url, generate_oauth_token,
    query::{GetArtistQuery, GetSearchSuggestionsQuery},
    ChannelID,
};

mod querybuilder;

pub async fn handle_cli_command(cli: Cli, rt: RuntimeInfo) -> Result<()> {
    let config = rt.config;
    match cli {
        // TODO: Block this action using type system.
        Cli {
            command: None,
            show_source: true,
            ..
        } => println!("Show source requires an associated API command"),
        Cli {
            command: None,
            input_json: Some(_),
            ..
        } => println!("API command must be provided when providing an input json file"),
        Cli {
            command: None,
            input_json: None,
            show_source: false,
        } => println!("No command provided"),
        Cli {
            command: Some(command),
            input_json: Some(input_json),
            show_source,
        } => {
            let source = tokio::fs::read_to_string(input_json).await?;
            let cli_query = CliQuery {
                query_type: QueryType::FromSourceFile(source),
                show_source,
            };
            let api = get_api(&config).await?;
            let res = command_to_query(command, cli_query, &api).await?;
            println!("{:#?}", res);
        }
        Cli {
            command: Some(command),
            input_json: None,
            show_source,
        } => {
            let cli_query = CliQuery {
                query_type: QueryType::FromApi,
                show_source,
            };
            let api = get_api(&config).await?;
            let res = command_to_query(command, cli_query, &api).await?;
            println!("{:#?}", res);
        }
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
