//! Due to quota limits - all live api tests are extracted out into their own
//! integration tests module.
use common::{EpisodeID, LikeStatus, PodcastChannelID, PodcastChannelParams, PodcastID, VideoID};
use parse::{GetArtistAlbumsAlbum, Lyrics};
use std::time::Duration;
use ytmapi_rs::common::{
    ApiOutcome, ArtistChannelID, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary,
};
use ytmapi_rs::common::{LyricsID, PlaylistID, YoutubeID};
use ytmapi_rs::error::ErrorKind;
use ytmapi_rs::parse::{ArtistParams, GetAlbum, ParseFrom};
use ytmapi_rs::query::*;
use ytmapi_rs::{auth::*, *};

use crate::utils::{new_standard_api, new_standard_oauth_api, INVALID_COOKIE};

#[macro_use]
mod utils;

#[tokio::test]
async fn test_refresh_expired_oauth() {
    let mut api = utils::new_standard_oauth_api().await.unwrap();
    api.refresh_token().await.unwrap();
}

#[tokio::test]
async fn test_get_oauth_code() {
    let client = crate::client::Client::new().unwrap();
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
    assert!(matches!(
        error.into_kind(),
        ErrorKind::OAuthTokenExpired { .. }
    ));
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
//// BASIC STREAM TESTS
generate_stream_test!(
    test_stream_get_library_songs,
    GetLibrarySongsQuery::default()
);
generate_stream_test!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_artist_subscriptions,
    GetLibraryArtistSubscriptionsQuery::default()
);
generate_stream_test!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_playlists,
    GetLibraryPlaylistsQuery
);
generate_stream_test!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_albums,
    GetLibraryAlbumsQuery::default()
);
generate_stream_test!(
    test_stream_get_library_artists,
    GetLibraryArtistsQuery::default()
);
generate_query_test!(
    test_search_suggestions,
    GetSearchSuggestionsQuery::new("faded")
);

generate_query_test!(test_get_mood_categories, GetMoodCategoriesQuery);
// NOTE: Set Taste Profile test is not implemented, to avoid impact to my YTM
// recommendations.
generate_query_test!(test_get_taste_profile, GetTasteProfileQuery);
generate_query_test!(test_get_history, GetHistoryQuery);
generate_query_test!(
    test_get_channel,
    // Rustacean Station
    GetChannelQuery::new(PodcastChannelID::from_raw("UCzYLos4qc2oC4r0Efd-tSuw"),)
);
// NOTE: Can be flaky - visiting this page on the website seems to reset it.
generate_query_test!(
    test_get_channel_episodes,
    // Rustacean Station
    GetChannelEpisodesQuery::new(
        PodcastChannelID::from_raw("UCzYLos4qc2oC4r0Efd-tSuw"),
        PodcastChannelParams::from_raw("6gPmAUdxa0JXcGtCQ3BZQkNpUjVkRjl3WVdkbFgzTnVZWEJ6YUc5MFgyMTFjMmxqWDNCaFoyVmZjbVZuYVc5dVlXd1NIM05mUzNKVGJtWlphemhuWmtWUWEzaDRSRVpqWWxSS1R6UXllbDlIYUdzYVRRQUFaVzR0UjBJQUFVRlZBQUZCVlFBQkFFWkZiWFZ6YVdOZlpHVjBZV2xzWDJGeWRHbHpkQUFCQVVNQUFBRUFBQUVCQUZWRGVsbE1iM00wY1dNeWIwTTBjakJGWm1RdGRGTjFkd0FCOHRxenFnb0hRQUJJQUZDYkFR")
    )
);
generate_query_test!(
    test_get_podcast,
    // Rustacean Station
    GetPodcastQuery::new(PodcastID::from_raw(
        "MPSPPLWnnGn_Lw9os50MbtFCouWYsArlq2s8ct"
    ))
);
generate_query_test!(
    test_get_episode,
    // Chasing scratch S7E21
    GetEpisodeQuery::new(EpisodeID::from_raw("MPED2i5poDoWjFU"))
);
generate_query_test!(test_get_new_episodes_playlist, GetNewEpisodesQuery);
generate_query_test!(
    test_get_playlist,
    GetPlaylistQuery::new(PlaylistID::from_raw("VLPL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu"))
);
generate_query_test!(
    test_get_artist,
    GetArtistQuery::new(ArtistChannelID::from_raw("UC2XdaAVUannpujzv32jcouQ",))
);
generate_query_test!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_upload_songs,
    GetLibraryUploadSongsQuery::default()
);
generate_query_test!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_upload_albums,
    GetLibraryUploadAlbumsQuery::default()
);
generate_query_test!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_upload_artists,
    GetLibraryUploadArtistsQuery::default()
);
generate_query_test!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_songs,
    GetLibrarySongsQuery::default()
);
generate_query_test!(test_get_library_albums, GetLibraryAlbumsQuery::default());
generate_query_test!(
    test_get_library_artist_subscriptions,
    GetLibraryArtistSubscriptionsQuery::default()
);
generate_query_test!(test_basic_search, SearchQuery::new("Beatles"));
generate_query_test!(
    test_basic_search_alternate_query_1,
    SearchQuery::new("Beaten")
);
generate_query_test!(
    test_basic_search_alternate_query_2,
    SearchQuery::new("Chasing scratch")
);
generate_query_test!(
    test_basic_search_alternate_query_3_genre,
    SearchQuery::new("Metal")
);
generate_query_test!(
    test_basic_search_alternate_query_no_results,
    SearchQuery::new("aaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbcccccccccccccccccc")
);
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

