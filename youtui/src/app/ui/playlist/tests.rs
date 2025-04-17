use super::Playlist;
use crate::{
    app::{
        structures::{ListSong, ListStatus},
        ui::{playlist::QueueState, ListSongID, PlayState},
    },
    async_rodio_sink::Stopped,
    config::Config,
};
use pretty_assertions::assert_eq;
use std::{rc::Rc, sync::OnceLock, time::Duration};
use ytmapi_rs::{
    auth::BrowserToken,
    common::{AlbumID, YoutubeID},
    parse::{AlbumSong, GetAlbum},
    query::GetAlbumQuery,
};

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
    let cfg = Config::default();
    let mut playlist = Playlist::new(&cfg).0;
    playlist.list.state = ListStatus::Loaded;
    let GetAlbum {
        title,
        year,
        mut tracks,
        ..
    } = get_dummy_album();
    playlist.list.add_raw_song(
        tracks.pop().unwrap(),
        Rc::new(title),
        Rc::new(AlbumID::from_raw("")),
        Rc::new(year),
        Rc::new(String::from("The Beatles")),
    );
    playlist
}
async fn assert_function_is_autoplay_queued(f: impl Fn(&mut Playlist, ListSongID)) {
    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::NotQueued;
    let input = p.clone();
    f(&mut p, ListSongID(0));
    test_is_autoplay_queued(input, ListSongID(0), p);

    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::Queued(ListSongID(0));
    let input = p.clone();
    f(&mut p, ListSongID(0));
    test_is_autoplay_queued(input, ListSongID(0), p);

    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::Queued(ListSongID(1));
    let input = p.clone();
    f(&mut p, ListSongID(0));
    test_is_autoplay_queued(input, ListSongID(0), p);
}
fn test_is_autoplay_queued(mut input: Playlist, s: ListSongID, output: Playlist) {
    match input.queue_status {
        QueueState::NotQueued => assert_eq!(input, output),
        QueueState::Queued(list_song_id) => {
            if list_song_id == s {
                input.queue_status = QueueState::NotQueued;
                assert_eq!(input, output)
            } else {
                assert_eq!(input, output)
            }
        }
    }
}
#[tokio::test]
async fn test_handle_autoplay_queued_when_other_queued() {
    let mut p = get_dummy_playlist().await;
    p.queue_status = QueueState::Queued(ListSongID(1));
    let output = p.clone();
    test_is_autoplay_queued(p, ListSongID(0), output);
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
