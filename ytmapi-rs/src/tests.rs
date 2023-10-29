use serde_json::json;

use super::common::{BrowseID, ChannelID};
use super::query::*;
use super::*;
use crate::common::{AlbumID, LyricsID, PlaylistID, YoutubeID};
use crate::Error;

async fn new_standard_oauth_api() -> Result<YtMusic> {
    let oauth_token = tokio::fs::read("oauth.json").await.unwrap();
    YtMusic::from_oauth_token(serde_json::from_slice(&oauth_token).unwrap()).await
}
async fn new_standard_api() -> Result<YtMusic> {
    YtMusic::from_header_file(Path::new("headers.txt")).await
}

#[tokio::test]
async fn test_new() {
    new_standard_api().await.unwrap();
    new_standard_oauth_api().await.unwrap();
}
#[tokio::test]
async fn test_search_oauth() {
    let api = new_standard_oauth_api().await.unwrap();
    let query = SearchQuery::new("Beatles")
        .set_filter(Filter::Artists)
        .set_spelling_mode(SpellingMode::ExactMatch);
    let res = api.search(query).await.unwrap();
}
#[tokio::test]
async fn test_search() {
    let api = new_standard_api().await.unwrap();
    let query = SearchQuery::new("Beatles")
        .set_filter(Filter::Artists)
        .set_spelling_mode(SpellingMode::ExactMatch);
    let res = api.search(query).await.unwrap();
}
#[tokio::test]
async fn test_watch_playlist() {
    // TODO: Make more generic
    let api = YtMusic::from_header_file(Path::new("headers.txt"))
        .await
        .unwrap();
    let res = api
        .get_watch_playlist(GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(
            "9mWr4c_ig54",
        )))
        .await
        .unwrap();
    let example = WatchPlaylist {
        _tracks: Vec::new(),
        playlist_id: Some(PlaylistID::from_raw("RDAMVM9mWr4c_ig54")),
        lyrics_id: LyricsID("MPLYt_C8aRK1qmsDJ-1".into()),
    };
    assert_eq!(res, example)
}
#[tokio::test]
async fn test_get_lyrics() {
    // TODO: Make more generic
    let api = YtMusic::from_header_file(Path::new("headers.txt"))
        .await
        .unwrap();
    let res = api
        .get_watch_playlist(GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(
            "9mWr4c_ig54",
        )))
        .await
        .unwrap();
    let res = api
        .get_lyrics(GetLyricsQuery::new(res.lyrics_id))
        .await
        .unwrap();
    let example = Lyrics {
            lyrics: "You're my lesson I had to learn\nAnother page I'll have to turn\nI got one more message, always tryna be heard\nBut you never listen to a word\n\nHeaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nJust give me one second and I'll be fine\nJust let me catch my breath and come back to life\nI finally get the message, you were never meant to be mine\nCouldn't see the truth, I was blind (meant to be mine)\n\nWhoa, heaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nLetting go, we came so close (how foolish of me)\nOh, I'm letting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me".into(),
            source: "Source: Musixmatch".into(),
        };
    assert_eq!(res, example)
}
#[tokio::test]
async fn test_search_suggestions_oauth() {
    let api = new_standard_oauth_api().await.unwrap();
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = vec![
        TextRun::Bold("faded".into()),
        TextRun::Normal(" alan walker".into()),
    ];
    assert!(res.contains(&example));
}
#[tokio::test]
async fn test_search_suggestions() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = vec![
        TextRun::Bold("faded".into()),
        TextRun::Normal(" alan walker".into()),
    ];
    assert!(res.contains(&example));
}
#[tokio::test]
async fn test_get_artist() {
    let now = std::time::Instant::now();
    let api = YtMusic::from_header_file(Path::new("headers.txt"))
        .await
        .unwrap();
    println!("API took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = api
        .raw_query(GetArtistQuery::new(ChannelID::from_raw(
            "UC2XdaAVUannpujzv32jcouQ",
        )))
        .await
        .unwrap();
    println!("Get artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = res.process().unwrap();
    let res = res.parse().unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
}
#[tokio::test]
async fn test_get_artist_albums() {
    let now = std::time::Instant::now();
    let api = YtMusic::from_header_file(Path::new("headers.txt"))
        .await
        .unwrap();
    println!("API took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = api
        .raw_query(GetArtistQuery::new(ChannelID::from_raw(
            // Metallica
            "UCGexNm_Kw4rdQjLxmpb2EKw",
        )))
        .await
        .unwrap();
    println!("Get artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    // TODO: fix temporary value dropped while borrowed error.
    // This won't compile:
    // let res = res.process().unwrap().parse().unwrap();
    let res = res.process().unwrap();
    let res = res.parse().unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let albums = res.top_releases.albums.unwrap();
    let params = albums.params.unwrap();
    // For some reason the params is wrong. needs investigation.
    let channel_id = &albums.browse_id.unwrap();
    let q = GetArtistAlbumsQuery::new(ChannelID::from_raw(channel_id.get_raw()), params);
    api.get_artist_albums(q).await.unwrap();
    let now = std::time::Instant::now();
    println!("Get albums took {} ms", now.elapsed().as_millis());
}

#[tokio::test]
async fn test_get_oauth_code() {
    let client = Client::new();
    let code = OAuthTokenGenerator::new(&client).await.unwrap();
}

#[tokio::test]
async fn test_get_artist_album_songs() {
    let now = std::time::Instant::now();
    let api = YtMusic::from_header_file(Path::new("headers.txt"))
        .await
        .unwrap();
    println!("API took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = api
        .raw_query(GetArtistQuery::new(ChannelID::from_raw(
            "UCGexNm_Kw4rdQjLxmpb2EKw",
        )))
        .await
        .unwrap();
    println!("Get artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    // TODO: fix temporary value dropped while borrowed error.
    // This won't compile:
    // let res = res.process().unwrap().parse().unwrap();
    let res = res.process().unwrap();
    let res = res.parse().unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let albums = res.top_releases.albums.unwrap();
    let params = albums.params.unwrap();
    let channel_id = &albums.browse_id.unwrap();
    let res = api
        .raw_query(GetArtistAlbumsQuery::new(
            ChannelID::from_raw(channel_id.get_raw()),
            params,
        ))
        .await
        .unwrap();
    println!("Get albums took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = res.process().unwrap();
    let res = res.parse().unwrap();
    println!("Process albums took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let browse_id = AlbumID::from_raw(&res[0].browse_id);
    let res = api.raw_query(GetAlbumQuery::new(browse_id)).await.unwrap();
    println!("Get album took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = res.process().map_err(|e| write_json(&e)).unwrap();
    let res = res.parse().unwrap();
    println!("Process album took {} ms", now.elapsed().as_millis());
}
pub fn write_json(e: &Error) {
    if let Some((json, key)) = e.get_json_and_key() {
        std::fs::write("err.json", json)
            .unwrap_or_else(|_| eprintln!("Error writing json to err.json"));
        panic!("{key} not found, wrote to err.json");
    }
}
