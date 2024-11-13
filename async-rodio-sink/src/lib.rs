//! Provides an asynchronous handle to a rodio sink, specifically designed to
//! handle gapless playback.
use futures::Stream;
use rodio::decoder::DecoderError;
use rodio::source::EmptyCallback;
use rodio::source::PeriodicAccess;
use rodio::source::TrackPosition;
use rodio::Decoder;
use rodio::Source;
use std::fmt::Debug;
use std::io::Cursor;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

#[derive(Debug)]
pub struct Percentage(pub u8);

#[derive(Debug)]
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
    PausePlay(I, RodioOneshot<PlayActionTaken>),
    IncreaseVolume(i8, RodioOneshot<Percentage>),
    Seek(Duration, SeekDirection, RodioOneshot<(Duration, I)>),
}

pub struct VolumeUpdate(pub Percentage);
pub struct ProgressUpdate<I>(Duration, I);
// At this stage this difference between DonePlaying and Stopped is very thin.
// DonePlaying means that the song has been dropped by the player, whereas
// Stopped simply means that a Stop message to the player was succesful.
pub struct Stopped<I>(I);
pub enum PausePlayResponse<I> {
    Paused(I),
    Resumed(I),
}

#[derive(Debug)]
enum AsyncRodioResponse {
    ProgressUpdate(Duration),
    StartedPlaying(Option<Duration>),
    Queued(Option<Duration>),
    AutoplayingQueued,
    StoppedPlaying,
}

#[derive(Debug)]
/// The action rodio took when it received a PausePlay message.
pub enum PlayActionTaken {
    Paused,
    Played,
}

/// Newtype for oneshot channel with custom derive
pub struct RodioOneshot<T>(oneshot::Sender<T>);