// # MULTISTAGE TESTS

#[tokio::test]
async fn test_get_mood_playlists() {
    let browser_api = crate::utils::new_standard_api().await.unwrap();
    let first_mood_playlist = browser_api
        .query(GetMoodCategoriesQuery)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .mood_categories
        .into_iter()
        .next()
        .unwrap();
    let query = GetMoodPlaylistsQuery::new(first_mood_playlist.params);
    let oauth_fut = async {
        let mut api = crate::utils::new_standard_oauth_api().await.unwrap();
        // Don't stuff around trying the keep the local OAuth secret up to
        //date, just refresh it each time.
        api.refresh_token().await.unwrap();
        api.query(query.clone()).await.unwrap();
    };
    let browser_fut = async {
        browser_api.query(query.clone()).await.unwrap();
    };
    tokio::join!(oauth_fut, browser_fut);
}

#[ignore = "Ignored by default due to quota"]
#[tokio::test]
async fn test_get_library_upload_artist() {
    let browser_api = crate::utils::new_standard_api().await.unwrap();
    let first_artist = browser_api
        .query(GetLibraryUploadArtistsQuery::default())
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("To run this test, you will need to upload songs from at least one artist");
    let query = GetLibraryUploadArtistQuery::new(first_artist.artist_id);
    let oauth_fut = async {
        let mut api = crate::utils::new_standard_oauth_api().await.unwrap();
        // Don't stuff around trying the keep the local OAuth secret up to date, just
        // refresh it each time.
        api.refresh_token().await.unwrap();
        let _ = api.query(query.clone()).await.unwrap();
    };
    let browser_fut = async {
        browser_api.query(query.clone()).await.unwrap();
    };
    tokio::join!(oauth_fut, browser_fut);
}

#[ignore = "Ignored by default due to quota"]
#[tokio::test]
async fn test_get_library_upload_album() {
    let browser_api = crate::utils::new_standard_api().await.unwrap();
    let first_album = browser_api
        .query(GetLibraryUploadAlbumsQuery::default())
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("To run this test, you will need to upload songs from at least one album");
    let query = GetLibraryUploadAlbumQuery::new(first_album.album_id);
    let oauth_fut = async {
        let mut api = crate::utils::new_standard_oauth_api().await.unwrap();
        // Don't stuff around trying the keep the local OAuth secret up to date, just
        // refresh it each time.
        api.refresh_token().await.unwrap();
        let _ = api.query(query.clone()).await.unwrap();
    };
    let browser_fut = async {
        browser_api.query(query.clone()).await.unwrap();
    };
    tokio::join!(oauth_fut, browser_fut);
}

#[tokio::test]
async fn test_get_artist_albums() {
    let now = std::time::Instant::now();
    let api = new_standard_api().await.unwrap();
    println!("API took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let q = GetArtistQuery::new(ArtistChannelID::from_raw(
        // Metallica
        "UCGexNm_Kw4rdQjLxmpb2EKw",
    ));
    let res = api.raw_query(&q).await.unwrap();
    println!("Get artist took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = res.process().unwrap();
    let res: ArtistParams = ParseFrom::parse_from(res).unwrap();
    println!("Parse artist took {} ms", now.elapsed().as_millis());
    let _now = std::time::Instant::now();
    let albums = res.top_releases.albums.unwrap();
    let params = albums.params.unwrap();
    let channel_id = albums.browse_id.unwrap();
    api.get_artist_albums(channel_id, params).await.unwrap();
    let now = std::time::Instant::now();
    println!("Get albums took {} ms", now.elapsed().as_millis());
}

