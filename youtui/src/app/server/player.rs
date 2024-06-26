use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc;

use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

use crate::app::structures::Percentage;
use crate::core::blocking_send_or_error;
use crate::Result;

use crate::app::structures::ListSongID;
use crate::app::taskmanager::TaskID;

use super::KillableTask;

const EVENT_POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(10);
const PROGRESS_UPDATE_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(100);
const PLAYER_MSG_QUEUE_SIZE: usize = 256;

#[derive(Debug)]
pub enum Request {
    GetVolume(KillableTask),
    IncreaseVolume(i8, TaskID),
    PlaySong(Arc<Vec<u8>>, ListSongID, TaskID),
    GetPlayProgress(ListSongID, TaskID), // Should give ID?
    Stop(ListSongID, TaskID),
    PausePlay(ListSongID, TaskID),
}

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    Paused(ListSongID, TaskID),
    Playing(ListSongID, TaskID),
    Stopped(ListSongID, TaskID),
    ProgressUpdate(f64, ListSongID, TaskID),
    VolumeUpdate(Percentage, TaskID), // Should be Percentage
}

pub struct PlayerManager {
    _response_tx: mpsc::Sender<super::Response>,
    _rodio: JoinHandle<()>,
    msg_tx: mpsc::Sender<Request>,
}

// Consider if this can be managed by Server.
impl PlayerManager {
    pub fn new(response_tx: mpsc::Sender<super::Response>) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        let response_tx_clone = response_tx.clone();
        let rodio = spawn_rodio_thread(msg_rx, response_tx_clone);
        Ok(Self {
            _response_tx: response_tx,
            msg_tx,
            _rodio: rodio,
        })
    }
    pub async fn handle_request(&self, request: Request) -> Result<()> {
        Ok(self.msg_tx.send(request).await?)
    }
}

pub fn spawn_rodio_thread(
    mut msg_rx: mpsc::Receiver<Request>,
    response_tx: mpsc::Sender<super::Response>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        // Rodio can produce output to stderr when we don't want it to, so we use Gag to
        // suppress stdout/stderr. The downside is that even though this runs in
        // a seperate thread all stderr for the whole app may be gagged.
        // Also seems to spew out characters?
        // TODO: also handle the errors from Rodio, or write to a file.
        let _gag = match gag::Gag::stderr() {
            Ok(gag) => gag,
            Err(e) => {
                warn!("Error <{e}> gagging stderr output");
                return;
            }
        };
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        let mut last_tick_time;
        let mut cur_song_elapsed = std::time::Duration::default();
        // Hopefully someone else can't create a song with the same ID?!
        let mut cur_song_id = ListSongID::default();
        let mut thinks_is_playing = false;
        loop {
            while let Ok(msg) = msg_rx.try_recv() {
                match msg {
                    Request::PlaySong(song_pointer, song_id, id) => {
                        // XXX: Perhaps should let the state know that we are playing.
                        info!("Got message to play song {:?}", id);
                        // TODO: remove allocation
                        let owned_song =
                            Arc::try_unwrap(song_pointer).unwrap_or_else(|arc| (*arc).clone());
                        let cur = std::io::Cursor::new(owned_song);
                        let source = rodio::Decoder::new(cur).unwrap();
                        if !sink.empty() {
                            sink.stop()
                        }
                        sink.append(source);
                        // Handle case we're we've received a play message but queue was paused.
                        if sink.is_paused() {
                            sink.play();
                        }
                        debug!("Now playing {:?}", id);
                        // Send the Now Playing message for good orders sake to avoid
                        // synchronization issues.
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::Playing(song_id, id)),
                        );
                        cur_song_elapsed = Duration::default();
                        cur_song_id = song_id;
                        thinks_is_playing = true;
                    }
                    Request::Stop(song_id, id) => {
                        info!("Got message to stop playing {:?}", song_id);
                        if cur_song_id != song_id {
                            continue;
                        }
                        if !sink.empty() {
                            sink.stop()
                        }
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::Stopped(song_id, id)),
                        );
                        thinks_is_playing = false;
                    }
                    Request::PausePlay(song_id, id) => {
                        info!("Got message to pause / play {:?}", id);
                        if cur_song_id != song_id {
                            continue;
                        }
                        if sink.is_paused() {
                            sink.play();
                            info!("Sending Play message {:?}", id);
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::Playing(song_id, id)),
                            );
                        // We don't want to pause if sink is empty (but case
                        // could be handled in Playlist also)
                        } else if !sink.is_paused() && !sink.empty() {
                            sink.pause();
                            info!("Sending Pause message {:?}", id);
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::Paused(song_id, id)),
                            );
                        }
                    }
                    // XXX: May be able to handle this by reporting progress updates when playing
                    // instead of needing to request/response here.
                    Request::GetPlayProgress(song_id, id) => {
                        debug!("Got message to provide song progress update");
                        if cur_song_id == song_id {
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::ProgressUpdate(
                                    cur_song_elapsed.as_secs_f64(),
                                    song_id,
                                    id,
                                )),
                            );
                            debug!("Sending song progress update");
                        }
                    }
                    // XXX: Should this just be IncreaseVolume(0)?
                    Request::GetVolume(task) => {
                        // TODO: Implment ability to kill this task using kill_rx.
                        let KillableTask { id, .. } = task;
                        info!("Received get volume message");
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::VolumeUpdate(
                                Percentage((sink.volume() * 100.0).round() as u8),
                                id,
                            )),
                        );
                        info!("Sending volume update");
                    }
                    Request::IncreaseVolume(vol_inc, id) => {
                        info!("Received {:?}", msg);
                        sink.set_volume((sink.volume() + vol_inc as f32 / 100.0).clamp(0.0, 1.0));
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::VolumeUpdate(
                                Percentage((sink.volume() * 100.0).round() as u8),
                                id,
                            )),
                        );
                        info!("Sending volume update");
                    }
                }
            }
            // Avoid empty infinite loop, but still poll more frequently than when sending
            // progress updates for responsiveness. TODO: Maintain the
            // responsiveness whilst still sending progress updates.
            // TODO: Better architecture for this component in general.
            last_tick_time = std::time::Instant::now();
            std::thread::sleep(EVENT_POLL_INTERVAL);
            if !sink.empty() && !sink.is_paused() {
                std::thread::sleep(PROGRESS_UPDATE_INTERVAL.saturating_sub(EVENT_POLL_INTERVAL));
                let passed = std::time::Instant::now() - last_tick_time;
                cur_song_elapsed = cur_song_elapsed + passed;
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
    })
}
