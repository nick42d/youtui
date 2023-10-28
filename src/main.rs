// Utilising nightly until async trait stabilised
#![feature(async_fn_in_trait)]

mod app;
mod appevent;
mod core;

pub mod error;

pub use error::Result;

use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use error::Error;
use std::path::PathBuf;
use ytmapi_rs::{
    common::YoutubeID,
    query::{GetArtistQuery, GetSearchSuggestionsQuery},
    ChannelID, YtMusic,
};

pub const HEADER_FILENAME: &str = "headers.txt";

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
    // This does not work with the show_source command!
    SetupOAuth,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
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
            command: Some(Commands::SetupOAuth),
            show_source: _,
            ..
        } => setup_oauth().await,
    }
    Ok(())
}

async fn setup_oauth() {
    let api = YtMusic::default();
    let (code, url) = api.generate_oauth_code_and_url().await.unwrap();
    // Hack to wait for input
    // TODO: Remove unwraps
    println!("Go to {url}, finish the login flow, and press enter when done");
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
    let token = api.generate_oauth_token(code).await.unwrap();
    println!("{:?}", token);
}

async fn print_artist(query: String) {
    // TODO: remove unwrap
    let res = get_api()
        .await
        .get_artist(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await
        .unwrap();
    println!("{:#?}", res)
}

async fn print_artist_json(query: String) {
    // TODO: remove unwrap
    let json = get_api()
        .await
        .json_query(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await
        .unwrap();
    // TODO: remove unwrap
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

async fn print_search_suggestions(query: String) {
    // TODO: remove unwrap
    let res = get_api().await.get_search_suggestions(query).await.unwrap();
    println!("{:#?}", res)
}

async fn print_search_suggestions_json(query: String) {
    // TODO: remove unwrap
    let json = get_api()
        .await
        .json_query(GetSearchSuggestionsQuery::from(query))
        .await
        .unwrap();
    // TODO: remove unwrap
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
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
    let directory = if let Ok(s) = std::env::var("YOUTUI_DATA_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNotFound);
    };
    Ok(directory)
}

pub fn get_config_dir() -> Result<PathBuf> {
    let directory = if let Ok(s) = std::env::var("YOUTUI_CONFIG_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNotFound);
    };
    Ok(directory)
}
