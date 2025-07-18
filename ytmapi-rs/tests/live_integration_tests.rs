//! Due to quota limits - all live api tests are extracted out into their own
//! integration tests module.
use crate::utils::{new_standard_api, new_standard_oauth_api};
use common::{EpisodeID, LikeStatus, PodcastChannelID, PodcastChannelParams, PodcastID, VideoID};
use futures::{StreamExt, TryStreamExt};
use std::time::Duration;
use utils::get_oauth_client_id_and_secret;
use ytmapi_rs::auth::*;
use ytmapi_rs::common::{
    ApiOutcome, ArtistChannelID, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary,
    PlaylistID, UserChannelID, YoutubeID,
};
use ytmapi_rs::error::ErrorKind;
use ytmapi_rs::query::playlist::{GetPlaylistDetailsQuery, PrivacyStatus};
use ytmapi_rs::query::search::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    PlaylistsFilter, PodcastsFilter, ProfilesFilter, SongsFilter, VideosFilter,
};
use ytmapi_rs::query::*;
use ytmapi_rs::*;

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
    let (client_id, _) = get_oauth_client_id_and_secret().unwrap();
    let _code = OAuthTokenGenerator::new(&client, client_id).await.unwrap();
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
generate_stream_test_logged_in!(
    test_stream_get_library_songs,
    GetLibrarySongsQuery::default()
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_artist_subscriptions,
    GetLibraryArtistSubscriptionsQuery::default()
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_playlists,
    GetLibraryPlaylistsQuery
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_albums,
    GetLibraryAlbumsQuery::default()
);
generate_stream_test_logged_in!(
    test_stream_get_library_artists,
    GetLibraryArtistsQuery::default()
);
generate_stream_test_logged_in!(
    test_stream_get_library_podcasts,
    GetLibraryPodcastsQuery::default()
);
generate_stream_test_logged_in!(
    test_stream_get_library_channels,
    GetLibraryChannelsQuery::default()
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_upload_songs,
    GetLibraryUploadSongsQuery::default()
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_upload_albums,
    GetLibraryUploadAlbumsQuery::default()
);
generate_stream_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_stream_get_library_upload_artists,
    GetLibraryUploadArtistsQuery::default()
);
generate_stream_test!(
    test_stream_search_artists,
    SearchQuery::new("Beatles").with_filter(ArtistsFilter)
);
generate_stream_test!(
    test_stream_search_songs,
    SearchQuery::new("Beatles").with_filter(SongsFilter)
);
generate_stream_test!(
    test_stream_search_albums,
    SearchQuery::new("Beatles").with_filter(AlbumsFilter)
);
generate_stream_test!(
    test_stream_search_videos,
    SearchQuery::new("Beatles").with_filter(VideosFilter)
);
generate_stream_test!(
    test_stream_search_episodes,
    SearchQuery::new("Beatles").with_filter(EpisodesFilter)
);
generate_stream_test!(
    test_stream_search_podcasts,
    SearchQuery::new("Beatles").with_filter(PodcastsFilter)
);
generate_stream_test!(
    test_stream_search_profiles,
    SearchQuery::new("Beatles").with_filter(ProfilesFilter)
);
generate_stream_test!(
    test_stream_search_featured_playlists,
    SearchQuery::new("Beatles").with_filter(FeaturedPlaylistsFilter)
);
generate_stream_test!(
    test_stream_search_community_playlists,
    SearchQuery::new("Beatles").with_filter(CommunityPlaylistsFilter)
);
generate_stream_test!(
    test_stream_search_playlists,
    SearchQuery::new("Beatles").with_filter(PlaylistsFilter)
);
generate_stream_test!(
    test_stream_get_playlist,
    GetPlaylistTracksQuery::new(PlaylistID::from_raw("VLPL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu"))
);
generate_stream_test!(
    test_stream_get_watch_playlist,
    GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw("9mWr4c_ig54"))
);

//// BASIC QUERY TESTS
generate_query_test!(
    test_search_suggestions,
    GetSearchSuggestionsQuery::new("faded")
);