pub fn rodio_oneshot_channel<T>() -> (RodioOneshot<T>, oneshot::Receiver<T>) {
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

/// Newtype for mpsc channel with custom derive
pub struct RodioMpscSender<T>(mpsc::Sender<T>);

pub fn rodio_mpsc_channel<T>(buffer: usize) -> (RodioMpscSender<T>, mpsc::Receiver<T>) {
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

const PROGRESS_UPDATE_DELAY: Duration = Duration::from_millis(100);
const PLAYER_MSG_QUEUE_SIZE: usize = 50;

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
    S: AsRef<[u8]> + Send + Sync + 'static,
    I: Debug,
{
    handle: tokio::task::JoinHandle<()>,
    tx: tokio::sync::mpsc::Sender<AsyncRodioRequest<Decoder<Cursor<S>>, I>>,
}

impl<S, I> Drop for AsyncRodio<S, I>
where
    S: AsRef<[u8]> + Send + Sync + 'static,
    I: Debug,
{
    // WARNING! This doesn't do what I thought it did. Since JoinHandle is for a
    // blocking task, I can't abort it.
    fn drop(&mut self) {
        self.handle.abort()
    }
}

impl<S, I> AsyncRodio<S, I>
where
    S: AsRef<[u8]> + Send + Sync + 'static,
    I: Debug + PartialEq + Copy + Send + 'static,
{
    pub fn autoplay_song(
        &self,
        song: S,
        identifier: I,
    ) -> Result<impl Stream<Item = AutoplayUpdate<I>>, ()> {
        let song = try_decode(song).map_err(|_| ())?;
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            selftx
                .send(AsyncRodioRequest::AutoplaySong(song, identifier, tx))
                .await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        streamtx
                            .send(AutoplayUpdate::PlayProgress(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::Queued(_) => {
                        streamtx
                            .send(AutoplayUpdate::Error(format!(
                                "Received queued message, but I wasn't queued... {:?}",
                                identifier
                            )))
                            .await;
                    }
                    // This is the case where the song we asked to play is already
                    // queued. In this case, this task can finish, as the task that
                    // added the song to the queue is responsible for the playback
                    // updates.
                    AsyncRodioResponse::AutoplayingQueued => {
                        streamtx
                            .send(AutoplayUpdate::AutoplayQueued(identifier))
                            .await;
                        return;
                    }
                    AsyncRodioResponse::StartedPlaying(duration) => {
                        streamtx
                            .send(AutoplayUpdate::Playing(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        streamtx.send(AutoplayUpdate::DonePlaying(identifier)).await;
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
        Ok(ReceiverStream::new(streamrx))
    }
    pub fn queue_song(
        &self,
        song: S,
        identifier: I,
    ) -> Result<impl Stream<Item = QueueUpdate<I>>, ()> {
        let song = try_decode(song).map_err(|_| ())?;
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            selftx
                .send(AsyncRodioRequest::QueueSong(song, identifier, tx))
                .await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        streamtx
                            .send(QueueUpdate::PlayProgress(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::Queued(duration) => {
                        streamtx
                            .send(QueueUpdate::Queued(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::AutoplayingQueued => {
                        streamtx
                            .send(QueueUpdate::Error(format!(
                                "Received AutoPlayingQueued message, but I asked to queue... {:?}",
                                identifier
                            )))
                            .await;
                    }
                    AsyncRodioResponse::StartedPlaying(_) => {
                        streamtx
                            .send(QueueUpdate::Error(format!(
                                "Received StartedPlaying message, but I asked to queue... {:?}",
                                identifier,
                            )))
                            .await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        streamtx.send(QueueUpdate::DonePlaying(identifier)).await;
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
        Ok(ReceiverStream::new(streamrx))
    }
    pub fn play_song(
        &self,
        song: S,
        identifier: I,
    ) -> Result<impl Stream<Item = PlayUpdate<I>>, ()> {
        let song = try_decode(song).map_err(|_| ())?;
        let (tx, mut rx) = rodio_mpsc_channel(PLAYER_MSG_QUEUE_SIZE);
        let (streamtx, streamrx) = tokio::sync::mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let selftx = self.tx.clone();
        tokio::task::spawn(async move {
            selftx
                .send(AsyncRodioRequest::PlaySong(song, identifier, tx))
                .await;
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsyncRodioResponse::ProgressUpdate(duration) => {
                        streamtx
                            .send(PlayUpdate::PlayProgress(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::Queued(_) => {
                        streamtx
                            .send(PlayUpdate::Error(format!(
                                "Received Queued message, but I wasn't queued... {:?}",
                                identifier
                            )))
                            .await;
                    }
                    AsyncRodioResponse::AutoplayingQueued => {
                        streamtx
                            .send(PlayUpdate::Error(format!(
                                "Received AutoPlayingQueued message, but I asked to play... {:?}",
                                identifier
                            )))
                            .await;
                    }
                    AsyncRodioResponse::StartedPlaying(duration) => {
                        streamtx
                            .send(PlayUpdate::Playing(duration, identifier))
                            .await;
                    }
                    AsyncRodioResponse::StoppedPlaying => {
                        streamtx.send(PlayUpdate::DonePlaying(identifier)).await;
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
        Ok(ReceiverStream::new(streamrx))
    }
    pub async fn seek(
        &self,
        duration: Duration,
        direction: SeekDirection,
    ) -> Option<ProgressUpdate<I>> {
        let (tx, rx) = rodio_oneshot_channel();
        self.tx
            .send(AsyncRodioRequest::Seek(duration, direction, tx))
            .await;
        let Ok((current_duration, song_id)) = rx.await else {
            // This happens intentionally - when a seek is requested for a song
            // but all songs have finished, instead of sending a reply, rodio will drop
            // sender.
            info!("The song I tried to seek is no longer playing");
            return None;
        };
        Some(ProgressUpdate(current_duration, song_id))
    }
    pub async fn stop(&self, identifier: I) -> Option<Stopped<I>> {
        let (tx, rx) = rodio_oneshot_channel();
        self.tx.send(AsyncRodioRequest::Stop(identifier, tx)).await;
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
        self.tx
            .send(AsyncRodioRequest::PausePlay(identifier, tx))
            .await;
        let Ok(play_action_taken) = rx.await else {
            // This happens intentionally - when a pauseplay is requested for a song
            // that's no longer playing, instead of sending a reply, rodio will drop sender.
            info!("The song I tried to pause/play was no longer selected",);
            return None;
        };
        match play_action_taken {
            PlayActionTaken::Paused => Some(PausePlayResponse::Paused(identifier)),
            PlayActionTaken::Played => Some(PausePlayResponse::Resumed(identifier)),
        }
    }
    pub async fn increase_volume(&self, vol_inc: i8) -> Option<VolumeUpdate> {
        let (tx, rx) = rodio_oneshot_channel();
        self.tx
            .send(AsyncRodioRequest::IncreaseVolume(vol_inc, tx))
            .await;
        let Ok(current_volume) = rx.await else {
            // Should never happen!
            error!("The player has been dropped while I was waiting for a volume update for",);
            return None;
        };
        Some(VolumeUpdate(current_volume))
    }
    pub fn new(channel_size: usize) -> Self {
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<AsyncRodioRequest<Decoder<Cursor<S>>, I>>(channel_size);
        let handle = tokio::task::spawn(async move {
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
            while let Some(msg) = rx.blocking_recv() {
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
                            tx.0.blocking_send(AsyncRodioResponse::AutoplayingQueued);
                            continue;
                        }
                        if Some(song_id) == cur_song_id {
                            error!(
                            "Received autoplay for {:?}, it's already playing. I was expecting it to be queued up.",
                            song_id
                        );
                            tx.0.blocking_send(AsyncRodioResponse::AutoplayingQueued);
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
                            txs.blocking_send(AsyncRodioResponse::ProgressUpdate(s.get_pos()));
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
                        tx.0.send(AsyncRodioResponse::StartedPlaying(cur_song_duration));
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
                        &tx.0.send(AsyncRodioResponse::Queued(next_song_duration));
                        let txs = tx.0.clone();
                        let song = add_periodic_access(song, PROGRESS_UPDATE_DELAY, move |s| {
                            txs.blocking_send(AsyncRodioResponse::ProgressUpdate(s.get_pos()));
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
                            txs.blocking_send(AsyncRodioResponse::ProgressUpdate(s.get_pos()));
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
                        tx.0.send(AsyncRodioResponse::StartedPlaying(cur_song_duration));
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
                        tx.0.send(());
                    }
                    AsyncRodioRequest::PausePlay(song_id, tx) => {
                        info!("Got message to pause / play {:?}", song_id);
                        if cur_song_id != Some(song_id) {
                            continue;
                        }
                        if sink.is_paused() {
                            sink.play();
                            info!("Sending Play message {:?}", song_id);
                            tx.0.send(PlayActionTaken::Played);
                        // We don't want to pause if sink is empty (but case
                        // could be handled in Playlist also)
                        } else if !sink.is_paused() && !sink.empty() {
                            sink.pause();
                            info!("Sending Pause message {:?}", song_id);
                            tx.0.send(PlayActionTaken::Paused);
                        }
                    }
                    AsyncRodioRequest::IncreaseVolume(vol_inc, tx) => {
                        sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                        tx.0.send(Percentage((sink.volume() * 100.0).round() as u8));
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
                        tx.0.send((sink.get_pos(), cur_song_id));
                    }
                }
            }
        });
        Self { handle, tx }
    }
}

/// Specific helper function to generate a source that sends a stopped playing
/// message to the sender.
fn on_done_cb(tx: &RodioMpscSender<AsyncRodioResponse>) -> EmptyCallback<f32> {
    let tx = tx.0.clone();
    let cb = move || {
        tx.blocking_send(AsyncRodioResponse::StoppedPlaying);
    };
    EmptyCallback::new(Box::new(cb))
}

/// Try to decode bytes into Source.
fn try_decode<S: AsRef<[u8]> + Send + Sync + 'static>(
    song: S,
) -> std::result::Result<Decoder<Cursor<S>>, DecoderError> {
    let cur = std::io::Cursor::new(song);
    rodio::Decoder::new(cur)
}

/// Add a periodic access callback to song.
fn add_periodic_access<S>(
    song: Decoder<Cursor<S>>,
    interval: Duration,
    callback: impl FnMut(&mut TrackPosition<Decoder<Cursor<S>>>),
) -> PeriodicAccess<
    TrackPosition<Decoder<Cursor<S>>>,
    impl FnMut(&mut TrackPosition<Decoder<Cursor<S>>>),
>
where
    S: AsRef<[u8]> + Send + Sync + 'static,
{
    song.track_position().periodic_access(interval, callback)
}
