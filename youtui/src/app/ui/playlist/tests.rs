use super::Playlist;
use crate::{
    app::ui::{ListSongID, PlayState},
    async_rodio_sink::Stopped,
    config::Config,
};

fn get_dummy_playlist() -> Playlist {
    let cfg = Config::default();
    Playlist::new(&cfg).0
}
#[tokio::test]
async fn test_handle_resumed_when_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_resumed(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Playing(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_other_song_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(1));
    p.handle_resumed(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Paused(ListSongID(1));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_other_state() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Error(ListSongID(0));
    p.handle_resumed(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Error(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_stopped_none() {
    let mut p = get_dummy_playlist();
    p.handle_stopped(None);
    assert_eq!(p, get_dummy_playlist());
}
#[tokio::test]
async fn test_handle_stopped_when_playing_id() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Stopped;
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_stopped_when_not_playing_id() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(1));
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Playing(ListSongID(1));
    assert_eq!(p, expected_p);
}
