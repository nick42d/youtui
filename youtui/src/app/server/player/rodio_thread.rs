use crate::app::server::downloader::InMemSong;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::core::blocking_send_or_error;
use crate::core::oneshot_send_or_error;
use crate::core::send_or_error;
use rodio::decoder::DecoderError;
use rodio::source::PeriodicAccess;
use rodio::source::SkipDuration;
use rodio::source::TrackPosition;
use rodio::Decoder;
use rodio::Source;
use std::borrow::Borrow;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::PROGRESS_UPDATE_DELAY;

#[derive(Debug)]
pub enum RodioMessage {
    PlaySong(Arc<InMemSong>, ListSongID, RodioMpsc<PlaySongResponse>),
    AutoplaySong(Arc<InMemSong>, ListSongID, RodioMpsc<PlaySongResponse>),
    QueueSong(Arc<InMemSong>, ListSongID, RodioMpsc<PlaySongResponse>),
    Stop(ListSongID, RodioOneshot<()>),
    PausePlay(ListSongID, RodioOneshot<PlayActionTaken>),
    IncreaseVolume(i8, RodioOneshot<Percentage>),
    Seek(i8, RodioOneshot<(Duration, ListSongID)>),
}

#[derive(Debug)]
pub enum PlaySongResponse {
    ProgressUpdate(Duration),
    StartedPlaying(Option<Duration>),
    Queued(Option<Duration>),
    AutoplayingQueued,
    StoppedPlaying,
    Error(DecoderError),
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

impl<T> std::fmt::Debug for RodioOneshot<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oneshot channel - {}", std::any::type_name::<T>())
    }
}

/// Newtype for mpsc channel with custom derive
pub struct RodioMpsc<T>(mpsc::Sender<T>);

pub fn rodio_mpsc_channel<T>(buffer: usize) -> (RodioMpsc<T>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(buffer);
    (RodioMpsc(tx), rx)
}

impl<T> std::fmt::Debug for RodioMpsc<T>
where
    T: std::fmt::Debug,
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

/// Playable song doubling up as a RAII guard that will send a message once it
/// has finished playing.
struct DroppableSong {
    song: Arc<InMemSong>,
    // Song ID is stored for debugging purposes only - the receiver already knows the Song ID.
    song_id: ListSongID,
    channel: mpsc::Sender<PlaySongResponse>,
}
impl AsRef<[u8]> for DroppableSong {
    fn as_ref(&self) -> &[u8] {
        self.song.0.as_ref()
    }
}
impl Drop for DroppableSong {
    fn drop(&mut self) {
        debug!("DroppableSong {:?} was dropped!", self.song_id);
        self.channel
            .blocking_send(PlaySongResponse::StoppedPlaying)
            .unwrap_or_else(|e| {
                error!("Tried to send StoppedPlaying message but reciever was closed {e}")
            })
    }
}

pub fn spawn_rodio_thread(mut msg_rx: mpsc::Receiver<RodioMessage>) {
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
        // NOTE: the OutputStream is not Send, hence why this requires a blocking task.
        let (_stream, stream_handle) =
            rodio::OutputStream::try_default().expect("Expect to get a handle to output stream");
        let sink = rodio::Sink::try_new(&stream_handle).expect("Expect music player not to error");
        // Hopefully someone else can't create a song with the same ID?!
        let mut cur_song_duration = None;
        let mut next_song_duration = None;
        let mut cur_song_id = None;
        let mut next_song_id = None;
        while let Some(msg) = msg_rx.blocking_recv() {
            debug!("Rodio received {:?}", msg);
            match msg {
                RodioMessage::AutoplaySong(song_pointer, song_id, tx) => {
                    if Some(song_id) == next_song_id {
                        info!(
                            "Received autoplay for {:?}, it's already queued up. It will play automatically.",
                            song_id
                        );
                        cur_song_id = Some(song_id);
                        next_song_id = None;
                        cur_song_duration = next_song_duration;
                        next_song_duration = None;
                        blocking_send_or_error(
                            tx.0,
                            PlaySongResponse::StartedPlaying(cur_song_duration),
                        );
                        continue;
                    }
                    if Some(song_id) == cur_song_id {
                        error!(
                            "Received autoplay for {:?}, it's already playing. I was expecting it to be queued up.",
                            song_id
                        );
                        blocking_send_or_error(
                            tx.0,
                            PlaySongResponse::StartedPlaying(cur_song_duration),
                        );
                        continue;
                    }
                    info!(
                        "Autoplaying a song that wasn't queued; clearing queue. Queued: {:?}",
                        next_song_id
                    );
                    // DUPLICATE FROM PLAYSONG
                    let source = match try_decode(song_pointer, song_id, tx.0.clone()) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to play song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            blocking_send_or_error(tx.0, PlaySongResponse::Error(e));
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
                    blocking_send_or_error(
                        tx.0,
                        PlaySongResponse::StartedPlaying(cur_song_duration),
                    );
                    cur_song_id = Some(song_id);
                    next_song_id = None;
                    next_song_duration = None;
                    // END DUPLICATE
                }
                RodioMessage::QueueSong(song_pointer, song_id, tx) => {
                    // DUPLICATE FROM PLAYSONG
                    let source = match try_decode(song_pointer, song_id, tx.0.clone()) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to decode queued song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            blocking_send_or_error(&tx.0, PlaySongResponse::Error(e));
                            continue;
                        }
                    };
                    // END DUPLICATE
                    if sink.empty() {
                        error!("Tried to queue up a song, but sink was empty... Continuing anyway");
                    }
                    next_song_duration = source.total_duration();
                    blocking_send_or_error(&tx.0, PlaySongResponse::Queued(next_song_duration));
                    sink.append(source);
                    next_song_id = Some(song_id);
                }
                RodioMessage::PlaySong(song_pointer, song_id, tx) => {
                    let source = match try_decode(song_pointer, song_id, tx.0.clone()) {
                        Ok(source) => source,
                        Err(e) => {
                            error!("Received error when trying to play song <{e}>");
                            if !sink.empty() {
                                sink.stop()
                            }
                            blocking_send_or_error(&tx.0, PlaySongResponse::Error(e));
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
                    blocking_send_or_error(
                        tx.0,
                        PlaySongResponse::StartedPlaying(cur_song_duration),
                    );
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
                    // It seems that there is a race condition with seeking a paused track in rodio
                    // itself. This delay is sufficient.
                    std::thread::sleep(Duration::from_millis(5));
                    oneshot_send_or_error(tx.0, (sink.get_pos(), cur_song_id));
                }
            }
        }
    });
}

fn try_decode(
    song: Arc<InMemSong>,
    song_id: ListSongID,
    tx: mpsc::Sender<PlaySongResponse>,
) -> std::result::Result<
    PeriodicAccess<
        TrackPosition<Decoder<Cursor<DroppableSong>>>,
        impl FnMut(&mut TrackPosition<Decoder<Cursor<DroppableSong>>>),
    >,
    DecoderError,
> {
    // DUPLICATE FROM PLAYSONG
    let sp = DroppableSong {
        song,
        song_id,
        channel: tx.clone(),
    };
    let cur = std::io::Cursor::new(sp);
    rodio::Decoder::new(cur).map(move |s| {
        s.track_position()
            .periodic_access(PROGRESS_UPDATE_DELAY, move |s| {
                blocking_send_or_error(&tx, PlaySongResponse::ProgressUpdate(s.get_pos()));
            })
    })
}
