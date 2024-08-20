use super::spawn_run_or_kill;
use super::spawn_unkillable;
use super::KillableTask;
use super::ServerComponent;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::app::taskmanager::TaskID;
use crate::core::blocking_send_or_error;
use crate::core::send_or_error;
use crate::Result;
use rodio::decoder::DecoderError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::warn;

const EVENT_POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(10);
const PROGRESS_UPDATE_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(100);
const PLAYER_MSG_QUEUE_SIZE: usize = 256;

#[derive(Debug)]
pub enum UnkillableServerRequest {
    IncreaseVolume(i8),
    PlaySong(Arc<Vec<u8>>, ListSongID),
    GetPlayProgress(ListSongID),
    Stop(ListSongID),
    PausePlay(ListSongID),
}

#[derive(Debug)]
pub enum KillableServerRequest {
    GetVolume(),
}

#[derive(Debug)]
enum RodioMessage {
    PlaySong(
        Arc<Vec<u8>>,
        ListSongID,
        oneshot::Sender<std::result::Result<(), DecoderError>>,
    ),
    GetPlayProgress(ListSongID, oneshot::Sender<Duration>),
    Stop(ListSongID, oneshot::Sender<()>),
    PausePlay(ListSongID, oneshot::Sender<PlayActionTaken>),
    IncreaseVolume(i8, oneshot::Sender<Percentage>),
    GetVolume(oneshot::Sender<Percentage>),
}

#[derive(Debug)]
/// The action rodio took when it received a PausePlay message.
enum PlayActionTaken {
    Paused,
    Played,
}

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    Paused(ListSongID, TaskID),
    Playing(ListSongID, TaskID),
    Stopped(ListSongID, TaskID),
    Error(ListSongID, TaskID),
    ProgressUpdate(Duration, ListSongID, TaskID),
    VolumeUpdate(Percentage, TaskID),
}

pub struct PlayerManager {
    response_tx: mpsc::Sender<super::Response>,
    rodio_tx: mpsc::Sender<RodioMessage>,
}

// Consider if this can be managed by Server.
impl PlayerManager {
    pub fn new(response_tx: mpsc::Sender<super::Response>) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let response_tx_clone = response_tx.clone();
        spawn_rodio_thread(msg_rx, response_tx_clone);
        Self {
            response_tx,
            rodio_tx: msg_tx,
        }
    }
}

impl ServerComponent for PlayerManager {
    type KillableRequestType = KillableServerRequest;
    type UnkillableRequestType = UnkillableServerRequest;
    async fn handle_killable_request(
        &self,
        request: Self::KillableRequestType,
        task: KillableTask,
    ) -> Result<()> {
        let KillableTask { id, kill_rx } = task;
        match request {
            KillableServerRequest::GetVolume() => spawn_run_or_kill(
                get_volume(self.rodio_tx.clone(), id, self.response_tx.clone()),
                kill_rx,
            ),
        }
        Ok(())
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
            UnkillableServerRequest::GetPlayProgress(song_id) => {
                spawn_unkillable(get_play_progress(song_id, rodio_tx, task, response_tx))
            }
            UnkillableServerRequest::Stop(song_id) => {
                spawn_unkillable(stop(song_id, rodio_tx, task, response_tx))
            }
            UnkillableServerRequest::PausePlay(song_id) => {
                spawn_unkillable(pause_play(song_id, rodio_tx, task, response_tx))
            }
        }
        Ok(())
    }
}

