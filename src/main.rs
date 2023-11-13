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
    get_and_output_oauth_token, print_artist, print_artist_json, print_search_suggestions,
    print_search_suggestions_json,
};
use config::{ApiKey, Config};
pub use error::Result;

use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use error::Error;
use std::path::PathBuf;

pub const HEADER_FILENAME: &str = "headers.txt";
pub const OAUTH_FILENAME: &str = "oauth.json";

#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
/// A text-based user interface for YouTube Music.
struct Arguments {
    // Unsure how to represent that these two values are mutually exlucsive
    #[arg(short, long, default_value_t = false)]
    debug: bool,
    #[arg(short, long, default_value_t = false)]
    show_source: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    GetSearchSuggestions { query: String },
    GetArtist { channel_id: String },
    // This does not work well with the show_source command!
    SetupOAuth { file_name: Option<PathBuf> },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    initialise_directories()?;
    // Not implementing config just yet
    // let cfg = config::Config::new().unwrap();
    // Once config has loaded, load API key to memory
    // let api_key = load_api_key(&cfg);
    // TODO: Error handling
    match args {
        Arguments {
            command: None,
            debug: false,
            ..
        } => run_app().await?,
        Arguments {
            command: None,
            debug: true,
            ..
        } => todo!(),
        Arguments {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: false,
            ..
        } => print_search_suggestions(query).await,
        Arguments {
            command: Some(Commands::GetSearchSuggestions { query }),
            show_source: true,
            ..
        } => print_search_suggestions_json(query).await,
        Arguments {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: false,
            ..
        } => print_artist(channel_id).await,
        Arguments {
            command: Some(Commands::GetArtist { channel_id }),
            show_source: true,
            ..
        } => print_artist_json(channel_id).await,
        Arguments {
            command: Some(Commands::SetupOAuth { file_name }),
            show_source: _,
            ..
        } => get_and_output_oauth_token(file_name).await,
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

pub async fn run_app() -> Result<()> {
    let mut app = app::Youtui::new()?;
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

pub fn load_header_file() -> Result<String> {
    todo!()
}

pub fn load_oauth_file() -> Result<String> {
    todo!()
}

/// Create the Config and Data directories for the app if they do not already exist.
/// Returns an error if unsuccesful.
fn initialise_directories() -> Result<()> {
    let config_dir = get_config_dir()?;
    let data_dir = get_data_dir()?;
    std::fs::create_dir_all(config_dir)?;
    std::fs::create_dir_all(data_dir)?;
    Ok(())
}

fn load_api_key(cfg: &Config) -> Result<ApiKey> {
    let api_key = match cfg.get_auth_type() {
        config::AuthType::OAuth => ApiKey::new(load_oauth_file()?, cfg.get_auth_type()),
        config::AuthType::Browser => ApiKey::new(load_header_file()?, cfg.get_auth_type()),
    };
    Ok(api_key)
}
