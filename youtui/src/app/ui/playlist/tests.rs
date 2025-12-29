use super::Playlist;
use crate::app::server::{Stop, TaskMetadata};
use crate::app::structures::ListStatus;
use crate::app::ui::playlist::QueueState;
use crate::app::ui::{ListSongID, PlayState};
use crate::async_rodio_sink::{AllStopped, Stopped};
use async_callback_manager::{AsyncTask, Constraint, MaybePartialEq};
use pretty_assertions::assert_eq;
use std::sync::OnceLock;
use std::time::Duration;
use ytmapi_rs::auth::BrowserToken;
use ytmapi_rs::common::{AlbumID, YoutubeID};
use ytmapi_rs::parse::{GetAlbum, ParsedSongAlbum};
use ytmapi_rs::query::GetAlbumQuery;

static DUMMY_ALBUM: OnceLock<GetAlbum> = OnceLock::new();

fn get_dummy_album() -> GetAlbum {
    DUMMY_ALBUM
        .get_or_init(|| {
            let json =
                std::fs::read_to_string("../ytmapi-rs/test_json/get_album_20240724.json").unwrap();
            ytmapi_rs::process_json::<_, BrowserToken>(
                json,
                GetAlbumQuery::new(AlbumID::from_raw("")),
            )
            .unwrap()
        })
        .clone()
}

async fn get_dummy_playlist() -> Playlist {
    let mut playlist = Playlist::new().0;
    playlist.list.state = ListStatus::Loaded;
    let GetAlbum {
        title,
        year,
        tracks,
        ..
    } = get_dummy_album();
    playlist.list.append_raw_album_songs(
        tracks,
        ParsedSongAlbum {
            name: title,
            id: AlbumID::from_raw(""),
        },
        year,
        vec![],
        vec![],
    );
    playlist
}
#[tokio::test]
async fn test_reset_when_playing_stops_song_id() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(1));
    let effect = p.reset();
    let expected_effect = AsyncTask::new_future_with_closure_handler(
        Stop(ListSongID(1)),
        Playlist::handle_stopped,
        Some(Constraint::new_block_matching_metadata(
            TaskMetadata::PlayPause,
        )),
    );
    assert_eq!(effect.maybe_eq(expected_effect), Some(true));
}
#[tokio::test]
async fn test_reset_when_not_playing_has_no_effect() {
    let mut p = get_dummy_playlist().await;
    let effect = p.reset();
    assert_eq!(effect.maybe_eq(AsyncTask::new_no_op()), Some(true));
}
#[tokio::test]
async fn test_handle_autoplay_queued_when_other_queued() {
    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::Queued(ListSongID(1));
    let expected_p = p.clone();
    p.handle_autoplay_queued(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_autoplay_queued_when_queued() {
    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::Queued(ListSongID(0));
    p.handle_autoplay_queued(ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.queue_status = QueueState::NotQueued;
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_autoplay_queued_not_queued() {
    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::NotQueued;
    let expected_p = p.clone();
    p.handle_autoplay_queued(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_playing_modifies_duration() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(1));
    let new_duration = Duration::from_secs(180);
    p.handle_playing(Some(new_duration), ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Paused(ListSongID(1));
    expected_p
        .list
        .get_list_iter_mut()
        .next()
        .unwrap()
        .actual_duration = Some(new_duration);
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_queued_modifies_duration() {
    let mut p = get_dummy_playlist().await;
    let new_duration = Duration::from_secs(180);
    p.handle_queued(Some(new_duration), ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p
        .list
        .get_list_iter_mut()
        .next()
        .unwrap()
        .actual_duration = Some(new_duration);
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_playing_no_duration_when_paused() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_playing(None, ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Playing(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_playing_no_duration_when_other_song_paused() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(1));
    let expected_p = p.clone();
    p.handle_playing(None, ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_playing_no_duration_when_other_state() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_playing(None, ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_set_to_error() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_set_to_error(ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Error(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_paused() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_resumed(ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Playing(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_other_song_paused() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Paused(ListSongID(1));
    let expected_p = p.clone();
    p.handle_resumed(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_other_state() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_resumed(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_paused_when_playing() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_paused(ListSongID(0));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_paused_when_other_song_playing() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(1));
    let expected_p = p.clone();
    p.handle_paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_paused_when_other_state() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_stopped_none() {
    let mut p = get_dummy_playlist().await;
    p.handle_stopped(None);
    assert_eq!(p, get_dummy_playlist().await);
}
#[tokio::test]
async fn test_handle_stopped_when_playing_id() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Stopped;
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_stopped_when_not_playing_id() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(1));
    let expected_p = p.clone();
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_all_stopped_none() {
    let mut p = get_dummy_playlist().await;
    p.handle_all_stopped(None);
    assert_eq!(p, get_dummy_playlist().await);
}
#[tokio::test]
async fn test_handle_all_stopped_when_playing() {
    let mut p = get_dummy_playlist().await;
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_all_stopped(Some(AllStopped));
    let mut expected_p = get_dummy_playlist().await;
    expected_p.play_status = PlayState::Stopped;
    assert_eq!(p, expected_p);
}
