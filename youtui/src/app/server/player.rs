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
    // At this stage this difference between DonePlaying and Stopped is very thin. DonePlaying
    // means that the song has been dropped by the player, whereas Stopped simply means that a
    // Stop message to the player was succesful.
    DonePlaying(ListSongID),
    Playing(Option<Duration>, ListSongID),
    Queued(Option<Duration>, ListSongID),
    AutoplayQueued(ListSongID),
    Error(ListSongID),
}

pub struct VolumeUpdate(Percentage);
pub struct ProgressUpdate(Duration, ListSongID);
pub struct Stopped(ListSongID);
pub enum PausePlayResponse {
    Paused(ListSongID),
    Resumed(ListSongID),
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

#[cfg(FALSE)]
async fn autoplay_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
    send_or_error(
        rodio_tx,
        RodioMessage::AutoplaySong(song_pointer, song_id, tx),
    )
    .await;
    while let Some(msg) = rx.recv().await {
        match msg {
            PlaySongResponse::ProgressUpdate(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::ProgressUpdate(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::Queued(_) => {
                error!("Received queued message, but I wasn't queued... {:?}", id)
            }
            // This is the case where the song we asked to play is already
            // queued. In this case, this task can finish, as the task that
            // added the song to the queue is responsible for the playback
            // updates.
            PlaySongResponse::AutoplayingQueued => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::AutoplayQueued(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::StartedPlaying(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Playing(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::StoppedPlaying => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::DonePlaying(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::Error(e) => {
                error!("Received error {e} when trying to decode song");
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Error(song_id)),
                )
                .await;
                return;
            }
        }
    }
    // Should never reach here! Player should send either Error, Stopped or Playing
    // message first.
    error!(
        "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
        id
    );
}
#[cfg(FALSE)]
async fn queue_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
    send_or_error(rodio_tx, RodioMessage::QueueSong(song_pointer, song_id, tx)).await;
    while let Some(msg) = rx.recv().await {
        match msg {
            PlaySongResponse::ProgressUpdate(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::ProgressUpdate(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::StartedPlaying(_) => {
                error!(
                    "Received StartedPlaying message, but I asked to queue... {:?}",
                    id
                )
            }
            PlaySongResponse::Queued(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Queued(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::StoppedPlaying => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::DonePlaying(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::Error(e) => {
                error!("Received error {e} when trying to decode song");
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Error(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::AutoplayingQueued => error!(
                "Received AutoplayingQueued message, but I asked to queue... {:?}",
                id
            ),
        }
    }
    // Should never reach here! Player should send either Error, Stopped or Queued
    // message first.
    error!(
        "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
        id
    );
}
#[cfg(FALSE)]
async fn play_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
    send_or_error(rodio_tx, RodioMessage::PlaySong(song_pointer, song_id, tx)).await;
    while let Some(msg) = rx.recv().await {
        match msg {
            PlaySongResponse::ProgressUpdate(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::ProgressUpdate(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::Queued(_) => {
                error!("Received queued message, but I wasn't queued... {:?}", id)
            }
            PlaySongResponse::StartedPlaying(duration) => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Playing(duration, song_id)),
                )
                .await;
            }
            PlaySongResponse::StoppedPlaying => {
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::DonePlaying(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::Error(e) => {
                error!("Received error {e} when trying to decode song");
                send_or_error(
                    response_tx.clone(),
                    ServerResponse::new_player(id, Response::Error(song_id)),
                )
                .await;
                return;
            }
            PlaySongResponse::AutoplayingQueued => error!(
                "Received AutoplayingQueued message, but I asked to play... {:?}",
                id
            ),
        }
    }
    // Should never reach here! Player should send either Error, Stopped or Playing
    // message first.
    error!(
        "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
        id
    );
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
