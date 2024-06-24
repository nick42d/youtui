use super::common::ChannelID;
use super::query::*;
use super::*;
use crate::common::{LyricsID, PlaylistID, TextRun, YoutubeID};
use crate::parse::GetPlaylist;
use crate::Error;
use std::env;

const COOKIE_PATH: &str = "cookie.txt";
const EXPIRED_OAUTH_PATH: &str = "oauth.json";
// Cookie filled with nonsense values to test this case.
const INVALID_COOKIE: &str = "HSID=abc; SSID=abc; APISID=abc; SAPISID=abc; __Secure-1PAPISID=abc; __Secure-3PAPISID=abc; YSC=abc; LOGIN_INFO=abc; VISITOR_INFO1_LIVE=abc; _gcl_au=abc; PREF=tz=Australia.Perth&f6=40000000&f7=abc; VISITOR_PRIVACY_METADATA=abc; __Secure-1PSIDTS=abc; __Secure-3PSIDTS=abc; SID=abc; __Secure-1PSID=abc; __Secure-3PSID=abc; SIDCC=abc; __Secure-1PSIDCC=abc; __Secure-3PSIDCC=abc";
// Placeholder for future implementation.
// const INVALID_EXPIRED_OAUTH: &str = "
// {
//   \"token_type\": \"Bearer\",
//   \"access_token\": \"abc\",
//   \"refresh_token\": \"abc\",
//   \"expires_in\": 62609,
//   \"request_time\": {
//     \"secs_since_epoch\": 1702907669,
//     \"nanos_since_epoch\": 594642820
//   }
// }";

async fn new_standard_oauth_api() -> Result<YtMusic<OAuthToken>> {
    let oauth_token = if let Ok(tok) = env::var("youtui_test_oauth") {
        tok
    } else {
        tokio::fs::read_to_string(EXPIRED_OAUTH_PATH).await.unwrap()
    };
    Ok(YtMusic::from_oauth_token(
        serde_json::from_slice(oauth_token.as_bytes()).unwrap(),
    ))
}
async fn new_standard_api() -> Result<YtMusic<BrowserToken>> {
    if let Ok(cookie) = env::var("youtui_test_cookie") {
        YtMusic::from_cookie(cookie).await
    } else {
        YtMusic::from_cookie_file(Path::new(COOKIE_PATH)).await
    }
}
pub fn write_json(e: &Error) {
    if let Some((json, key)) = e.get_json_and_key() {
        std::fs::write("err.json", json)
            .unwrap_or_else(|_| eprintln!("Error writing json to err.json"));
        panic!("{key} not found, wrote to err.json");
    }
}

#[tokio::test]
async fn test_refresh_expired_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    api.refresh_token().await.unwrap();
}
#[tokio::test]
async fn test_expired_oauth() {
    // XXX: Assuming this error only occurs for expired headers.
    // This assumption may be incorrect.
    let api = new_standard_oauth_api().await.unwrap();
    // Library query needs authentication.
    let res = api.json_query(GetLibraryPlaylistsQuery).await;
    // TODO: Add matching functions to error type. Current method not very
    // ergonomic.
    let Err(error) = res else {
        panic!("Expected an error")
    };
    assert!(error.is_oauth_expired());
}
// Placeholder for future implementation.
// #[tokio::test]
// async fn test_expired_header() {
// }
#[tokio::test]
async fn test_invalid_header() {
    let api = YtMusic::from_cookie(INVALID_COOKIE).await;
    // Library query needs authentication.
    let res = api.unwrap().json_query(GetLibraryPlaylistsQuery).await;
    // TODO: Add matching functions to error type. Current method not very
    // ergonomic.
    let Err(error) = res else {
        eprintln!("{:#?}", res);
        panic!("Expected an error")
    };
    assert!(error.is_browser_authentication_failed());
}
// Placeholder for future implementation
// #[tokio::test]
// async fn test_invalid_expired_oauth() {
//     let oauth_token: OAuthToken =
// serde_json::from_str(INVALID_EXPIRED_OAUTH).unwrap();     let api =
// YtMusic::from_oauth_token(oauth_token);     // Library query needs
// authentication.     let res = api.json_query(GetLibraryPlaylistsQuery).await;
//     // TODO: Add matching functions to error type. Current method not very
// ergonomic.     let Err(error) = res else {
//         eprintln!("{:#?}", res);
//         panic!("Expected an error")
//     };
//     eprintln!("{:#?}", error);
//     assert!(error.is_browser_authentication_failed());
// }

