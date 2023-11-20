// Utilising nightly until async trait stabilised
#![feature(async_fn_in_trait)]

mod app;
mod appevent;
mod cli;
mod config;
mod core;
mod drawutils;
pub mod error;

use cli::{
    get_and_output_oauth_token, handle_cli_command, print_artist, print_artist_json,
    print_library_artists, print_library_artists_json, print_library_playlists,
    print_library_playlists_json, print_search_suggestions, print_search_suggestions_json,
};
use config::{ApiKey, Config};
pub use error::Result;

use clap::{Args, Parser, Subcommand};
use directories::ProjectDirs;
use error::Error;
use std::path::{Path, PathBuf};

pub const HEADER_FILENAME: &str = "headers.txt";
pub const OAUTH_FILENAME: &str = "oauth.json";

#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
/// A text-based user interface for YouTube Music.
struct Arguments {
    /// Display and log additional debug information.
    #[arg(short, long, default_value_t = false)]
    debug: bool,
    // What happens if given both cli and auth_cmd?
    #[command(flatten)]
    cli: Cli,
    #[command(subcommand)]
    auth_cmd: Option<AuthCmd>,
}

#[derive(Args, Debug, Clone)]
// Probably shouldn't be public
pub struct Cli {
    /// Print the source output Json from YouTube Music's API instead of the processed value.
    #[arg(short, long, default_value_t = false)]
    show_source: bool,
    #[command(subcommand)]
    command: Option<Commands>,
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
enum Commands {
    GetSearchSuggestions { query: String },
    GetArtist { channel_id: String },
    GetLibraryPlaylists,
    GetLibraryArtists, //TODO: Allow sorting
    Search { query: String },
}

pub struct RuntimeInfo {
    debug: bool,
    config: Config,
    api_key: ApiKey,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    // Config and API key files will be in OS directories.
    // Create them if they don't exist.
    initialise_directories().await?;
    let config = config::Config::new().unwrap();
    // Once config has loaded, load API key to memory
    // (Which key to load depends on configuration)
    // XXX: check that this won't cause any delays.
    let api_key = load_api_key(&config).await?;
    let Arguments {
        debug,
        cli,
        auth_cmd,
    } = args;
    let rt = RuntimeInfo {
        debug,
        config,
        api_key,
    };
    if let Some(c) = auth_cmd {
        match c {
            AuthCmd::SetupOauth { file_name } => cli::get_and_output_oauth_token(file_name).await?,
        };
        // Done here if we got this command. No need to go further.
        return Ok(());
    };
    match cli.command {
        None => run_app(rt).await?,
        Some(_) => handle_cli_command(cli, rt).await?,
    }
    Ok(())
}

async fn get_api() -> ytmapi_rs::YtMusic {
    // TODO: remove unwrap
    let confdir = get_config_dir().unwrap();
    let mut headers_loc = PathBuf::from(confdir);
    headers_loc.push(HEADER_FILENAME);
    // TODO: remove unwrap
    ytmapi_rs::YtMusic::from_header_file(headers_loc)
        .await
        .unwrap()
}

pub async fn run_app(rt: RuntimeInfo) -> Result<()> {
    let mut app = app::Youtui::new(rt)?;
    app.run().await;
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

async fn load_header_file() -> Result<String> {
    let mut path = get_config_dir()?;
    path.push(HEADER_FILENAME);
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::new_auth_token_error(config::AuthType::Browser, path, e))
}

async fn load_oauth_file() -> Result<String> {
    let mut path = get_config_dir()?;
    path.push(OAUTH_FILENAME);
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::new_auth_token_error(config::AuthType::OAuth, path, e))
}

/// Create the Config and Data directories for the app if they do not already exist.
/// Returns an error if unsuccesful.
async fn initialise_directories() -> Result<()> {
    let config_dir = get_config_dir()?;
    let data_dir = get_data_dir()?;
    tokio::fs::create_dir_all(config_dir).await?;
    tokio::fs::create_dir_all(data_dir).await?;
    Ok(())
}

async fn load_api_key(cfg: &Config) -> Result<ApiKey> {
    // TODO: Better error hanadling
    let api_key = match cfg.get_auth_type() {
        config::AuthType::OAuth => ApiKey::new(load_oauth_file().await?, config::AuthType::OAuth),
        config::AuthType::Browser => {
            ApiKey::new(load_header_file().await?, config::AuthType::Browser)
        }
    };
    Ok(api_key)
}
