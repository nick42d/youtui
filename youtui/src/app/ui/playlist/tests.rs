use super::Playlist;
use crate::{
    app::ui::{ListSongID, PlayState},
    config::Config,
};

fn get_dummy_playlist() -> Playlist {
    let cfg = Config::default();
    Playlist::new(&cfg).0
}
#[tokio::test]
async fn test_handle_resumed_when_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID::default());
    p.handle_resumed(ListSongID::default());
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Playing(ListSongID::default());
    assert_eq!(p, expected_p);
}
#[tokio::test]
async fn test_handle_resumed_when_other_state() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Error(ListSongID::default());
    p.handle_resumed(ListSongID::default());
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Error(ListSongID::default());
    assert_eq!(p, expected_p);
}
