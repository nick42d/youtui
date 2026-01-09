use super::Playlist;
use crate::app::server::song_downloader::InMemSong;
use crate::app::server::song_thumbnail_downloader::SongThumbnailID;
use crate::app::server::{DecodeSong, GetSongThumbnail, PlayDecodedSong, Stop, TaskMetadata};
use crate::app::structures::{ListSong, ListStatus, MaybeRc};
use crate::app::ui::playlist::{
    HandleGetSongThumbnailError, HandleGetSongThumbnailOk, HandlePlayUpdateError,
    HandlePlayUpdateOk, HandleStopped, QueueState,
};
use crate::app::ui::{ListSongID, PlayState};
use crate::async_rodio_sink::{AllStopped, Stopped};
use async_callback_manager::{AsyncTask, Constraint, TryBackendTaskExt};
use pretty_assertions::assert_eq;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use ytmapi_rs::auth::BrowserToken;
use ytmapi_rs::common::{AlbumID, Thumbnail, YoutubeID};
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

fn get_dummy_playlist() -> Playlist {
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
#[test]
fn newly_added_song_downloads_album_art() {
    let mut p = get_dummy_playlist();
    let s = p.list.get_list_iter_mut().next().unwrap();
    s.thumbnails = MaybeRc::Owned(vec![Thumbnail {
        height: 0,
        width: 0,
        url: "dummy_url".to_string(),
    }]);
    let dummy_song = s.clone();
    let thumbnail_id = SongThumbnailID::from(&dummy_song as &ListSong).into_owned();
    let (_, effect) = p.push_song_list(vec![dummy_song]);
    let expected_effect = AsyncTask::new_future_try(
        GetSongThumbnail {
            thumbnail_url: "dummy_url".to_string(),
            thumbnail_id: thumbnail_id.clone(),
        },
        HandleGetSongThumbnailOk,
        HandleGetSongThumbnailError(thumbnail_id),
        None,
    );
    assert!(
        effect
            .contains(&expected_effect)
            .is_some_and(std::convert::identity),
        "Expected Left to contain Right {}",
        pretty_assertions::Comparison::new(&effect, &expected_effect)
    );
}
#[test]
fn downloaded_song_plays_if_buffered() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Buffering(ListSongID(1));
    let dummy_song = Arc::new(InMemSong(vec![1]));
    p.list.get_list_iter_mut().nth(1).unwrap().download_status =
        crate::app::structures::DownloadStatus::Downloaded(dummy_song.clone());
    let effect = p.handle_song_downloaded(ListSongID(1));
    assert_eq!(p.play_status, PlayState::Playing(ListSongID(1)));
    let expected_effect = AsyncTask::new_stream_try(
        DecodeSong(dummy_song.clone()).map_stream(PlayDecodedSong(ListSongID(1))),
        HandlePlayUpdateOk,
        HandlePlayUpdateError(ListSongID(1)),
        Some(Constraint::new_block_matching_metadata(
            TaskMetadata::PlayingSong,
        )),
    );
    assert!(
        effect
            .maybe_contains(&expected_effect)
            .is_some_and(std::convert::identity),
        "Expected Left to contain Right {}",
        pretty_assertions::Comparison::new(&effect, &expected_effect)
    );
}
#[test]
fn test_reset_when_playing_stops_song_id() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(1));
    let effect = p.reset();
    let expected_effect = AsyncTask::new_future(
        Stop(ListSongID(1)),
        HandleStopped,
        Some(Constraint::new_block_matching_metadata(
            TaskMetadata::PlayPause,
        )),
    );
    assert_eq!(effect.maybe_eq(&expected_effect), Some(true));
}
#[test]
fn test_reset_when_not_playing_has_no_effect() {
    let mut p = get_dummy_playlist();
    let effect = p.reset();
    assert_eq!(effect.maybe_eq(&AsyncTask::new_no_op()), Some(true));
}
#[test]
fn test_handle_autoplay_queued_when_other_queued() {
    let mut p = get_dummy_playlist();
    p.queue_status = QueueState::Queued(ListSongID(1));
    let expected_p = p.clone();
    p.handle_autoplay_queued(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_autoplay_queued_when_queued() {
    let mut p = get_dummy_playlist();
    p.queue_status = QueueState::Queued(ListSongID(0));
    p.handle_autoplay_queued(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.queue_status = QueueState::NotQueued;
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_autoplay_queued_not_queued() {
    let mut p = get_dummy_playlist();
    p.queue_status = QueueState::NotQueued;
    let expected_p = p.clone();
    p.handle_autoplay_queued(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_playing_modifies_duration() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(1));
    let new_duration = Duration::from_secs(180);
    p.handle_playing(Some(new_duration), ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Paused(ListSongID(1));
    expected_p
        .list
        .get_list_iter_mut()
        .next()
        .unwrap()
        .actual_duration = Some(new_duration);
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_queued_modifies_duration() {
    let mut p = get_dummy_playlist();
    let new_duration = Duration::from_secs(180);
    p.handle_queued(Some(new_duration), ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p
        .list
        .get_list_iter_mut()
        .next()
        .unwrap()
        .actual_duration = Some(new_duration);
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_playing_no_duration_when_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_playing(None, ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Playing(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_playing_no_duration_when_other_song_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(1));
    let expected_p = p.clone();
    p.handle_playing(None, ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_playing_no_duration_when_other_state() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_playing(None, ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_set_to_error() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_set_to_error(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Error(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_resumed_when_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(0));
    p.handle_resumed(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Playing(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_resumed_when_other_song_paused() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Paused(ListSongID(1));
    let expected_p = p.clone();
    p.handle_resumed(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_resumed_when_other_state() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_resumed(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_paused_when_playing() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_paused(ListSongID(0));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_paused_when_other_song_playing() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(1));
    let expected_p = p.clone();
    p.handle_paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_paused_when_other_state() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Error(ListSongID(0));
    let expected_p = p.clone();
    p.handle_paused(ListSongID(0));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_stopped_none() {
    let mut p = get_dummy_playlist();
    p.handle_stopped(None);
    assert_eq!(p, get_dummy_playlist());
}
#[test]
fn test_handle_stopped_when_playing_id() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Stopped;
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_stopped_when_not_playing_id() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(1));
    let expected_p = p.clone();
    p.handle_stopped(Some(Stopped(ListSongID(0))));
    assert_eq!(p, expected_p);
}
#[test]
fn test_handle_all_stopped_none() {
    let mut p = get_dummy_playlist();
    p.handle_all_stopped(None);
    assert_eq!(p, get_dummy_playlist());
}
#[test]
fn test_handle_all_stopped_when_playing() {
    let mut p = get_dummy_playlist();
    p.play_status = PlayState::Playing(ListSongID(0));
    p.handle_all_stopped(Some(AllStopped));
    let mut expected_p = get_dummy_playlist();
    expected_p.play_status = PlayState::Stopped;
    assert_eq!(p, expected_p);
}
