use clap::{Args, Parser, Subcommand};
use cli::handle_cli_command;
use config::{ApiKey, AuthType, Config};
use directories::ProjectDirs;
use error::Error;
pub use error::Result;
use std::{path::PathBuf, process::ExitCode};
use ytmapi_rs::auth::{BrowserToken, OAuthToken};

mod api;
mod app;
mod appevent;
mod cli;
mod config;
mod core;
mod drawutils;
mod error;

pub const COOKIE_FILENAME: &str = "cookie.txt";
pub const OAUTH_FILENAME: &str = "oauth.json";

#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
/// A text-based user interface for YouTube Music.
struct Arguments {
    /// Display and log additional debug information.
    #[arg(short, long, default_value_t = false)]
    debug: bool,
    #[command(flatten)]
    cli: Cli,
    #[command(subcommand)]
    auth_cmd: Option<AuthCmd>,
    /// Force the use of an auth type.
    #[arg(value_enum, short, long)]
    auth_type: Option<AuthType>,
}

#[derive(Args, Debug, Clone)]
struct Cli {
    /// Print the source output Json from YouTube Music's API instead of the
    /// processed value.
    #[arg(short, long, default_value_t = false)]
    show_source: bool,
    /// Process the passed Json file as if it were received from YouTube Music.
    #[arg(short, long)]
    input_json: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand, Debug, Clone)]
enum AuthCmd {
    /// Generate an OAuth token.
    SetupOauth {
        /// Optional: Write to a file.
        file_name: Option<PathBuf>,
    },
}
#[derive(Subcommand, Debug, Clone)]
enum Command {
    GetSearchSuggestions {
        query: String,
    },
    GetArtist {
        channel_id: String,
    },
    GetArtistAlbums {
        channel_id: String,
        browse_params: String,
    },
    GetAlbum {
        browse_id: String,
    },
    GetPlaylist {
        playlist_id: String,
    },
    GetLibraryPlaylists,
    GetLibraryArtists,             //TODO: Allow sorting
    GetLibrarySongs,               //TODO: Allow sorting
    GetLibraryAlbums,              //TODO: Allow sorting
    GetLibraryArtistSubscriptions, //TODO: Allow sorting
    Search {
        query: String,
    },
    SearchArtists {
        query: String,
    },
    SearchAlbums {
        query: String,
    },
    SearchSongs {
        query: String,
    },
    SearchPlaylists {
        query: String,
    },
    SearchCommunityPlaylists {
        query: String,
    },
    SearchFeaturedPlaylists {
        query: String,
    },
    SearchVideos {
        query: String,
    },
    SearchEpisodes {
        query: String,
    },
    SearchProfiles {
        query: String,
    },
    SearchPodcasts {
        query: String,
    },
    // TODO: Privacy status, video ids, source playlist
    CreatePlaylist {
        title: String,
        description: Option<String>,
    },
    DeletePlaylist {
        playlist_id: String,
    },
    RemovePlaylistItems {
        playlist_id: String,
        video_ids: Vec<String>,
    },
    AddVideosToPlaylist {
        playlist_id: String,
        video_ids: Vec<String>,
    },
    AddPlaylistToPlaylist {
        playlist_id: String,
        from_playlist_id: String,
    },
    EditPlaylistTitle {
        playlist_id: String,
        new_title: String,
    },
    GetHistory,
    RemoveHistoryItems {
        feedback_tokens: Vec<String>,
    },
    RateSong {
        video_id: String,
        like_status: String,
    },
    RatePlaylist {
        playlist_id: String,
        like_status: String,
    },
    EditSongLibraryStatus {
        feedback_tokens: Vec<String>,
    },
    // TODO: Sorting
    GetLibraryUploadSongs,
    // TODO: Sorting
    GetLibraryUploadArtists,
    // TODO: Sorting
    GetLibraryUploadAlbums,
    GetLibraryUploadArtist {
        upload_artist_id: String,
    },
    GetLibraryUploadAlbum {
        upload_album_id: String,
    },
    DeleteUploadEntity {
        upload_entity_id: String,
    },
    GetTasteProfile,
    // Simple implementation - only allows a single set per command.
    SetTasteProfile {
        impression_token: String,
        selection_token: String,
    },
    GetMoodCategories,
    GetMoodPlaylists {
        mood_category_params: String,
    },
    AddHistoryItem {
        song_tracking_url: String,
    },
    GetSongTrackingUrl {
        video_id: String,
    },
}

pub struct RuntimeInfo {
    _debug: bool,
    config: Config,
    api_key: ApiKey,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Using try block to print error using Display instead of Debug.
    if let Err(e) = try_main().await {
        println!("{e}");
        return ExitCode::FAILURE;
    };
    ExitCode::SUCCESS
}

