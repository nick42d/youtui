//! Provides an asynchronous handle to a rodio sink, specifically designed to
//! handle gapless playback.
use futures::Stream;
use rodio::cpal::FromSample;
use rodio::source::EmptyCallback;
use rodio::source::PeriodicAccess;
use rodio::source::TrackPosition;
use rodio::Sample;
use rodio::Source;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

pub mod rodio {
    pub use rodio::*;
}

const PROGRESS_UPDATE_DELAY: Duration = Duration::from_millis(100);
const PLAYER_MSG_QUEUE_SIZE: usize = 50;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Percentage(u8);
impl From<Percentage> for u8 {
    fn from(value: Percentage) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SeekDirection {
    Forward,
    Back,
}

#[derive(Debug)]
enum AsyncRodioRequest<S, I> {
    PlaySong(S, I, RodioMpscSender<AsyncRodioResponse>),
    AutoplaySong(S, I, RodioMpscSender<AsyncRodioResponse>),
    QueueSong(S, I, RodioMpscSender<AsyncRodioResponse>),
    Stop(I, RodioOneshot<()>),
    PausePlay(I, RodioOneshot<AsyncRodioPlayActionTaken>),
    IncreaseVolume(i8, RodioOneshot<Percentage>),
    Seek(Duration, SeekDirection, RodioOneshot<(Duration, I)>),
}
#[derive(Debug)]
enum AsyncRodioResponse {
    ProgressUpdate(Duration),
    StartedPlaying(Option<Duration>),
    Queued(Option<Duration>),
    AutoplayingQueued,
    StoppedPlaying,
}
/// The action rodio took when it received a PausePlay message.
#[derive(Debug)]
enum AsyncRodioPlayActionTaken {
    Paused,
    Played,
}

/// Newtype for oneshot sender with custom debug implementation.
struct RodioOneshot<T>(oneshot::Sender<T>);
fn rodio_oneshot_channel<T>() -> (RodioOneshot<T>, oneshot::Receiver<T>) {
    let (tx, rx) = oneshot::channel();
    (RodioOneshot(tx), rx)
}
impl<T> Debug for RodioOneshot<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oneshot channel - {}", std::any::type_name::<T>())
    }
}

