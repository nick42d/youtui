use super::downloader::InMemSong;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::core::send_or_error;
use crate::Result;
use rodio_thread::rodio_mpsc_channel;
use rodio_thread::rodio_oneshot_channel;
use rodio_thread::spawn_rodio_thread;
use rodio_thread::PlayActionTaken;
use rodio_thread::PlaySongResponse;
use rodio_thread::RodioMessage;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;

mod rodio_thread;

const PLAYER_MSG_QUEUE_SIZE: usize = 256;
const PROGRESS_UPDATE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    Playing(Option<Duration>, ListSongID),
    Queued(Option<Duration>, ListSongID),
    AutoplayQueued(ListSongID),
    Error(ListSongID),
}

pub struct Player {
    rodio_tx: mpsc::Sender<RodioMessage>,
}

// Consider if this can be managed by Server.
impl Player {
    pub fn new() -> Self {
        let (msg_tx, msg_rx) = mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        spawn_rodio_thread(msg_rx);
        Self { rodio_tx: msg_tx }
    }
}
async fn seek(inc: i8, rodio_tx: mpsc::Sender<RodioMessage>) -> Option<ProgressUpdate> {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::Seek(inc, tx)).await;
    let Ok((current_duration, song_id)) = rx.await else {
        // This happens intentionally - when a seek is requested for a song
        // but all songs have finished, instead of sending a reply, rodio will drop
        // sender.
        info!("The song I tried to seek is no longer playing");
        return None;
    };
    Some(ProgressUpdate(current_duration, song_id))
}
async fn stop(song_id: ListSongID, rodio_tx: mpsc::Sender<RodioMessage>) -> Option<Stopped> {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::Stop(song_id, tx)).await;
    let Ok(_) = rx.await else {
        // This happens intentionally - when a stop is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!("The song I tried to stop is no longer playing");
        return None;
    };
    Some(Stopped(song_id))
}
async fn pause_play(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
) -> Option<PausePlayResponse> {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::PausePlay(song_id, tx)).await;
    let Ok(play_action_taken) = rx.await else {
        // This happens intentionally - when a pauseplay is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!("The song I tried to pause/play was no longer selected",);
        return None;
    };
    match play_action_taken {
        PlayActionTaken::Paused => Some(PausePlayResponse::Paused(song_id)),
        PlayActionTaken::Played => Some(PausePlayResponse::Resumed(song_id)),
    }
}
async fn increase_volume(
    vol_inc: i8,
    rodio_tx: mpsc::Sender<RodioMessage>,
) -> Option<VolumeUpdate> {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::IncreaseVolume(vol_inc, tx)).await;
    let Ok(current_volume) = rx.await else {
        // Should never happen!
        error!("The player has been dropped while I was waiting for a volume update for",);
        return None;
    };
    Some(VolumeUpdate(current_volume))
}