generate_query_test!(test_get_mood_categories, GetMoodCategoriesQuery);
// NOTE: Set Taste Profile test is not implemented, to avoid impact to my YTM
// recommendations.
generate_query_test!(test_get_taste_profile, GetTasteProfileQuery);
generate_query_test_logged_in!(test_get_history, GetHistoryQuery);
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
        PodcastChannelID::from_raw("UCupvZG-5ko_eiXAupbDfxWw"),
        PodcastChannelParams::from_raw("6gPiAUdxWUJXcFlCQ3BNQkNpUjVkRjl3WVdkbFgzTnVZWEJ6YUc5MFgyMTFjMmxqWDNCaFoyVmZjbVZuYVc5dVlXd1NIM05mUzNKVGJtWlphemhuWmtWUWEzaDRSRVpqWWxSS0xXODNXVUprUW1zYVNnQUFaVzRBQVVGVkFBRkJWUUFCQUVaRmJYVnphV05mWkdWMFlXbHNYMkZ5ZEdsemRBQUJBVU1BQUFFQUFBRUJBRlZEZFhCMldrY3ROV3R2WDJWcFdFRjFjR0pFWm5oWGR3QUI4dHF6cWdvSFFBQklBRkMwQVE%3D")
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
    test_get_artist,
    GetArtistQuery::new(ArtistChannelID::from_raw("UC2XdaAVUannpujzv32jcouQ",))
);
generate_query_test_logged_in!(
    #[ignore = "Ignored by default due to quota"]
    test_get_library_songs,
    GetLibrarySongsQuery::default()
);
generate_query_test_logged_in!(test_get_library_albums, GetLibraryAlbumsQuery::default());
generate_query_test_logged_in!(
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
    test_get_lyrics_id,
    GetLyricsIDQuery::new(VideoID::from_raw("lYBUbBu4W08"))
);
generate_query_test!(
    test_get_playlist_details,
    GetPlaylistDetailsQuery::new(PlaylistID::from_raw("VLPL0jp-uZ7a4g9FQWW5R_u0pz4yzV4RiOXu"))
);
generate_query_test!(
    test_get_user,
    GetUserQuery::new(UserChannelID::from_raw("UCj0boSvCVfTmO9JHlclA8eQ"))
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
    browser_api.query(query.clone()).await.unwrap();
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
    browser_api
        .stream(&query)
        .take(5)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
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
    browser_api.query(query.clone()).await.unwrap();
}

