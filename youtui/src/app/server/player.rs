use super::downloader::InMemSong;
use super::messages::ServerResponse;
use super::spawn_unkillable;
use super::KillableTask;
use super::ServerComponent;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::app::taskmanager::TaskID;
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
// NOTE: I considered giving player more control of the playback than playlist,
// and increasing message size. However this seems to be more combinatorially
// difficult without a well defined data structure.
pub enum UnkillableServerRequest {
    IncreaseVolume(i8),
    // Play a song, starting from the start, regardless what's queued.
    PlaySong(Arc<InMemSong>, ListSongID),
    // Play a song, unless it's already queued.
    AutoplaySong(Arc<InMemSong>, ListSongID),
    // Queue a song to play next.
    QueueSong(Arc<InMemSong>, ListSongID),
    Stop(ListSongID),
    PausePlay(ListSongID),
    Seek(i8),
}

#[derive(Debug)]
pub enum KillableServerRequest {}

#[derive(Debug)]
pub enum Response {
    // At this stage this difference between DonePlaying and Stopped is very thin. DonePlaying
    // means that the song has been dropped by the player, whereas Stopped simply means that a
    // Stop message to the player was succesful.
    DonePlaying(ListSongID),
    Stopped(ListSongID),
    Paused(ListSongID),
    Resumed(ListSongID),
    Playing(Option<Duration>, ListSongID),
    Queued(Option<Duration>, ListSongID),
    AutoplayQueued(ListSongID),
    Error(ListSongID),
    ProgressUpdate(Duration, ListSongID),
    VolumeUpdate(Percentage),
}

pub struct Player {
    response_tx: mpsc::Sender<ServerResponse>,
    rodio_tx: mpsc::Sender<RodioMessage>,
}

// Consider if this can be managed by Server.
impl Player {
    pub fn new(response_tx: mpsc::Sender<ServerResponse>) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        spawn_rodio_thread(msg_rx);
        Self {
            response_tx,
            rodio_tx: msg_tx,
        }
    }
}

impl ServerComponent for Player {
    type KillableRequestType = KillableServerRequest;
    type UnkillableRequestType = UnkillableServerRequest;
    async fn handle_killable_request(
        &self,
        request: Self::KillableRequestType,
        task: KillableTask,
    ) -> Result<()> {
        let KillableTask { id: _, kill_rx: _ } = task;
        match request {}
    }
    async fn handle_unkillable_request(
        &self,
        request: Self::UnkillableRequestType,
        task: TaskID,
    ) -> Result<()> {
        let rodio_tx = self.rodio_tx.clone();
        let response_tx = self.response_tx.clone();
        match request {
            UnkillableServerRequest::IncreaseVolume(vol_inc) => {
                spawn_unkillable(increase_volume(vol_inc, rodio_tx, task, response_tx))
            }
            UnkillableServerRequest::PlaySong(song_pointer, song_id) => spawn_unkillable(
                play_song(song_pointer, song_id, rodio_tx, task, response_tx),
            ),
            UnkillableServerRequest::AutoplaySong(song_pointer, song_id) => spawn_unkillable(
                autoplay_song(song_pointer, song_id, rodio_tx, task, response_tx),
            ),
            UnkillableServerRequest::QueueSong(song_pointer, song_id) => spawn_unkillable(
                queue_song(song_pointer, song_id, rodio_tx, task, response_tx),
            ),
            UnkillableServerRequest::Stop(song_id) => {
                spawn_unkillable(stop(song_id, rodio_tx, task, response_tx))
            }
            UnkillableServerRequest::PausePlay(song_id) => {
                spawn_unkillable(pause_play(song_id, rodio_tx, task, response_tx))
            }
            UnkillableServerRequest::Seek(inc) => {
                spawn_unkillable(seek(inc, rodio_tx, task, response_tx))
            }
        }
        Ok(())
    }
}

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

async fn seek(
    inc: i8,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::Seek(inc, tx)).await;
    let Ok((current_duration, song_id)) = rx.await else {
        // This happens intentionally - when a seek is requested for a song
        // but all songs have finished, instead of sending a reply, rodio will drop
        // sender.
        info!("The song I tried to seek is no longer playing {:?}", id);
        return;
    };
    send_or_error(
        response_tx,
        ServerResponse::new_player(id, Response::ProgressUpdate(current_duration, song_id)),
    )
    .await;
}
async fn stop(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::Stop(song_id, tx)).await;
    let Ok(_) = rx.await else {
        // This happens intentionally - when a stop is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!("The song I tried to stop is no longer playing {:?}", id);
        return;
    };
    send_or_error(
        response_tx,
        ServerResponse::new_player(id, Response::Stopped(song_id)),
    )
    .await;
}
async fn pause_play(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::PausePlay(song_id, tx)).await;
    let Ok(play_action_taken) = rx.await else {
        // This happens intentionally - when a pauseplay is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!(
            "The song I tried to pause/play was no longer selected {:?}",
            id
        );
        return;
    };
    match play_action_taken {
        PlayActionTaken::Paused => {
            send_or_error(
                response_tx,
                ServerResponse::new_player(id, Response::Paused(song_id)),
            )
            .await
        }
        PlayActionTaken::Played => {
            send_or_error(
                response_tx,
                ServerResponse::new_player(id, Response::Resumed(song_id)),
            )
            .await
        }
    }
}
async fn increase_volume(
    vol_inc: i8,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_oneshot_channel();
    send_or_error(rodio_tx, RodioMessage::IncreaseVolume(vol_inc, tx)).await;
    let Ok(current_volume) = rx.await else {
        // Should never happen!
        error!(
            "The player has been dropped while I was waiting for a volume update for {:?}",
            id
        );
        return;
    };
    send_or_error(
        response_tx,
        ServerResponse::new_player(id, Response::VolumeUpdate(current_volume)),
    )
    .await;
}