pub fn spawn_rodio_thread(
    mut msg_rx: mpsc::Receiver<RodioMessage>,
    response_tx: mpsc::Sender<super::Response>,
) {
    tokio::task::spawn_blocking(move || {
        // Rodio can produce output to stderr when we don't want it to, so we use Gag to
        // suppress stdout/stderr. The downside is that even though this runs in
        // a seperate thread all stderr for the whole app may be gagged.
        // Also seems to spew out characters?
        // TODO: Try to handle the errors from Rodio or write to a file.
        // Allow unused - this is a guard we want to hold for the whole block.
        #[allow(unused)]
        let gag = match gag::Gag::stderr() {
            Ok(gag) => gag,
            Err(e) => {
                warn!("Error <{e}> gagging stderr output");
                return;
            }
        };
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        // Hopefully someone else can't create a song with the same ID?!
        let mut cur_song_id = ListSongID::default();
        let mut thinks_is_playing = false;
        loop {
            while let Ok(msg) = msg_rx.try_recv() {
                info!("Rodio received {:?}", msg);
                match msg {
                    RodioMessage::PlaySong(song_pointer, song_id, tx) => {
                        // XXX: Perhaps should let the state know that we are playing.
                        // TODO: remove allocation
                        let owned_song =
                            Arc::try_unwrap(song_pointer).unwrap_or_else(|arc| (*arc).clone());
                        let cur = std::io::Cursor::new(owned_song);
                        let source = match rodio::Decoder::new(cur) {
                            Ok(source) => source,
                            Err(e) => {
                                error!("Received error when trying to play song <{e}>");
                                if !sink.empty() {
                                    sink.stop()
                                }
                                tx.send(Err(e));
                                thinks_is_playing = false;
                                continue;
                            }
                        };
                        if !sink.empty() {
                            sink.stop()
                        }
                        sink.append(source);
                        // Handle case were we've received a play message but queue was paused.
                        if sink.is_paused() {
                            sink.play();
                        }
                        debug!("Now playing {:?}", song_id);
                        // Send the Now Playing message for good orders sake to avoid
                        // synchronization issues.
                        tx.send(Ok(()));
                        cur_song_id = song_id;
                        thinks_is_playing = true;
                    }
                    RodioMessage::Stop(song_id, tx) => {
                        info!("Got message to stop playing {:?}", song_id);
                        if cur_song_id != song_id {
                            continue;
                        }
                        if !sink.empty() {
                            sink.stop()
                        }
                        tx.send(());
                        thinks_is_playing = false;
                    }
                    RodioMessage::PausePlay(song_id, tx) => {
                        info!("Got message to pause / play {:?}", song_id);
                        if cur_song_id != song_id {
                            continue;
                        }
                        if sink.is_paused() {
                            sink.play();
                            info!("Sending Play message {:?}", song_id);
                            tx.send(PlayActionTaken::Played);
                        // We don't want to pause if sink is empty (but case
                        // could be handled in Playlist also)
                        } else if !sink.is_paused() && !sink.empty() {
                            sink.pause();
                            info!("Sending Pause message {:?}", song_id);
                            tx.send(PlayActionTaken::Paused);
                        }
                    }
                    // XXX: May be able to handle this by reporting progress updates when playing
                    // instead of needing to request/response here.
                    RodioMessage::GetPlayProgress(song_id, tx) => {
                        info!("Got message to provide song progress update");
                        if cur_song_id == song_id {
                            tx.send(sink.get_pos());
                            info!("Rodio sent song progress update");
                        } else {
                            info!(
                                "Rodio didn't send song progress update, it was no longer playing"
                            );
                            drop(tx)
                        }
                    }
                    // XXX: Should this just be IncreaseVolume(0)?
                    RodioMessage::GetVolume(tx) => {
                        tx.send(Percentage((sink.volume() * 100.0).round() as u8));
                        info!("Rodio sent volume update");
                    }
                    RodioMessage::IncreaseVolume(vol_inc, tx) => {
                        sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                        tx.send(Percentage((sink.volume() * 100.0).round() as u8));
                        info!("Rodio sent volume update");
                    }
                }
            }
            // Avoid empty infinite loop, but still poll more frequently than when sending
            // progress updates for responsiveness. TODO: Maintain the
            // responsiveness whilst still sending progress updates.
            // TODO: Remove this mechanic, but we will need to decide how we keep track of
            // playing songs.
            std::thread::sleep(EVENT_POLL_INTERVAL);
            if !sink.empty() && !sink.is_paused() {
                std::thread::sleep(PROGRESS_UPDATE_INTERVAL.saturating_sub(EVENT_POLL_INTERVAL));
            }
            if sink.empty() && thinks_is_playing {
                // NOTE: This simple model won't work if we have multiple songs in the sink.
                // Instead we should keep track of number of songs and use sink.len().
                trace!("Finished playing {:?}", cur_song_id);
                thinks_is_playing = false;
                blocking_send_or_error(
                    &response_tx,
                    super::Response::Player(Response::DonePlaying(cur_song_id)),
                );
            }
        }
    });
}

async fn play_song(
    song_pointer: Arc<Vec<u8>>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    send_or_error(rodio_tx, RodioMessage::PlaySong(song_pointer, song_id, tx)).await;
    let Ok(play_song_outcome) = rx.await else {
        // Should never happen!
        error!(
            "The player has been dropped while I was waiting for a volume update for {:?}",
            id
        );
        return;
    };
    match play_song_outcome {
        Ok(()) => send_or_error(
            response_tx,
            super::Response::Player(Response::Playing(song_id, id)),
        ),
        Err(_) => send_or_error(
            response_tx,
            super::Response::Player(Response::Error(song_id, id)),
        ),
    }
    .await;
}
async fn get_play_progress(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    send_or_error(rodio_tx, RodioMessage::GetPlayProgress(song_id, tx)).await;
    let Ok(current_duration) = rx.await else {
        // This happens intentionally - when a play update is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!(
            "The player has been dropped while I was waiting for a volume update for {:?}",
            id
        );
        return;
    };
    send_or_error(
        response_tx,
        super::Response::Player(Response::ProgressUpdate(current_duration, song_id, id)),
    )
    .await;
}
async fn stop(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    send_or_error(rodio_tx, RodioMessage::Stop(song_id, tx)).await;
    let Ok(_) = rx.await else {
        // This happens intentionally - when a stop is requested for a song
        // that's no longer playing, instead of sending a reply, rodio will drop sender.
        info!("The song I tried to stop is no longer playing {:?}", id);
        return;
    };
    send_or_error(
        response_tx,
        super::Response::Player(Response::Stopped(song_id, id)),
    )
    .await;
}
async fn pause_play(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
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
                super::Response::Player(Response::Paused(song_id, id)),
            )
            .await
        }
        PlayActionTaken::Played => {
            send_or_error(
                response_tx,
                super::Response::Player(Response::Playing(song_id, id)),
            )
            .await
        }
    }
}
async fn increase_volume(
    vol_inc: i8,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
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
        super::Response::Player(Response::VolumeUpdate(current_volume, id)),
    )
    .await;
}

async fn get_volume(
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<super::Response>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    send_or_error(rodio_tx, RodioMessage::GetVolume(tx)).await;
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
        super::Response::Player(Response::VolumeUpdate(current_volume, id)),
    )
    .await;
}
