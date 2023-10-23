use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ytmapi_rs::{
    common::YoutubeID,
    query::{GetArtistQuery, GetSearchSuggestionsQuery},
    ChannelID,
};

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
}

#[tokio::main]
async fn main() -> youtui::Result<()> {
    let args = Arguments::parse();
    match args {
        Arguments {
            command: None,
            debug: false,
            ..
        } => youtui::run_app().await?,
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
    }
    Ok(())
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
    let confdir = youtui::get_config_dir().unwrap();
    let mut headers_loc = PathBuf::from(confdir);
    headers_loc.push(youtui::HEADER_FILENAME);
    // TODO: remove unwrap
    ytmapi_rs::YtMusic::from_header_file(headers_loc)
        .await
        .unwrap()
}