/// Newtype for mpsc sender with custom debug implementation.
struct RodioMpscSender<T>(mpsc::Sender<T>);
fn rodio_mpsc_channel<T>(buffer: usize) -> (RodioMpscSender<T>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(buffer);
    (RodioMpscSender(tx), rx)
}
impl<T> Debug for RodioMpscSender<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mpsc channel - {}", std::any::type_name::<T>())
    }
}
impl<T> From<RodioOneshot<T>> for oneshot::Sender<T> {
    fn from(value: RodioOneshot<T>) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct VolumeUpdate(pub Percentage);
#[derive(Debug)]
pub struct ProgressUpdate<I> {
    pub duration: Duration,
    pub identifier: I,
}
// NOTE: At this stage this difference between StoppedPlaying and Stopped is
// very thin. DonePlaying means that the song has been dropped by the player,
// whereas Stopped simply means that a Stop message to the player was succesful.
#[derive(Debug)]
pub struct Stopped<I>(pub I);
#[derive(Debug)]
pub enum PausePlayResponse<I> {
    Paused(I),
    Resumed(I),
}
#[derive(Debug)]
pub enum AutoplayUpdate<I>
where
    I: Debug,
{
    PlayProgress(Duration, I),
    Playing(Option<Duration>, I),
    DonePlaying(I),
    AutoplayQueued(I),
    Error(String),
}
#[derive(Debug)]
pub enum PlayUpdate<I>
where
    I: Debug,
{
    PlayProgress(Duration, I),
    Playing(Option<Duration>, I),
    DonePlaying(I),
    Error(String),
}
#[derive(Debug)]
pub enum QueueUpdate<I>
where
    I: Debug,
{
    PlayProgress(Duration, I),
    Queued(Option<Duration>, I),
    DonePlaying(I),
    Error(String),
}

pub struct AsyncRodio<S, I>
where
    I: Debug,
{
    handle: tokio::task::JoinHandle<()>,
    tx: std::sync::mpsc::Sender<AsyncRodioRequest<S, I>>,
}

impl<S, I> AsyncRodio<S, I>
where
    S: Source + Send + Sync + 'static,
    f32: FromSample<S::Item>,
    S::Item: Sample + Send,
    I: Debug + PartialEq + Copy + Send + 'static,
{
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<AsyncRodioRequest<S, I>>();
        let handle = tokio::task::spawn_blocking(move || {
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
            // NOTE: the OutputStream is not Send, hence why this requires a blocking task.
            let (_stream, stream_handle) = rodio::OutputStream::try_default()
                .expect("Expect to get a handle to output stream");
            let sink =
                rodio::Sink::try_new(&stream_handle).expect("Expect music player not to error");
            // Hopefully someone else can't create a song with the same ID?!
            let mut cur_song_duration = None;
            let mut next_song_duration = None;
            let mut cur_song_id = None;
            let mut next_song_id = None;
            // There is no need for a drop implementation on AsyncRodio, since if AsyncRodio
            // has dropped with it's sender, receive loop will receive Err and end.
            while let Ok(msg) = rx.recv() {
                match msg {
                    AsyncRodioRequest::AutoplaySong(song, song_id, tx) => {
                        if Some(song_id) == next_song_id {
                            info!(
                            "Received autoplay for {:?}, it's already queued up. It will play automatically.",
                            song_id
                        );
                            cur_song_id = Some(song_id);
                            next_song_id = None;
                            cur_song_duration = next_song_duration;
                            next_song_duration = None;
                            blocking_send_or_error(tx.0, AsyncRodioResponse::AutoplayingQueued);
                            continue;
                        }
                        if Some(song_id) == cur_song_id {
                            error!(
                            "Received autoplay for {:?}, it's already playing. I was expecting it to be queued up.",
                            song_id
                        );
                            blocking_send_or_error(tx.0, AsyncRodioResponse::AutoplayingQueued);
                            continue;
                        }
                        info!(
                            "Autoplaying a song that wasn't queued; clearing queue. Queued: {:?}",
                            next_song_id
                        );
                        cur_song_duration = song.total_duration();
                        if !sink.empty() {
                            sink.stop()
                        }
                        let txs = tx.0.clone();
                        let song = add_periodic_access(song, PROGRESS_UPDATE_DELAY, move |s| {
                            blocking_send_or_error(
                                &txs,
                                AsyncRodioResponse::ProgressUpdate(s.get_pos()),
                            );
                        });
                        let on_done = on_done_cb(&tx);
                        sink.append(song);
                        sink.append(on_done);
                        // Handle case were we've received a play message but queue was paused.
                        if sink.is_paused() {
                            sink.play();
                        }
                        debug!("Now playing {:?}", song_id);
                        // Send the Now Playing message for good orders sake to avoid
                        // synchronization issues.
                        blocking_send_or_error(
                            tx.0,
                            AsyncRodioResponse::StartedPlaying(cur_song_duration),
                        );
                        cur_song_id = Some(song_id);
                        next_song_id = None;
                        next_song_duration = None;
                    }
                    AsyncRodioRequest::QueueSong(song, song_id, tx) => {
                        if sink.empty() {
                            error!(
                                "Tried to queue up a song, but sink was empty... Continuing anyway"
                            );
                        }
                        next_song_duration = song.total_duration();
                        blocking_send_or_error(
                            &tx.0,
                            AsyncRodioResponse::Queued(next_song_duration),
                        );
                        let txs = tx.0.clone();
                        let song = add_periodic_access(song, PROGRESS_UPDATE_DELAY, move |s| {
                            blocking_send_or_error(
                                &txs,
                                AsyncRodioResponse::ProgressUpdate(s.get_pos()),
                            );
                        });
                        let on_done = on_done_cb(&tx);
                        sink.append(song);
                        sink.append(on_done);
                        next_song_id = Some(song_id);
                    }
                    AsyncRodioRequest::PlaySong(song, song_id, tx) => {
                        cur_song_duration = song.total_duration();
                        if !sink.empty() {
                            sink.stop()
                        }
                        let txs = tx.0.clone();
                        let song = add_periodic_access(song, PROGRESS_UPDATE_DELAY, move |s| {
                            blocking_send_or_error(
                                &txs,
                                AsyncRodioResponse::ProgressUpdate(s.get_pos()),
                            );
                        });
                        let on_done = on_done_cb(&tx);
                        sink.append(song);
                        sink.append(on_done);
                        // Handle case were we've received a play message but queue was paused.
                        if sink.is_paused() {
                            sink.play();
                        }
                        debug!("Now playing {:?}", song_id);
                        // Send the Now Playing message for good orders sake to avoid
                        // synchronization issues.
                        blocking_send_or_error(
                            tx.0,
                            AsyncRodioResponse::StartedPlaying(cur_song_duration),
                        );
                        cur_song_id = Some(song_id);
                        next_song_id = None;
                    }
                    AsyncRodioRequest::Stop(song_id, tx) => {
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
                    AsyncRodioRequest::PausePlay(song_id, tx) => {
                        info!("Got message to pause / play {:?}", song_id);
                        if cur_song_id != Some(song_id) {
                            continue;
                        }
                        if sink.is_paused() {
                            sink.play();
                            info!("Sending Play message {:?}", song_id);
                            oneshot_send_or_error(tx.0, AsyncRodioPlayActionTaken::Played);
                        // We don't want to pause if sink is empty (but case
                        // could be handled in Playlist also)
                        } else if !sink.is_paused() && !sink.empty() {
                            sink.pause();
                            info!("Sending Pause message {:?}", song_id);
                            oneshot_send_or_error(tx.0, AsyncRodioPlayActionTaken::Paused);
                        }
                    }
                    AsyncRodioRequest::IncreaseVolume(vol_inc, tx) => {
                        sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                        oneshot_send_or_error(
                            tx.0,
                            Percentage((sink.volume() * 100.0).round() as u8),
                        );
                        info!("Rodio sent volume update");
                    }
                    AsyncRodioRequest::Seek(inc, direction, tx) => {
                        // Rodio always you to seek past song end when paused, and will report back
                        // an incorrect position for sink.get_pos().
                        // TODO: Report upstream
                        let res = match direction {
                            SeekDirection::Forward => sink.try_seek(
                                sink.get_pos()
                                    .saturating_add(inc)
                                    .min(cur_song_duration.unwrap_or_default()),
                            ),
                            SeekDirection::Back => sink.try_seek(
                                sink.get_pos()
                                    .saturating_sub(inc)
                                    .min(cur_song_duration.unwrap_or_default()),
                            ),
                        };
                        if res.is_err() {
                            error!("Failed to seek!!");
                        }
                        let Some(cur_song_id) = cur_song_id else {
                            warn!("Tried to seek, but no song loaded");
                            continue;
                        };
                        // It seems that there is a race condition with seeking a paused track in
                        // rodio itself. This delay is sufficient.
                        std::thread::sleep(Duration::from_millis(5));
                        oneshot_send_or_error(tx.0, (sink.get_pos(), cur_song_id));
                    }
                }
            }
        });
        Self { handle, tx }
    }
    pub fn autoplay_song(&self, song: S, identifier: I) -> impl Stream<Item = AutoplayUpdate<I>> {
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            std_send_or_error(
                selftx,
                AsyncRodioRequest::AutoplaySong(song, identifier, tx),
            )
            .await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        send_or_error(
                            &streamtx,
                            AutoplayUpdate::PlayProgress(duration, identifier),
                        )
                        .await;
                    }
                    AsyncRodioResponse::Queued(_) => {
                        send_or_error(
                            &streamtx,
                            AutoplayUpdate::Error(format!(
                                "Received queued message, but I wasn't queued... {:?}",
                                identifier
                            )),
                        )
                        .await;
                    }
                    // This is the case where the song we asked to play is already
                    // queued. In this case, this task can finish, as the task that
                    // added the song to the queue is responsible for the playback
                    // updates.
                    AsyncRodioResponse::AutoplayingQueued => {
                        send_or_error(&streamtx, AutoplayUpdate::AutoplayQueued(identifier)).await;
                        return;
                    }
                    AsyncRodioResponse::StartedPlaying(duration) => {
                        send_or_error(&streamtx, AutoplayUpdate::Playing(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        send_or_error(&streamtx, AutoplayUpdate::DonePlaying(identifier)).await;
                        return;
                    }
                }
            }
            // Should never reach here! Player should send either Error, Stopped or Playing
            // message first.
            error!(
                "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
                identifier
            );
        });
        ReceiverStream::new(streamrx)
    }
    pub fn queue_song(&self, song: S, identifier: I) -> impl Stream<Item = QueueUpdate<I>> {
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            std_send_or_error(selftx, AsyncRodioRequest::QueueSong(song, identifier, tx)).await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        send_or_error(&streamtx, QueueUpdate::PlayProgress(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::Queued(duration) => {
                        send_or_error(&streamtx, QueueUpdate::Queued(duration, identifier)).await;
                    }
                    AsyncRodioResponse::AutoplayingQueued => {
                        send_or_error(
                            &streamtx,
                            QueueUpdate::Error(format!(
                                "Received AutoPlayingQueued message, but I asked to queue... {:?}",
                                identifier
                            )),
                        )
                        .await;
                    }
                    AsyncRodioResponse::StartedPlaying(_) => {
                        send_or_error(
                            &streamtx,
                            QueueUpdate::Error(format!(
                                "Received StartedPlaying message, but I asked to queue... {:?}",
                                identifier,
                            )),
                        )
                        .await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        send_or_error(&streamtx, QueueUpdate::DonePlaying(identifier)).await;
                        return;
                    }
                }
            }
            // Should never reach here! Player should send either Error, Stopped or Playing
            // message first.
            error!(
                "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
                identifier
            );
        });
        ReceiverStream::new(streamrx)
    }
    pub fn play_song(&self, song: S, identifier: I) -> impl Stream<Item = PlayUpdate<I>> {
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            std_send_or_error(selftx, AsyncRodioRequest::PlaySong(song, identifier, tx)).await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        send_or_error(&streamtx, PlayUpdate::PlayProgress(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::Queued(_) => {
                        send_or_error(
                            &streamtx,
                            PlayUpdate::Error(format!(
                                "Received Queued message, but I wasn't queued... {:?}",
                                identifier
                            )),
                        )
                        .await;
                    }
                    AsyncRodioResponse::AutoplayingQueued => {
                        send_or_error(
                            &streamtx,
                            PlayUpdate::Error(format!(
                                "Received AutoPlayingQueued message, but I asked to play... {:?}",
                                identifier
                            )),
                        )
                        .await;
                    }
                    AsyncRodioResponse::StartedPlaying(duration) => {
                        send_or_error(&streamtx, PlayUpdate::Playing(duration, identifier)).await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        send_or_error(&streamtx, PlayUpdate::DonePlaying(identifier)).await;
                        return;
                    }
                }
            }
            // Should never reach here! Player should send either Error, Stopped or Playing
            // message first.
            error!(
                "The sender has been dropped and there are no further messages while I was waiting for a play song outcome for {:?}",
                identifier
            );
        });
        ReceiverStream::new(streamrx)
    }
    pub async fn seek(
        &self,
        duration: Duration,
        direction: SeekDirection,
    ) -> Option<ProgressUpdate<I>> {
        let (tx, rx) = rodio_oneshot_channel();
        std_send_or_error(&self.tx, AsyncRodioRequest::Seek(duration, direction, tx)).await;
        let Ok((current_duration, song_id)) = rx.await else {
            // This happens intentionally - when a seek is requested for a song
            // but all songs have finished, instead of sending a reply, rodio will drop
            // sender.
            info!("The song I tried to seek is no longer playing");
            return None;
        };
        Some(ProgressUpdate {
            duration: current_duration,
            identifier: song_id,
        })
    }
    pub async fn stop(&self, identifier: I) -> Option<Stopped<I>> {
        let (tx, rx) = rodio_oneshot_channel();
        std_send_or_error(&self.tx, AsyncRodioRequest::Stop(identifier, tx)).await;
        let Ok(_) = rx.await else {
            // This happens intentionally - when a stop is requested for a song
            // that's no longer playing, instead of sending a reply, rodio will drop sender.
            info!("The song I tried to stop is no longer playing");
            return None;
        };
        Some(Stopped(identifier))
    }
    pub async fn pause_play(&self, identifier: I) -> Option<PausePlayResponse<I>> {
        let (tx, rx) = rodio_oneshot_channel();
        std_send_or_error(&self.tx, AsyncRodioRequest::PausePlay(identifier, tx)).await;
        let Ok(play_action_taken) = rx.await else {
            // This happens intentionally - when a pauseplay is requested for a song
            // that's no longer playing, instead of sending a reply, rodio will drop sender.
            info!("The song I tried to pause/play was no longer selected",);
            return None;
        };
        match play_action_taken {
            AsyncRodioPlayActionTaken::Paused => Some(PausePlayResponse::Paused(identifier)),
            AsyncRodioPlayActionTaken::Played => Some(PausePlayResponse::Resumed(identifier)),
        }
    }
    pub async fn increase_volume(&self, vol_inc: i8) -> Option<VolumeUpdate> {
        let (tx, rx) = rodio_oneshot_channel();
        std_send_or_error(&self.tx, AsyncRodioRequest::IncreaseVolume(vol_inc, tx)).await;
        let Ok(current_volume) = rx.await else {
            // Should never happen!
            error!("The player has been dropped while I was waiting for a volume update for",);
            return None;
        };
        Some(VolumeUpdate(current_volume))
    }
}

