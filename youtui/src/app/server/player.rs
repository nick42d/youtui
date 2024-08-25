use super::downloader::InMemSong;
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
use rodio::source::PeriodicAccess;
use rodio::source::TrackPosition;
use rodio::Decoder;
use rodio::Source;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

const PLAYER_MSG_QUEUE_SIZE: usize = 256;
const PROGRESS_UPDATE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct SongTypeNew {
    status: SongStatus,
    id: ListSongID,
}

#[derive(Debug)]
enum SongStatus {
    Downloaded(Arc<InMemSong>),
    Buffering,
}

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
    GetPlayProgress(ListSongID),
    Stop(ListSongID),
    PausePlay(ListSongID),
    Seek(i8),
}

#[derive(Debug)]
pub enum KillableServerRequest {}

/// Newtype for custom derive
pub struct RodioOneshot<T>(oneshot::Sender<T>);

fn rodio_channel<T>() -> (RodioOneshot<T>, oneshot::Receiver<T>) {
    let (tx, rx) = oneshot::channel();
    (RodioOneshot(tx), rx)
}

impl<T> std::fmt::Debug for RodioOneshot<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("")
    }
}

impl<T> From<RodioOneshot<T>> for oneshot::Sender<T> {
    fn from(value: RodioOneshot<T>) -> Self {
        value.0
    }
}

#[derive(Debug)]
enum RodioMessage {
    PlaySong(
        Arc<InMemSong>,
        ListSongID,
        // Where to send other updates
        RodioOneshot<std::result::Result<Option<Duration>, DecoderError>>,
        // Where to send Done message
        RodioOneshot<()>,
    ),
    AutoplaySong(
        Arc<InMemSong>,
        ListSongID,
        // Where to send other updates
        RodioOneshot<std::result::Result<Option<Duration>, DecoderError>>,
        // Where to send Done message
        RodioOneshot<()>,
    ),
    QueueSong(
        Arc<InMemSong>,
        ListSongID,
        // Where to send other updates
        RodioOneshot<std::result::Result<Option<Duration>, DecoderError>>,
        // Where to send Done message
        RodioOneshot<()>,
    ),
    GetPlayProgress(ListSongID, RodioOneshot<Duration>),
    Stop(ListSongID, RodioOneshot<()>),
    PausePlay(ListSongID, RodioOneshot<PlayActionTaken>),
    IncreaseVolume(i8, RodioOneshot<Percentage>),
    Seek(i8, RodioOneshot<(Duration, ListSongID)>),
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
    Resumed(ListSongID),
    Playing(Option<Duration>, ListSongID),
    Queued(Option<Duration>, ListSongID),
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
            UnkillableServerRequest::GetPlayProgress(song_id) => {
                spawn_unkillable(get_play_progress(song_id, rodio_tx, task, response_tx))
            }
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

/// Playable song doubling up as a RAII guard that will send a message once it
/// has finished playing.
struct DroppableSong {
    song: Arc<InMemSong>,
    // Song ID is stored for debugging purposes only - the receiver already knows the Song ID.
    song_id: ListSongID,
    dropped_channel: Option<RodioOneshot<()>>,
}
impl AsRef<[u8]> for DroppableSong {
    fn as_ref(&self) -> &[u8] {
        self.song.0.as_ref()
    }
}
impl Drop for DroppableSong {
    // Need to consider what to do if a song is dropped as part of a playsong, will
    // get a drop and then immediately after a play message. And I think the drop
    // triggers playlist functionality.
    fn drop(&mut self) {
        debug!("DroppableSong {:?} was dropped!", self.song_id);
        if let Some(tx) = self.dropped_channel.take() {
            oneshot_send_or_error(tx.0, ())
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
        let mut cur_song_duration = None;
        let mut cur_song_id = None;
        let mut next_song_id = None;
        while let Some(msg) = msg_rx.blocking_recv() {
            debug!("Rodio received {:?}", msg);
            match msg {
                RodioMessage::AutoplaySong(song_pointer, song_id, tx, done_tx) => {
                    if Some(song_id) == next_song_id {
                        info!(
                            "Received autoplay for {:?}, it's already queued up. It will play automatically.",
                            song_id
                        );
                        cur_song_id = Some(song_id);
                        next_song_id = None;
                    }
                    if Some(song_id) == cur_song_id {
                        error!(
                            "Received autoplay for {:?}, it's already playing. I was expecting it to be queued up.",
                            song_id
                        );
                    }
                    // DUPLICATE FROM PLAYSONG
                    let source = match try_decode(song_pointer, song_id, done_tx) {
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
                    cur_song_duration = source.total_duration();
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
                    oneshot_send_or_error(tx.0, Ok(cur_song_duration));
                    cur_song_id = Some(song_id);
                    next_song_id = None;
                    // END DUPLICATE
                }
                RodioMessage::QueueSong(song_pointer, song_id, tx, done_tx) => {
                    // DUPLICATE FROM PLAYSONG
                    // TEST CODE
                    let source = match try_decode(song_pointer, song_id, done_tx) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to decode queued song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            oneshot_send_or_error(tx.0, Err(e));
                            continue;
                        }
                    };
                    if sink.empty() {
                        error!("Tried to queue up a song, but sink was empty... Continuing anyway");
                    }
                    // END DUPLICATE
                    let next_song_duration = source.total_duration();
                    oneshot_send_or_error(tx.0, Ok(next_song_duration));
                    sink.append(source);
                    next_song_id = Some(song_id);
                }
                RodioMessage::PlaySong(song_pointer, song_id, tx, done_tx) => {
                    let source = match try_decode(song_pointer, song_id, done_tx) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to play song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            oneshot_send_or_error(tx.0, Err(e));
                            continue;
                        }
                    };
                    cur_song_duration = source.total_duration();
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
                    oneshot_send_or_error(tx.0, Ok(cur_song_duration));
                    cur_song_id = Some(song_id);
                    next_song_id = None;
                }
                RodioMessage::Stop(song_id, tx) => {
                    info!("Got message to stop playing {:?}", song_id);
                    if cur_song_id != Some(song_id) {
                        continue;
                    }
                    if !sink.empty() {
                        sink.stop()
                    }
                    cur_song_id = None;
                    next_song_id = None;
                    cur_song_duration = None;
                    oneshot_send_or_error(tx.0, ());
                }
                RodioMessage::PausePlay(song_id, tx) => {
                    info!("Got message to pause / play {:?}", song_id);
                    if cur_song_id != Some(song_id) {
                        continue;
                    }
                    if sink.is_paused() {
                        sink.play();
                        info!("Sending Play message {:?}", song_id);
                        oneshot_send_or_error(tx.0, PlayActionTaken::Played);
                    // We don't want to pause if sink is empty (but case
                    // could be handled in Playlist also)
                    } else if !sink.is_paused() && !sink.empty() {
                        sink.pause();
                        info!("Sending Pause message {:?}", song_id);
                        oneshot_send_or_error(tx.0, PlayActionTaken::Paused);
                    }
                }
                // XXX: May be able to handle this by reporting progress updates when playing
                // instead of needing to request/response here.
                // Seems to go out of sync with Sink - I can send a request here after song is
                // dropped and still get a progress update - why?
                RodioMessage::GetPlayProgress(song_id, tx) => {
                    info!("Got message to provide song progress update");
                    if cur_song_id == Some(song_id) {
                        oneshot_send_or_error(tx.0, sink.get_pos());
                        info!("Rodio sent song progress update");
                    } else {
                        info!("Rodio didn't send song progress update, it was no longer playing");
                        drop(tx)
                    }
                }
                RodioMessage::IncreaseVolume(vol_inc, tx) => {
                    sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                    oneshot_send_or_error(tx.0, Percentage((sink.volume() * 100.0).round() as u8));
                    info!("Rodio sent volume update");
                }
                RodioMessage::Seek(inc, tx) => {
                    // Rodio always you to seek past song end when paused, and will report back an
                    // incorrect position for sink.get_pos().
                    // TODO: Report upstream
                    let res = if inc > 0 {
                        sink.try_seek(
                            sink.get_pos()
                                .saturating_add(Duration::from_secs(inc as u64))
                                .min(cur_song_duration.unwrap_or_default()),
                        )
                    } else {
                        sink.try_seek(
                            sink.get_pos()
                                .saturating_sub(Duration::from_secs((-inc) as u64))
                                .min(cur_song_duration.unwrap_or_default()),
                        )
                    };
                    if res.is_err() {
                        error!("Failed to seek!!");
                    }
                    let Some(cur_song_id) = cur_song_id else {
                        warn!("Tried to seek, but no song loaded");
                        continue;
                    };
                    oneshot_send_or_error(tx.0, (sink.get_pos(), cur_song_id));
                }
            }
        }
    });
}

