use super::messages::ServerResponse;
use super::spawn_run_or_kill;
use super::spawn_unkillable;
use super::KillableTask;
use super::ServerComponent;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::app::taskmanager::TaskID;
use crate::core::oneshot_send_or_error;
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
use tracing::warn;

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
    GetVolume,
}

#[derive(Debug)]
enum RodioMessage {
    PlaySong(
        Arc<Vec<u8>>,
        ListSongID,
        // Where to send other updates
        oneshot::Sender<std::result::Result<(), DecoderError>>,
        // Where to send Done message
        oneshot::Sender<()>,
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
    Paused(ListSongID),
    Playing(ListSongID),
    Stopped(ListSongID),
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
        let KillableTask { id, kill_rx } = task;
        match request {
            KillableServerRequest::GetVolume => spawn_run_or_kill(
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

/// Playable song doubling up as a RAII guard that will send a message once it
/// has finished playing.
struct DroppableSong {
    song: Arc<Vec<u8>>,
    dropped_channel: Option<oneshot::Sender<()>>,
}
impl AsRef<[u8]> for DroppableSong {
    fn as_ref(&self) -> &[u8] {
        self.song.as_ref()
    }
}
impl Drop for DroppableSong {
    // Need to consider what to do if a song is dropped as part of a playsong, will
    // get a drop and then immediately after a play message. And I think the drop
    // triggers playlist functionality.
    fn drop(&mut self) {
        debug!("DroppableSong was dropped!");
        if let Some(tx) = self.dropped_channel.take() {
            oneshot_send_or_error(tx, ())
        }
    }
}

fn spawn_rodio_thread(mut msg_rx: mpsc::Receiver<RodioMessage>) {
    tokio::task::spawn_blocking(move || {
        // Rodio can produce output to stderr when we don't want it to, so we use Gag to
        // suppress stdout/stderr. The downside is that even though this runs in
        // a seperate thread all stderr for the whole app may be gagged.
        // Also seems to spew out characters?
        // TODO: Try to handle the errors from Rodio or write to a file.
        let _gag = match gag::Gag::stderr() {
            Ok(gag) => gag,
            Err(e) => {
                warn!("Error <{e}> gagging stderr output");
                return;
            }
        };
        let (_stream, stream_handle) =
            rodio::OutputStream::try_default().expect("Expect to get a handle to output stream");
        let sink = rodio::Sink::try_new(&stream_handle).expect("Expect music player not to error");
        // Hopefully someone else can't create a song with the same ID?!
        let mut cur_song_id = ListSongID::default();
        while let Some(msg) = msg_rx.blocking_recv() {
            debug!("Rodio received {:?}", msg);
            match msg {
                RodioMessage::PlaySong(song_pointer, song_id, tx, done_tx) => {
                    // compile_error!("Remember to test the new song done functionality");
                    // XXX: Perhaps should let the state know that we are playing.
                    // TODO: remove allocation
                    let sp = DroppableSong {
                        song: song_pointer,
                        dropped_channel: Some(done_tx),
                    };
                    let cur = std::io::Cursor::new(sp);
                    let source = match rodio::Decoder::new(cur) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to play song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            oneshot_send_or_error(tx, Err(e));
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
                    oneshot_send_or_error(tx, Ok(()));
                    cur_song_id = song_id;
                }
                RodioMessage::Stop(song_id, tx) => {
                    info!("Got message to stop playing {:?}", song_id);
                    if cur_song_id != song_id {
                        continue;
                    }
                    if !sink.empty() {
                        sink.stop()
                    }
                    oneshot_send_or_error(tx, ());
                }
                RodioMessage::PausePlay(song_id, tx) => {
                    info!("Got message to pause / play {:?}", song_id);
                    if cur_song_id != song_id {
                        continue;
                    }
                    if sink.is_paused() {
                        sink.play();
                        info!("Sending Play message {:?}", song_id);
                        oneshot_send_or_error(tx, PlayActionTaken::Played);
                    // We don't want to pause if sink is empty (but case
                    // could be handled in Playlist also)
                    } else if !sink.is_paused() && !sink.empty() {
                        sink.pause();
                        info!("Sending Pause message {:?}", song_id);
                        oneshot_send_or_error(tx, PlayActionTaken::Paused);
                    }
                }
                // XXX: May be able to handle this by reporting progress updates when playing
                // instead of needing to request/response here.
                RodioMessage::GetPlayProgress(song_id, tx) => {
                    info!("Got message to provide song progress update");
                    if cur_song_id == song_id {
                        oneshot_send_or_error(tx, sink.get_pos());
                        info!("Rodio sent song progress update");
                    } else {
                        info!("Rodio didn't send song progress update, it was no longer playing");
                        drop(tx)
                    }
                }
                // XXX: Should this just be IncreaseVolume(0)?
                RodioMessage::GetVolume(tx) => {
                    oneshot_send_or_error(tx, Percentage((sink.volume() * 100.0).round() as u8));
                    info!("Rodio sent volume update");
                }
                RodioMessage::IncreaseVolume(vol_inc, tx) => {
                    sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                    oneshot_send_or_error(tx, Percentage((sink.volume() * 100.0).round() as u8));
                    info!("Rodio sent volume update");
                }
            }
        }
    });
}

async fn play_song(
    song_pointer: Arc<Vec<u8>>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (tx_done, rx_done) = tokio::sync::oneshot::channel();
    send_or_error(
        rodio_tx,
        RodioMessage::PlaySong(song_pointer, song_id, tx, tx_done),
    )
    .await;
    let Ok(play_song_outcome) = rx.await else {
        // Should never happen!
        error!(
            "The player has been dropped while I was waiting for a play song outcome for {:?}",
            id
        );
        return;
    };
    match play_song_outcome {
        Ok(()) => send_or_error(
            response_tx.clone(),
            ServerResponse::new_player(id, Response::Playing(song_id)),
        ),
        Err(_) => send_or_error(
            response_tx.clone(),
            ServerResponse::new_player(id, Response::Error(song_id)),
        ),
    }
    .await;
    let Ok(()) = rx_done.await else {
        // Should never happen!
        error!(
            "The player has been dropped while I was waiting for a play song outcome for {:?}",
            id
        );
        return;
    };
    send_or_error(
        response_tx,
        ServerResponse::new_player(id, Response::DonePlaying(song_id)),
    )
    .await;
}
async fn get_play_progress(
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
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
                ServerResponse::new_player(id, Response::Paused(song_id)),
            )
            .await
        }
        PlayActionTaken::Played => {
            send_or_error(
                response_tx,
                ServerResponse::new_player(id, Response::Playing(song_id)),
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
        ServerResponse::new_player(id, Response::VolumeUpdate(current_volume)),
    )
    .await;
}

async fn get_volume(
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
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
        ServerResponse::new_player(id, Response::VolumeUpdate(current_volume)),
    )
    .await;
}