/// Specific helper function to generate a source that sends a stopped playing
/// message to the sender.
fn on_done_cb<S>(tx: &RodioMpscSender<AsyncRodioResponse>) -> EmptyCallback<S> {
    let tx = tx.0.clone();
    let cb = move || {
        blocking_send_or_error(&tx, AsyncRodioResponse::StoppedPlaying);
    };
    EmptyCallback::new(Box::new(cb))
}

/// Add a periodic access callback to song.
fn add_periodic_access<S>(
    song: S,
    interval: Duration,
    callback: impl FnMut(&mut TrackPosition<S>),
) -> PeriodicAccess<TrackPosition<S>, impl FnMut(&mut TrackPosition<S>)>
where
    S: Source + Send + Sync + 'static,
    f32: FromSample<S::Item>,
    S::Item: Sample + Send,
{
    song.track_position().periodic_access(interval, callback)
}

/* #### BELOW CODE COPIED FROM youtui::core #### */
/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub async fn send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .await
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
pub async fn std_send_or_error<T, S: Borrow<std::sync::mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub fn blocking_send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .blocking_send(msg)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
/// Send a message to the specified Tokio oneshot::Sender, and if sending fails,
/// log an error with Tracing.
pub fn oneshot_send_or_error<T: Debug, S: Into<oneshot::Sender<T>>>(tx: S, msg: T) {
    tx.into()
        .send(msg)
        .unwrap_or_else(|e| error!("Error received when sending message {:?}", e));
}
/* #### ABOVE CODE COPIED FROM youtui::core #### */