#[tokio::test]
async fn test_get_artist_albums() {
    let api = YtMusic::new_unauthenticated().await.unwrap();
    let q = GetArtistQuery::new(ArtistChannelID::from_raw(
        // Metallica
        "UCGexNm_Kw4rdQjLxmpb2EKw",
    ));
    let res = api.query(q).await.unwrap();
    let albums = res.top_releases.albums.unwrap();
    let params = albums.params.unwrap();
    let channel_id = albums.browse_id.unwrap();
    api.get_artist_albums(channel_id, params).await.unwrap();
}
#[tokio::test]
async fn test_get_user_videos() {
    let api = YtMusic::new_unauthenticated().await.unwrap();
    // Turbo
    let channel_id = UserChannelID::from_raw("UCus8EVJ7Oc9zINhs-fg8l1Q");
    let user = api.get_user(&channel_id).await.unwrap();
    api.get_user_videos(&channel_id, user.all_videos_params.unwrap())
        .await
        .unwrap();
}
#[tokio::test]
async fn test_get_user_playlists() {
    let api = YtMusic::new_unauthenticated().await.unwrap();
    // kamarillobrillo
    let channel_id = UserChannelID::from_raw("UCj0boSvCVfTmO9JHlclA8eQ");
    let user = api.get_user(&channel_id).await.unwrap();
    api.get_user_playlists(&channel_id, user.all_playlists_params.unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_get_artist_album_songs() {
    let api = YtMusic::new_unauthenticated().await.unwrap();
    let q = GetArtistQuery::new(ArtistChannelID::from_raw(
        // Metallica
        "UCGexNm_Kw4rdQjLxmpb2EKw",
    ));
    let res = api.query(q).await.unwrap();
    let albums = res.top_releases.albums.unwrap();
    let params = albums.params.unwrap();
    let channel_id = &albums.browse_id.unwrap();
    let q = GetArtistAlbumsQuery::new(ArtistChannelID::from_raw(channel_id.get_raw()), params);
    let res = api.query(q).await.unwrap();
    let browse_id = &res[0].browse_id;
    let q = GetAlbumQuery::new(browse_id.clone());
    api.query(q).await.unwrap();
}

// # STATEFUL TESTS

#[tokio::test]
#[ignore = "Ignored due to long running and stateful"]
async fn test_add_remove_upload_song() {
    // Google spends some time processing songs after they are uploaded.
    const UPLOAD_PROCESSING_DELAY: Duration = Duration::from_secs(60);
    let browser_api = crate::utils::new_standard_api().await.unwrap();
    let outcome = browser_api
        .upload_song("test_json/test_upload.mp3")
        .await
        .unwrap();
    assert_eq!(outcome, ApiOutcome::Success);
    tokio::time::sleep(UPLOAD_PROCESSING_DELAY).await;
    let uploads = browser_api.get_library_upload_songs().await.unwrap();
    let uploaded_song = uploads
        .into_iter()
        .find(|song| song.title == "Lukewarm Banjo")
        .unwrap();
    browser_api
        .delete_upload_entity(uploaded_song.entity_id)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "Ignored due to long running and stateful"]
async fn test_subscribe_unsubscribe_artists() {
    // Timeout to avoid flaky test
    const GET_ARTIST_TIMEOUT: Duration = Duration::from_secs(5);
    let browser_api = crate::utils::new_standard_api().await.unwrap();
    let artist_id_list =
        ["UCwMzxvcq8VmfclCG6QUTm7g", "UCMyqqExD7o8zVB5SDUhhuCQ"].map(ArtistChannelID::from_raw);
    let artist_1_subscribed_initial = browser_api
        .get_artist(&artist_id_list[0])
        .await
        .unwrap()
        .subscribed;
    let artist_2_subscribed_initial = browser_api
        .get_artist(&artist_id_list[1])
        .await
        .unwrap()
        .subscribed;
    browser_api
        .subscribe_artist(&artist_id_list[0])
        .await
        .unwrap();
    browser_api
        .subscribe_artist(&artist_id_list[1])
        .await
        .unwrap();
    tokio::time::sleep(GET_ARTIST_TIMEOUT).await;
    assert!(
        browser_api
            .get_artist(&artist_id_list[0])
            .await
            .unwrap()
            .subscribed
    );
    assert!(
        browser_api
            .get_artist(&artist_id_list[1])
            .await
            .unwrap()
            .subscribed
    );
    browser_api
        .unsubscribe_artists(&artist_id_list)
        .await
        .unwrap();
    tokio::time::sleep(GET_ARTIST_TIMEOUT).await;
    assert!(
        !browser_api
            .get_artist(&artist_id_list[0])
            .await
            .unwrap()
            .subscribed
    );
    assert!(
        !browser_api
            .get_artist(&artist_id_list[1])
            .await
            .unwrap()
            .subscribed
    );
    tokio::time::sleep(GET_ARTIST_TIMEOUT).await;
    if artist_1_subscribed_initial {
        browser_api
            .subscribe_artist(&artist_id_list[0])
            .await
            .unwrap();
    }
    if artist_2_subscribed_initial {
        browser_api
            .subscribe_artist(&artist_id_list[1])
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "Ignored due to long running and stateful"]
async fn test_add_remove_history_items() {
    // Timeout to avoid flaky test
    const GET_HISTORY_TIMEOUT: Duration = Duration::from_secs(10);
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
// TODO: Test to see if status can be queried after adding / removing.
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
        .map(|item| item.set_video_id);
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
    let res = api.get_library_artists().await.unwrap();
    assert!(!res.is_empty());
}
#[tokio::test]
async fn test_get_library_artists() {
    let api = new_standard_api().await.unwrap();
    let res = api.get_library_artists().await.unwrap();
    assert!(!res.is_empty());
}
#[tokio::test]
async fn test_get_lyrics() {
    // TODO: Make more generic
    let api = new_standard_api().await.unwrap();
    let id = api
        .get_lyrics_id(VideoID::from_raw("lYBUbBu4W08"))
        .await
        .unwrap();
    let res = api.get_lyrics(id).await.unwrap();
    assert!(res.lyrics.contains("You know the rules and so do I"));
    assert!(res.source.contains("Musixmatch"));
}
