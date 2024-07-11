//! Due to quota limits - all live api tests are extracted out into their own
//! integration tests module.
use reqwest::Client;
use ytmapi_rs::common::browsing::Lyrics;
use ytmapi_rs::common::watch::WatchPlaylist;
use ytmapi_rs::common::{
    ChannelID, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary, SearchSuggestion,
};
use ytmapi_rs::common::{LyricsID, PlaylistID, TextRun, YoutubeID};
use ytmapi_rs::error::ErrorKind;
use ytmapi_rs::parse::{AlbumParams, ArtistParams, LikeStatus, ParseFrom};
use ytmapi_rs::query::lyrics::GetLyricsQuery;
use ytmapi_rs::query::watch::GetWatchPlaylistQuery;
use ytmapi_rs::query::*;
use ytmapi_rs::Error;
use ytmapi_rs::{auth::*, *};

use crate::utils::{new_standard_api, new_standard_oauth_api, write_json, INVALID_COOKIE};

#[macro_use]
mod utils;

#[tokio::test]
async fn test_refresh_expired_oauth() {
    let mut api = utils::new_standard_oauth_api().await.unwrap();
    api.refresh_token().await.unwrap();
}

#[tokio::test]
async fn test_get_oauth_code() {
    let client = Client::new();
    let _code = OAuthTokenGenerator::new(&client).await.unwrap();
}

// NOTE: Internal only - due to use of error.is_oauth_expired()
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
    assert!(matches!(error.into_kind(), ErrorKind::OAuthTokenExpired));
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
    assert!(matches!(
        error.into_kind(),
        ErrorKind::BrowserAuthenticationFailed
    ));
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

