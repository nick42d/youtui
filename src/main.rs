mod app;
mod appevent;
mod cli;
mod config;
mod core;
mod drawutils;
pub mod error;

use clap::{Args, Parser, Subcommand};
use cli::handle_cli_command;
use config::{ApiKey, Config};
use directories::ProjectDirs;
use error::Error;
pub use error::Result;
use std::path::PathBuf;
use ytmapi_rs::auth::OAuthToken;

pub const COOKIE_FILENAME: &str = "cookie.txt";
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
    _debug: bool,
    config: Config,
    api_key: ApiKey,
}

#[tokio::main]
async fn main() {
    // Using try block to print error using Display instead of Debug.
    if let Err(e) = try_main().await {
        println!("{e}");
        // XXX: Return error code?
    };
}

// Main function is refactored here so that we can pretty print errors.
// Regular main function returns debug errors so not as friendly.
async fn try_main() -> Result<()> {
    let args = Arguments::parse();
    let Arguments {
        debug,
        cli,
        auth_cmd,
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
    let config = config::Config::new()?;
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

async fn get_api(config: &Config) -> Result<ytmapi_rs::YtMusic> {
    let confdir = get_config_dir()?;
    let api = match config.get_auth_type() {
        config::AuthType::OAuth => {
            let mut oauth_loc = PathBuf::from(confdir);
            oauth_loc.push(OAUTH_FILENAME);
            let file = tokio::fs::read_to_string(oauth_loc).await?;
            let oath_tok = serde_json::from_str(&file)?;
            ytmapi_rs::YtMusic::from_oauth_token(oath_tok)
        }
        config::AuthType::Browser => {
            let mut cookies_loc = PathBuf::from(confdir);
            cookies_loc.push(COOKIE_FILENAME);
            ytmapi_rs::YtMusic::from_cookie_file(cookies_loc).await?
        }
    };
    Ok(api)
}

pub async fn run_app(rt: RuntimeInfo) -> Result<()> {
    // Oauth is not yet supported in the app due to needing to refresh the tokens.
    // So we'll error in that case for now.
    // TODO: Implement OAuth in the app.
    match &rt.api_key {
        ApiKey::OAuthToken(_) => return Err(Error::OAuthNotYetSupportedByApp),
        ApiKey::BrowserToken(_) => (),
    };
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
    let file = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::new_auth_token_error(config::AuthType::Browser, path, e));
    file
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
        config::AuthType::OAuth => ApiKey::OAuthToken(load_oauth_file().await?),
        config::AuthType::Browser => ApiKey::BrowserToken(load_cookie_file().await?),
    };
    Ok(api_key)
}
