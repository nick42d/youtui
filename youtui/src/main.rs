use anyhow::{bail, Context};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use cli::handle_cli_command;
use config::{ApiKey, AuthType, Config};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use ytmapi_rs::auth::OAuthToken;

mod api;
mod app;
mod appevent;
mod async_rodio_sink;
mod cli;
mod config;
mod core;
mod drawutils;
mod keyaction;
mod keybind;
#[cfg(test)]
mod tests;

pub const POTOKEN_FILENAME: &str = "po_token.txt";
pub const COOKIE_FILENAME: &str = "cookie.txt";
pub const OAUTH_FILENAME: &str = "oauth.json";
const DIRECTORY_NAME_ERROR_MESSAGE: &str = "Error generating application directory for your host system. See README.md for more information about application directories.";

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
    /// Generate shell completions for the specified shell
    #[arg(short, long, id = "SHELL", value_enum)]
    generate_completions: Option<Shell>,
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
    /// Process the passed Json file(s) as if received from YouTube Music. This
    /// parameter can be passed multiple times, processing multiple files if
    /// the endpoint supports continuations. If multiple files are
    /// passed but the endpoint doesn't support continuations, only the
    /// first one is processed.
    #[arg(short, long, id = "PATH")]
    input_json: Option<Vec<PathBuf>>,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand, Debug, Clone)]
