use super::footer::parse_simple_time_to_secs;
use super::YoutuiWindow;
use crate::app::media_controls::{MediaControlsStatus, MediaControlsUpdate, MediaControlsVolume};
use crate::app::structures::PlayState;
use itertools::Itertools;
use std::time::Duration;

pub fn draw_app_media_controls(w: &YoutuiWindow) -> MediaControlsUpdate<'_> {
    let cur = &w.playlist.play_status;
    let mut duration = 0;
    let mut progress = Duration::default();
    match cur {
        PlayState::Playing(id) | PlayState::Paused(id) => {
            duration = w
                .playlist
                .get_song_from_id(*id)
                .map(|s| &s.duration_string)
                .map(parse_simple_time_to_secs)
                .unwrap_or(0);
            progress = w.playlist.cur_played_dur.unwrap_or_default();
            (progress.as_secs_f64() / duration as f64).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    let cur = &w.playlist.play_status;
    let song_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .map(|s| s.title.as_ref())
            .unwrap_or("No title"),
        PlayState::NotPlaying => "Not playing",
        PlayState::Stopped => "Not playing",
    };
    let album_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .and_then(|s| s.album.as_ref())
            .map(|s| s.name.as_str())
            .unwrap_or_default(),
        PlayState::NotPlaying => "",
        PlayState::Stopped => "",
    };
    let album_art_path = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .and_then(|s| s.album_art.as_ref())
            .map(|s| format!("file://{}", &s.on_disk_path.display())),
        PlayState::NotPlaying => None,
        PlayState::Stopped => None,
    };
    let artist_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .map(|s| s.artists.as_ref())
            .map(|s| {
                Itertools::intersperse(s.iter().map(|s| s.name.as_str()), ", ").collect::<String>()
            })
            .unwrap_or("".to_string())
            .into(),
        PlayState::NotPlaying => "".into(),
        PlayState::Stopped => "".into(),
    };
    let playback_status = match cur {
        PlayState::Playing(_) => MediaControlsStatus::Playing { progress },
        PlayState::Paused(_) => MediaControlsStatus::Paused { progress },
        _ => MediaControlsStatus::Stopped,
    };
    let volume = MediaControlsVolume::from_percentage_clamped(w.playlist.volume);
    MediaControlsUpdate {
        title: Some(song_title.into()),
        album: Some(album_title.into()),
        artist: Some(artist_title),
        cover_url: album_art_path.map(Into::into),
        duration: Some(std::time::Duration::from_secs(duration as u64)),
        playback_status,
        volume,
    }
}