// Main function is refactored here so that we can pretty print errors.
// Regular main function returns debug errors so not as friendly.
async fn try_main() -> Result<()> {
    let args = Arguments::parse();
    let Arguments {
        debug,
        cli,
        auth_cmd,
        auth_type,
    } = args;
    // We don't need configuration to setup oauth token.
    if let Some(c) = auth_cmd {
        match c {
            AuthCmd::SetupOauth { file_name } => cli::get_and_output_oauth_token(file_name).await?,
        };
        // Done here if we got this command. No need to go further.
        return Ok(());
    };
    // Config and API key files will be in OS directories.
    // Create them if they don't exist.
    initialise_directories().await?;
    let mut config = config::Config::new()?;
    // Command line flag for auth_type should override config for auth_type.
    if let Some(auth_type) = auth_type {
        config.auth_type = auth_type
    }
    // Once config has loaded, load API key to memory
    // (Which key to load depends on configuration)
    // XXX: check that this won't cause any delays.
    // TODO: Remove delay, should be handled inside app instead.
    let api_key = load_api_key(&config).await?;
    let rt = RuntimeInfo {
        _debug: debug,
        config,
        api_key,
    };
    match cli.command {
        None => run_app(rt).await?,
        Some(_) => handle_cli_command(cli, rt).await?,
    };
    Ok(())
}

// XXX: Seems to be some duplication of load_api_key.
async fn get_api(config: &Config) -> Result<api::DynamicYtMusic> {
    let confdir = get_config_dir()?;
    let api = match config.auth_type {
        config::AuthType::OAuth => {
            let mut oauth_loc = confdir;
            oauth_loc.push(OAUTH_FILENAME);
            let file = tokio::fs::read_to_string(oauth_loc).await?;
            let oath_tok = serde_json::from_str(&file)?;
            let mut api = ytmapi_rs::builder::YtMusicBuilder::new_rustls_tls()
                .with_oauth_token(oath_tok)
                .build()?;
            // For simplicity for now - refresh OAuth token every time.
            api.refresh_token().await?;
            api::DynamicYtMusic::OAuth(api)
        }
        config::AuthType::Browser => {
            let mut cookies_loc = confdir;
            cookies_loc.push(COOKIE_FILENAME);
            let api = ytmapi_rs::builder::YtMusicBuilder::new_rustls_tls()
                .with_browser_token_cookie_file(cookies_loc)
                .build()
                .await?;
            api::DynamicYtMusic::Browser(api)
        }
    };
    Ok(api)
}

pub async fn run_app(rt: RuntimeInfo) -> Result<()> {
    let mut app = app::Youtui::new(rt)?;
    app.run().await?;
    Ok(())
}

pub fn get_data_dir() -> Result<PathBuf> {
    // TODO: Document that directory can be set by environment variable.
    let directory = if let Ok(s) = std::env::var("YOUTUI_DATA_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNameError);
    };
    Ok(directory)
}

pub fn get_config_dir() -> Result<PathBuf> {
    // TODO: Document that directory can be set by environment variable.
    let directory = if let Ok(s) = std::env::var("YOUTUI_CONFIG_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNameError);
    };
    Ok(directory)
}

async fn load_cookie_file() -> Result<String> {
    let mut path = get_config_dir()?;
    path.push(COOKIE_FILENAME);
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::new_auth_token_error(config::AuthType::Browser, path, e))
}

async fn load_oauth_file() -> Result<OAuthToken> {
    let mut path = get_config_dir()?;
    path.push(OAUTH_FILENAME);
    let file = tokio::fs::read_to_string(&path)
        .await
        // TODO: Remove clone
        .map_err(|e| Error::new_auth_token_error(config::AuthType::OAuth, path.clone(), e))?;
    serde_json::from_str(&file)
        .map_err(|_| Error::new_auth_token_parse_error(config::AuthType::OAuth, path))
}

/// Create the Config and Data directories for the app if they do not already
/// exist. Returns an error if unsuccesful.
async fn initialise_directories() -> Result<()> {
    let config_dir = get_config_dir()?;
    let data_dir = get_data_dir()?;
    tokio::fs::create_dir_all(config_dir).await?;
    tokio::fs::create_dir_all(data_dir).await?;
    Ok(())
}

async fn load_api_key(cfg: &Config) -> Result<ApiKey> {
    let api_key = match cfg.auth_type {
        config::AuthType::OAuth => ApiKey::OAuthToken(load_oauth_file().await?),
        config::AuthType::Browser => ApiKey::BrowserToken(load_cookie_file().await?),
    };
    Ok(api_key)
}
