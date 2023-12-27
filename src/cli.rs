use crate::config::Config;
use crate::get_api;
use crate::Cli;
use crate::Commands;
use crate::Result;
use crate::RuntimeInfo;
use std::path::PathBuf;
use ytmapi_rs::query::AlbumsFilter;
use ytmapi_rs::query::ArtistsFilter;
use ytmapi_rs::query::GetLibraryArtistsQuery;
use ytmapi_rs::query::GetLibraryPlaylistsQuery;
use ytmapi_rs::query::SearchQuery;
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
            command: Some(Commands::GetLibraryArtists),
            show_source: true,
        } => print_library_artists_json(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryArtists),
            show_source: false,
        } => print_library_artists(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryPlaylists),
            show_source: true,
        } => print_library_playlists_json(&config).await?,
        Cli {
            command: Some(Commands::GetLibraryPlaylists),
            show_source: false,
        } => print_library_playlists(&config).await?,
        Cli {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: false,
        } => print_search_suggestions(&config, query).await?,
        Cli {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: true,
        } => print_search_suggestions_json(&config, query).await?,
        Cli {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: false,
        } => print_artist(&config, channel_id).await?,
        Cli {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: true,
        } => print_artist_json(&config, channel_id).await?,
        Cli {
            command: Some(Commands::SearchArtists { query }),
            show_source: false,
        } => search_artists(&config, query).await?,
        Cli {
            command: Some(Commands::SearchArtists { query }),
            show_source: true,
        } => search_artists_json(&config, query).await?,
        Cli {
            command: Some(Commands::SearchAlbums { query }),
            show_source: false,
        } => search_albums(&config, query).await?,
        Cli {
            command: Some(Commands::SearchAlbums { query }),
            show_source: true,
        } => search_albums_json(&config, query).await?,
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