#[tokio::test]
async fn test_get_artist_album_songs() {
    let now = std::time::Instant::now();
    let api = new_standard_api().await.unwrap();
    println!("API took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let q = GetArtistQuery::new(ArtistChannelID::from_raw(
        // Metallica
        "UCGexNm_Kw4rdQjLxmpb2EKw",
    ));
    let res = api.raw_query(&q).await.unwrap();
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
    let q = GetArtistAlbumsQuery::new(ArtistChannelID::from_raw(channel_id.get_raw()), params);
    let res = api.raw_query(&q).await.unwrap();
    println!("Get albums took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let res = res.process().unwrap();
    let res: Vec<GetArtistAlbumsAlbum> = ParseFrom::parse_from(res).unwrap();
    println!("Process albums took {} ms", now.elapsed().as_millis());
    let now = std::time::Instant::now();
    let browse_id = &res[0].browse_id;
    let q = GetAlbumQuery::new(browse_id.clone());
    let res = api.raw_query(&q).await.unwrap();
    println!(
        "Get album {} took {} ms",
        browse_id.get_raw(),
        now.elapsed().as_millis()
    );
    let now = std::time::Instant::now();
    let res = res.process().unwrap();
    let _ = GetAlbum::parse_from(res).unwrap();
    println!("Process album took {} ms", now.elapsed().as_millis());
}

// # STATEFUL TESTS

#[tokio::test]
async fn test_add_remove_history_items() {
    // Timeout to avoid flaky test
    const GET_HISTORY_TIMEOUT: Duration = Duration::from_millis(2000);
    // TODO: Oauth.
    let api = new_standard_api().await.unwrap();
    let song = api
        .search_songs("Ride the lightning")
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let song_url = api.get_song_tracking_url(&song.video_id).await.unwrap();
    api.add_history_item(song_url).await.unwrap();
    // Get history has a slight lag.
    tokio::time::sleep(GET_HISTORY_TIMEOUT).await;
    let history = api.get_history().await.unwrap();
    let first_history_item_period = history.first().unwrap().period_name.clone();
    let (first_history_item_name, delete_token) =
        match history.first().unwrap().items.first().unwrap() {
            parse::HistoryItem::Song(i) => (i.title.as_str(), i.feedback_token_remove.clone()),
            parse::HistoryItem::Video(i) => (i.title.as_str(), i.feedback_token_remove.clone()),
            parse::HistoryItem::Episode(i) => (i.title.as_str(), i.feedback_token_remove.clone()),
            parse::HistoryItem::UploadSong(i) => {
                (i.title.as_str(), i.feedback_token_remove.clone())
            }
        };
    pretty_assertions::assert_eq!(first_history_item_name, song.title);
    assert!(matches!(
        api.remove_history_items(vec![delete_token])
            .await
            .unwrap()
            .first(),
        Some(ApiOutcome::Success)
    ));
    // Get history has a slight lag.
    tokio::time::sleep(GET_HISTORY_TIMEOUT).await;
    let history = api.get_history().await.unwrap();
    let first_history_item_period_new = history.first().unwrap().period_name.clone();
    let first_history_item_name_new = match history.first().unwrap().items.first().unwrap() {
        parse::HistoryItem::Song(i) => i.title.as_str(),
        parse::HistoryItem::Video(i) => i.title.as_str(),
        parse::HistoryItem::Episode(i) => i.title.as_str(),
        parse::HistoryItem::UploadSong(i) => i.title.as_str(),
    };
    // It's not enough to check last song isn't the one we added - as it may exist
    // on the previous day too :)
    pretty_assertions::assert_ne!(
        (first_history_item_name_new, first_history_item_period),
        (song.title.as_str(), first_history_item_period_new)
    );
}

