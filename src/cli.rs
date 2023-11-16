use std::path::PathBuf;

use ytmapi_rs::{
    common::YoutubeID,
    generate_oauth_code_and_url, generate_oauth_token,
    query::{GetArtistQuery, GetSearchSuggestionsQuery},
    ChannelID,
};

use crate::get_api;

pub async fn get_and_output_oauth_token(file_name: Option<PathBuf>) {
    let token_str = get_oauth_token().await;
    if let Some(file_name) = file_name {
        tokio::fs::write(&file_name, token_str).await.unwrap();
        println!("Wrote Oauth token to {}", file_name.display());
    } else {
        println!("{token_str}");
    }
}
async fn get_oauth_token() -> String {
    let (code, url) = generate_oauth_code_and_url().await.unwrap();
    // Hack to wait for input
    // TODO: Remove unwraps
    println!("Go to {url}, finish the login flow, and press enter when done");
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
    let token = generate_oauth_token(code).await.unwrap();
    serde_json::to_string_pretty(&token).unwrap()
}

pub async fn print_artist(query: String) {
    // TODO: remove unwrap
    let res = get_api()
        .await
        .get_artist(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await
        .unwrap();
    println!("{:#?}", res)
}

pub async fn print_artist_json(query: String) {
    // TODO: remove unwrap
    let json = get_api()
        .await
        .json_query(GetArtistQuery::new(ChannelID::from_raw(query)))
        .await
        .unwrap();
    // TODO: remove unwrap
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

pub async fn print_search_suggestions(query: String) {
    // TODO: remove unwrap
    let res = get_api().await.get_search_suggestions(query).await.unwrap();
    println!("{:#?}", res)
}

pub async fn print_search_suggestions_json(query: String) {
    // TODO: remove unwrap
    let json = get_api()
        .await
        .json_query(GetSearchSuggestionsQuery::from(query))
        .await
        .unwrap();
    // TODO: remove unwrap
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

pub async fn print_library_playlists() -> Result<()> {
    let res = get_api().await.get_library_playlists().await?;
    println!("{:#?}", res);
    Ok(())
}