#[tokio::test]
async fn test_new() {
    new_standard_api().await.unwrap();
    new_standard_oauth_api().await.unwrap();
}
#[tokio::test]
async fn test_basic_search() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_basic_search_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_artists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_artists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_artists() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    let _res = api.search_artists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_delete_create_playlist_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
    let id = api
        .create_playlist(CreatePlaylistQuery::new(
            "TEST PLAYLIST",
            None,
            PrivacyStatus::Unlisted,
            Vec::new(),
            None,
        ))
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_delete_create_playlist() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    let id = api
        .create_playlist(CreatePlaylistQuery::new(
            "TEST PLAYLIST",
            None,
            PrivacyStatus::Unlisted,
            Vec::new(),
            None,
        ))
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_delete_create_playlist_complex() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    let id = api
        .create_playlist(CreatePlaylistQuery::new(
            "TEST PLAYLIST",
            Some("TEST DESCRIPTION"),
            PrivacyStatus::Unlisted,
            vec![
                VideoID::from_raw("kfSQkZuIx84"),
                VideoID::from_raw("EjHzPrBCgf0"),
                VideoID::from_raw("Av-gUkwzvzk"),
            ],
            Some(PlaylistID::from_raw("VLPLCZQcydUIP07X8WURoQP8YEwKwVM7K2xl")),
        ))
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_get_playlist_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    api.get_playlist(GetPlaylistQuery::new(PlaylistID::from_raw(
        "VLPL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu",
    )))
    .await
    .unwrap();
}
#[tokio::test]
async fn test_get_playlist() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    api.get_playlist(GetPlaylistQuery::new(PlaylistID::from_raw(
        "VLPL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu",
    )))
    .await
    .unwrap();
}
#[tokio::test]
async fn test_search_songs_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_songs("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_songs() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_songs("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_albums_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_albums("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_albums() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_albums("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_videos_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_videos("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_videos() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_videos("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_episodes_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_episodes("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_episodes() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_episodes("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_podcasts_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_podcasts("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_podcasts() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_podcasts("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_profiles_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_profiles("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_profiles() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_profiles("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_featured_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_featured_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_featured_playlists() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_featured_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_community_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_community_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_community_playlists() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_community_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let _res = api.search_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_search_playlists() {
    let api = new_standard_api().await.unwrap();
    let _res = api.search_playlists("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_get_library_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let res = api.get_library_playlists().await.unwrap();
    assert!(res.len() > 0);
}
#[tokio::test]
async fn test_get_library_playlists() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_library_playlists().await.unwrap();
    assert!(res.len() > 0);
}
#[tokio::test]
async fn test_get_library_artists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let query = GetLibraryArtistsQuery::default();
    let res = api.get_library_artists(query).await.unwrap();
    assert!(res.len() > 0);
}
#[tokio::test]
async fn test_get_library_artists() {
    let api = new_standard_api().await.unwrap();
    let query = GetLibraryArtistsQuery::default();
    let res = api.get_library_artists(query).await.unwrap();
    assert!(res.len() > 0);
}
#[tokio::test]
async fn test_watch_playlist() {
    // TODO: Make more generic
    let api = new_standard_api().await.unwrap();
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
    let api = new_standard_api().await.unwrap();
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
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await;
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = SearchSuggestion::new(
        common::SuggestionType::Prediction,
        vec![
            TextRun::Bold("faded".into()),
            TextRun::Normal(" alan walker".into()),
        ],
    );
    assert!(res.contains(&example));
}
#[tokio::test]
async fn test_search_suggestions() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = SearchSuggestion::new(
        common::SuggestionType::Prediction,
        vec![
            TextRun::Bold("faded".into()),
            TextRun::Normal(" alan walker".into()),
        ],
    );
    assert!(res.contains(&example));
}
#[tokio::test]
async fn test_get_artist() {
    let now = std::time::Instant::now();
    let api = new_standard_api().await.unwrap();
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
    let _ = ArtistParams::parse_from(res).unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
}
#[tokio::test]
async fn test_get_artist_albums() {
    let now = std::time::Instant::now();
    let api = new_standard_api().await.unwrap();
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
    let res = ArtistParams::parse_from(res).unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
    let _now = std::time::Instant::now();
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
    let _code = OAuthTokenGenerator::new(&client).await.unwrap();
}

#[tokio::test]
async fn test_get_artist_album_songs() {
    let now = std::time::Instant::now();
    let api = new_standard_api().await.unwrap();
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
    let res = ArtistParams::parse_from(res).unwrap();
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
    let res = Vec::<Album>::parse_from(res).unwrap();
    println!("Process albums took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let browse_id = &res[0].browse_id;
    let res = api
        .raw_query(GetAlbumQuery::new(browse_id.clone()))
        .await
        .unwrap();
    println!(
        "Get album {} took {} ms",
        browse_id.get_raw(),
        now.elapsed().as_millis()
    );
    let now = std::time::Instant::now();
    let res = res.process().map_err(|e| write_json(&e)).unwrap();
    let _ = AlbumParams::parse_from(res).unwrap();
    println!("Process album took {} ms", now.elapsed().as_millis());
}