generate_query_test!(test_get_history, GetHistoryQuery {});
generate_query_test!(test_get_library_songs, GetLibrarySongsQuery::default());
generate_query_test!(test_get_library_albums, GetLibraryAlbumsQuery::default());
generate_query_test!(
    test_get_library_artist_subscriptions,
    GetLibraryArtistSubscriptionsQuery::default()
);
generate_query_test!(test_basic_search, SearchQuery::new("Beatles"));
generate_query_test!(
    test_search_artists,
    SearchQuery::new("Beatles").with_filter(ArtistsFilter)
);
generate_query_test!(
    test_search_songs,
    SearchQuery::new("Beatles").with_filter(SongsFilter)
);
generate_query_test!(
    test_search_albums,
    SearchQuery::new("Beatles").with_filter(AlbumsFilter)
);
generate_query_test!(
    test_search_videos,
    SearchQuery::new("Beatles").with_filter(VideosFilter)
);
generate_query_test!(
    test_search_episodes,
    SearchQuery::new("Beatles").with_filter(EpisodesFilter)
);
generate_query_test!(
    test_search_podcasts,
    SearchQuery::new("Beatles").with_filter(PodcastsFilter)
);
generate_query_test!(
    test_search_profiles,
    SearchQuery::new("Beatles").with_filter(ProfilesFilter)
);
generate_query_test!(
    test_search_featured_playlists,
    SearchQuery::new("Beatles").with_filter(FeaturedPlaylistsFilter)
);
generate_query_test!(
    test_search_community_playlists,
    SearchQuery::new("Beatles").with_filter(CommunityPlaylistsFilter)
);
generate_query_test!(
    test_search_playlists,
    SearchQuery::new("Beatles").with_filter(PlaylistsFilter)
);

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
        ))
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_add_remove_songs_from_library() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    // TODO: Confirm what songs these are.
    // Here Comes The Sun (Remastered 2009)
    let song1_add = FeedbackTokenAddToLibrary::from_raw("AB9zfpLNiumq5xDBgjkDSZZyCueh__JX4POenJBVzci5sOatPL8q7zs8D9LIYfLPEJ7k3N4OLy4vMfFr7os-GRla9I8RgMFf0A");
    let song1_rem = FeedbackTokenRemoveFromLibrary::from_raw("AB9zfpJpKmgLWemXCSIlIUIcrBZumoOPWw0Y0NKniqn8ZBFe2Knndo6LnKBMrFjKM1iZYZBYgzKTzATqdMZh-V8nq36Svggu5w");
    // Let it be
    let song2_add = FeedbackTokenAddToLibrary::from_raw("AB9zfpIy-gtxCX1XAx__pFt0APQ_fgGGtuUqY7D7Sz4Oupazo6dxxP-VJEfvnon4eigVa_aYBVPfW99DA2Y9Ns0AEVgbJUeDyQ");
    let song2_rem = FeedbackTokenRemoveFromLibrary::from_raw("AB9zfpLqhDJMIguP_8vxw5e-pV69_x5IVqe8KOy8jBEDoncBCCfAxOcvhaJPRi2NHLiKAukdmZgIlX7uoWcsOvqLA2zgNGUNAw");
    let q1 = EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(vec![song1_add]);
    let q2 = EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(vec![song2_add])
        .with_remove_from_library_feedback_tokens(vec![song1_rem]);
    let q3 =
        EditSongLibraryStatusQuery::new_from_remove_from_library_feedback_tokens(vec![song2_rem]);
    api.query(q1)
        .await
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .unwrap();
    api.query(q2)
        .await
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .unwrap();
    api.query(q3)
        .await
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .unwrap();
}
#[tokio::test]
async fn test_rate_songs() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    // TODO: Confirm what songs these are.
    api.rate_song(VideoID::from_raw("kfSQkZuIx84"), LikeStatus::Liked)
        .await
        .unwrap();
    api.rate_song(VideoID::from_raw("EjHzPrBCgf0"), LikeStatus::Disliked)
        .await
        .unwrap();
    api.rate_song(VideoID::from_raw("kfSQkZuIx84"), LikeStatus::Indifferent)
        .await
        .unwrap();
    api.rate_song(VideoID::from_raw("EjHzPrBCgf0"), LikeStatus::Indifferent)
        .await
        .unwrap();
}
#[tokio::test]
async fn test_rate_playlists() {
    // TODO: Add siginficantly more queries.
    let api = new_standard_api().await.unwrap();
    api.rate_playlist(
        // Beatles Jukebox (Featured Playlist)
        PlaylistID::from_raw("RDCLAK5uy_lHIiCEeknPkpJOowyykpfBu-ECJB9Q32I"),
        LikeStatus::Liked,
    )
    .await
    .unwrap();
    api.rate_playlist(
        // The Beatles - Beatles 100 (Community Playlist)
        PlaylistID::from_raw("PL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu"),
        LikeStatus::Disliked,
    )
    .await
    .unwrap();
    api.rate_playlist(
        // Beatles Jukebox (Featured Playlist)
        PlaylistID::from_raw("RDCLAK5uy_lHIiCEeknPkpJOowyykpfBu-ECJB9Q32I"),
        LikeStatus::Indifferent,
    )
    .await
    .unwrap();
    api.rate_playlist(
        // The Beatles - Beatles 100 (Community Playlist)
        PlaylistID::from_raw("PL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu"),
        LikeStatus::Indifferent,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn test_delete_create_playlist_complex() {
    // TODO: Add siginficantly more queries.
    // TODO: Oauth.
    let api = new_standard_api().await.unwrap();
    let id = api
        .create_playlist(
            CreatePlaylistQuery::new(
                "TEST PLAYLIST",
                Some("TEST DESCRIPTION"),
                PrivacyStatus::Unlisted,
            )
            .with_video_ids(vec![
                VideoID::from_raw("kfSQkZuIx84"),
                VideoID::from_raw("EjHzPrBCgf0"),
                VideoID::from_raw("Av-gUkwzvzk"),
            ]),
        )
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_add_remove_playlist_items() {
    // TODO: Oauth.
    let api = new_standard_api().await.unwrap();
    let id = api
        .create_playlist(CreatePlaylistQuery::new(
            "TEST PLAYLIST",
            None,
            PrivacyStatus::Unlisted,
        ))
        .await
        .unwrap();
    let set_video_ids = api
        .add_playlist_video_items(AddPlaylistItemsQuery::new_from_videos(
            id.clone(),
            vec![VideoID::from_raw("kfSQkZuIx84")],
            Default::default(),
        ))
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.set_video_id)
        .collect();
    api.remove_playlist_items(RemovePlaylistItemsQuery::new(id.clone(), set_video_ids))
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_edit_playlist() {
    // TODO: Add siginficantly more queries.
    // TODO: Oauth.
    let api = new_standard_api().await.unwrap();
    let id = api
        .create_playlist(CreatePlaylistQuery::new(
            "TEST PLAYLIST",
            None,
            PrivacyStatus::Unlisted,
        ))
        .await
        .unwrap();
    EditPlaylistQuery::new_title(id.clone(), "TEST_EDIT")
        .call(&api)
        .await
        .unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
async fn test_get_playlist_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
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
    api.refresh_token().await.unwrap();
    let _res = api.search_songs("Beatles").await.unwrap();
}
#[tokio::test]
async fn test_get_library_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
    let res = api.get_library_playlists().await.unwrap();
    assert!(!res.is_empty());
}
#[tokio::test]
async fn test_get_library_playlists() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_library_playlists().await.unwrap();
    assert!(!res.is_empty());
}
#[tokio::test]
async fn test_get_library_artists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
    let query = GetLibraryArtistsQuery::default();
    let res = api.get_library_artists(query).await.unwrap();
    assert!(!res.is_empty());
}
#[tokio::test]
async fn test_get_library_artists() {
    let api = new_standard_api().await.unwrap();
    let query = GetLibraryArtistsQuery::default();
    let res = api.get_library_artists(query).await.unwrap();
    assert!(!res.is_empty());
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
        lyrics_id: LyricsID::from_raw("MPLYt_C8aRK1qmsDJ-1"),
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
    api.refresh_token().await.unwrap();
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = SearchSuggestion {
        suggestion_type: common::SuggestionType::Prediction,
        runs: vec![
            TextRun::Bold("faded".into()),
            TextRun::Normal(" alan walker".into()),
        ],
    };
    assert!(res.contains(&example));
}
#[tokio::test]
async fn test_search_suggestions() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_search_suggestions("faded").await.unwrap();
    let example = SearchSuggestion {
        suggestion_type: common::SuggestionType::Prediction,
        runs: vec![
            TextRun::Bold("faded".into()),
            TextRun::Normal(" alan walker".into()),
        ],
    };
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