fn try_decode(
    song: Arc<InMemSong>,
    song_id: ListSongID,
    progress_stream_tx: mpsc::Sender<Duration>,
    done_tx: RodioOneshot<()>,
) -> std::result::Result<
    TrackPosition<PeriodicAccess<Decoder<Cursor<DroppableSong>>>, impl FnMut(Source)>,
    DecoderError,
> {
    // DUPLICATE FROM PLAYSONG
    let sp = DroppableSong {
        song,
        song_id,
        dropped_channel: Some(done_tx),
    };
    let cur = std::io::Cursor::new(sp);
    rodio::Decoder::new(cur).map(move |s| {
        s.track_position()
            .periodic_access(PROGRESS_UPDATE_DELAY, move |s| {
                progress_stream_tx.clone().blocking_send(s.get_pos());
            })
    })
}

async fn autoplay_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_channel();
    let (tx_done, rx_done) = rodio_channel();
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
        Ok(duration) => send_or_error(
            response_tx.clone(),
            ServerResponse::new_player(id, Response::Playing(duration, song_id)),
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
async fn queue_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_channel();
    let (tx_done, rx_done) = rodio_channel();
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
        Ok(duration) => send_or_error(
            response_tx.clone(),
            ServerResponse::new_player(id, Response::Playing(duration, song_id)),
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
async fn play_song(
    song_pointer: Arc<InMemSong>,
    song_id: ListSongID,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_channel();
    let (tx_done, rx_done) = rodio_channel();
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
        Ok(duration) => send_or_error(
            response_tx.clone(),
            ServerResponse::new_player(id, Response::Playing(duration, song_id)),
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
    let (tx, rx) = rodio_channel();
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
async fn seek(
    inc: i8,
    rodio_tx: mpsc::Sender<RodioMessage>,
    id: TaskID,
    response_tx: mpsc::Sender<ServerResponse>,
) {
    let (tx, rx) = rodio_channel();
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
    let (tx, rx) = rodio_channel();
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
    let (tx, rx) = rodio_channel();
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
    let (tx, rx) = rodio_channel();
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