#[tokio::test]
#[ignore = "Ignored by default due to quota"]
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
#[ignore = "Ignored by default due to quota"]
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
    // Let It Be
    let song2_add = FeedbackTokenAddToLibrary::from_raw("AB9zfpIy-gtxCX1XAx__pFt0APQ_fgGGtuUqY7D7Sz4Oupazo6dxxP-VJEfvnon4eigVa_aYBVPfW99DA2Y9Ns0AEVgbJUeDyQ");
    let song2_rem = FeedbackTokenRemoveFromLibrary::from_raw("AB9zfpLqhDJMIguP_8vxw5e-pV69_x5IVqe8KOy8jBEDoncBCCfAxOcvhaJPRi2NHLiKAukdmZgIlX7uoWcsOvqLA2zgNGUNAw");
    let q1 = EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(vec![song1_add]);
    let q2 = EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(vec![song2_add])
        .with_remove_from_library_feedback_tokens(vec![song1_rem]);
    let q3 =
        EditSongLibraryStatusQuery::new_from_remove_from_library_feedback_tokens(vec![song2_rem]);
    assert!(!api
        .query(q1)
        .await
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .any(|x| x == ApiOutcome::Failure));
    assert!(!api
        .query(q2)
        .await
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .any(|x| x == ApiOutcome::Failure));
    assert!(!api
        .query(q3)
        .await
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .any(|x| x == ApiOutcome::Failure));
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
#[ignore = "Ignored by default due to quota"]
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
#[ignore = "Ignored by default due to quota"]
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
        .add_video_items_to_playlist(&id, vec![VideoID::from_raw("kfSQkZuIx84")])
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.set_video_id)
        .collect();
    api.remove_playlist_items(&id, set_video_ids).await.unwrap();
    api.delete_playlist(id).await.unwrap();
}
#[tokio::test]
#[ignore = "Ignored by default due to quota"]
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
    assert_eq!(
        api.query(EditPlaylistQuery::new_title(id.clone(), "TEST_EDIT"))
            .await
            .unwrap(),
        ApiOutcome::Success
    );
    api.delete_playlist(id).await.unwrap();
}

// # BASIC TESTS WITH ADDITIONAL ASSERTIONS

#[tokio::test]
async fn test_get_library_playlists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
    let res = api.get_library_playlists().await.unwrap();
    assert!(!res.playlists.is_empty());
}
#[tokio::test]
async fn test_get_library_playlists() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_library_playlists().await.unwrap();
    assert!(!res.playlists.is_empty());
}
#[tokio::test]
async fn test_get_library_artists_oauth() {
    let mut api = new_standard_oauth_api().await.unwrap();
    // Don't stuff around trying the keep the local OAuth secret up to date, just
    // refresh it each time.
    api.refresh_token().await.unwrap();
    let res = api.get_library_artists().await.unwrap();
    assert!(!res.artists.is_empty());
}
#[tokio::test]
async fn test_get_library_artists() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_library_artists().await.unwrap();
    assert!(!res.artists.is_empty());
}
#[tokio::test]
async fn test_watch_playlist() {
    // TODO: Make more generic
    let api = new_standard_api().await.unwrap();
    let res = api
        .get_watch_playlist_from_video_id(VideoID::from_raw("9mWr4c_ig54"))
        .await
        .unwrap();
    assert_eq!(
        res.playlist_id,
        Some(PlaylistID::from_raw("RDAMVM9mWr4c_ig54"))
    );
    assert_eq!(res.lyrics_id, LyricsID::from_raw("MPLYt_C8aRK1qmsDJ-1"));
}
#[tokio::test]
async fn test_get_lyrics() {
    // TODO: Make more generic
    let api = new_standard_api().await.unwrap();
    let res = api
        .get_watch_playlist_from_video_id(VideoID::from_raw("9mWr4c_ig54"))
        .await
        .unwrap();
    let res = api.get_lyrics(res.lyrics_id).await.unwrap();
    let example = serde_json::json! ({
        "lyrics": "You're my lesson I had to learn\nAnother page I'll have to turn\nI got one more message, always tryna be heard\nBut you never listen to a word\n\nHeaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nJust give me one second and I'll be fine\nJust let me catch my breath and come back to life\nI finally get the message, you were never meant to be mine\nCouldn't see the truth, I was blind (meant to be mine)\n\nWhoa, heaven knows we came so close\nBut this ain't real, it's just a dream\nWake me up, I've been fast asleep\nLetting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me\n\nFoolish of me\nFoolish of me\nFoolish of me\nFoolish of me\n\nLetting go, we came so close (how foolish of me)\nOh, I'm letting go of fantasies\nBeen caught up in who I needed you to be\nHow foolish of me",
        "source": "Source: Musixmatch",
    });
    let example: Lyrics = serde_json::from_value(example).unwrap();
    assert_eq!(res, example)
}