enum AuthCmd {
    /// Generate an OAuth token.
    SetupOauth {
        /// Optional: Write to a specific file instead of the config directory.
        #[arg(short, long)]
        file_name: Option<PathBuf>,
        /// Optional: Print to stdout instead of the config directory.
        #[arg(short, long, default_value_t = false)]
        stdout: bool,
        /// Client ID - from Google Cloud Console
        client_id: String,
        /// Client Secret - from Google Cloud Console
        client_secret: String,
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
    GetLibraryPlaylists {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibraryArtists {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibrarySongs {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibraryAlbums {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibraryArtistSubscriptions {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibraryPodcasts {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    //TODO: Allow sorting
    GetLibraryChannels {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    Search {
        query: String,
    },
    SearchArtists {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchAlbums {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchSongs {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchPlaylists {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchCommunityPlaylists {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchFeaturedPlaylists {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchVideos {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchEpisodes {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchProfiles {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    SearchPodcasts {
        query: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
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
    GetLibraryUploadSongs {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    // TODO: Sorting
    GetLibraryUploadArtists {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    // TODO: Sorting
    GetLibraryUploadAlbums {
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
    },
    GetLibraryUploadArtist {
        upload_artist_id: String,
        /// Maximum number of pages that the API is allowed to return.
        #[arg(default_value_t = 1)]
        max_pages: usize,
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
    GetLyrics {
        lyrics_id: String,
    },
    // TODO: Option to use playlist ID instead
    GetWatchPlaylist {
        video_id: String,
    },
    GetChannel {
        channel_id: String,
    },
    GetChannelEpisodes {
        channel_id: String,
        podcast_channel_params: String,
    },
    GetPodcast {
        podcast_id: String,
    },
    GetEpisode {
        video_id: String,
    },
    GetNewEpisodes,
}

pub struct RuntimeInfo {
    debug: bool,
    config: Config,
    api_key: ApiKey,
    po_token: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Using try block to print error using Display instead of Debug.
    if let Err(e) = try_main().await {
        println!("{:?}", e);
        return ExitCode::FAILURE;
    };
    ExitCode::SUCCESS
}

// Main function is refactored here so that we can pretty print errors.
// Regular main function returns debug errors so not as friendly.
async fn try_main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let Arguments {
        debug,
        cli,
        auth_cmd,
        auth_type,
        generate_completions,
    } = args;
    // We don't need configuration to setup oauth token or generate completions.
    if let Some(c) = auth_cmd {
        match c {
            AuthCmd::SetupOauth {
                file_name,
                stdout,
                client_id,
                client_secret,
            } => {
                cli::get_and_output_oauth_token(file_name, stdout, client_id, client_secret).await?
            }
        };
        // Done here if we got this command. No need to go further.
        return Ok(());
    };
    // We don't need configuration to setup oauth token or generate completions.
    if let Some(shell) = generate_completions {
        let mut cmd = Arguments::command();
        let bin_name = cmd.get_name().to_string();
        eprintln!("Generating completion file for {shell:?}");
        generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        // Done here if we got this command. No need to go further.
        return Ok(());
    };
    // Config and API key files will be in OS directories.
    // Create them if they don't exist.
    initialise_directories().await?;
    let mut config = config::Config::new(debug).await?;
    // Command line flag for auth_type should override config for auth_type.
    if let Some(auth_type) = auth_type {
        config.auth_type = auth_type
    }
    // Once config has loaded, load API key to memory
    // (Which key to load depends on configuration)
    // TODO: api_key and po_token could be more lazily loaded.
    let api_key = load_api_key(&config).await?;
    // Use PoToken, if the user has supplied one (otherwise don't).
    let po_token = load_po_token().await.ok();
    let rt = RuntimeInfo {
        debug,
        config,
        api_key,
        po_token,
    };
    match cli.command {
        None => run_app(rt).await?,
        Some(_) => handle_cli_command(cli, rt).await?,
    };
    Ok(())
}

// XXX: Seems to be some duplication of load_api_key.
async fn get_api(config: &Config) -> anyhow::Result<api::DynamicYtMusic> {
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
        config::AuthType::Unauthenticated => {
            let api = ytmapi_rs::builder::YtMusicBuilder::new_rustls_tls()
                .build()
                .await?;
            api::DynamicYtMusic::NoAuth(api)
        }
    };
    Ok(api)
}

pub async fn run_app(rt: RuntimeInfo) -> anyhow::Result<()> {
    let mut app = app::Youtui::new(rt).await?;
    app.run().await?;
    Ok(())
}

pub fn get_data_dir() -> anyhow::Result<PathBuf> {
    // TODO: Document that directory can be set by environment variable.
    let directory = if let Ok(s) = std::env::var("YOUTUI_DATA_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        bail!(DIRECTORY_NAME_ERROR_MESSAGE);
    };
    Ok(directory)
}

pub fn get_config_dir() -> anyhow::Result<PathBuf> {
    // TODO: Document that directory can be set by environment variable.
    let directory = if let Ok(s) = std::env::var("YOUTUI_CONFIG_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        bail!(DIRECTORY_NAME_ERROR_MESSAGE);
    };
    Ok(directory)
}

async fn load_po_token() -> anyhow::Result<String> {
    let mut path = get_config_dir()?;
    path.push(POTOKEN_FILENAME);
    tokio::fs::read_to_string(&path)
        .await
        // Allocation is required here if we wish to trim within this function.
        .map(|s| s.trim().to_string())
        .with_context(|| {
            format!(
                "Error loading po_token from {}. Does the file exist?",
                path.display()
            )
        })
}

async fn load_cookie_file() -> anyhow::Result<String> {
    let mut path = get_config_dir()?;
    path.push(COOKIE_FILENAME);
    tokio::fs::read_to_string(&path)
        .await
        .with_context(|| auth_token_error_message(config::AuthType::Browser, &path))
}

async fn load_oauth_file() -> anyhow::Result<OAuthToken> {
    let mut path = get_config_dir()?;
    path.push(OAUTH_FILENAME);
    let file = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| auth_token_error_message(config::AuthType::OAuth, &path))?;
    serde_json::from_str(&file)
        .with_context(|| format!("Error parsing AuthType::OAuth auth token from {}. See README.md for more information on auth tokens.", path.display()))
}

/// Create the Config and Data directories for the app if they do not already
/// exist. Returns an error if unsuccesful.
async fn initialise_directories() -> anyhow::Result<()> {
    let config_dir = get_config_dir()?;
    let data_dir = get_data_dir()?;
    tokio::try_join!(
        tokio::fs::create_dir_all(config_dir),
        tokio::fs::create_dir_all(data_dir),
    )?;
    Ok(())
}

async fn load_api_key(cfg: &Config) -> anyhow::Result<ApiKey> {
    let api_key = match cfg.auth_type {
        config::AuthType::OAuth => ApiKey::OAuthToken(load_oauth_file().await?),
        config::AuthType::Browser => ApiKey::BrowserToken(load_cookie_file().await?),
        config::AuthType::Unauthenticated => ApiKey::None,
    };
    Ok(api_key)
}

fn auth_token_error_message(token_type: config::AuthType, path: &Path) -> String {
    format!("Error loading {:?} auth token from {}. Does the file exist? See README.md for more information on auth tokens.", token_type, path.display())
}
