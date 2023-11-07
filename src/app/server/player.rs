use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc;

use tracing::info;
use tracing::trace;

use crate::core::blocking_send_or_error;
use crate::Result;

use crate::app::structures::ListSongID;
use crate::app::taskmanager::TaskID;

use super::KillableTask;

const POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(100);
const PLAYER_MSG_QUEUE_SIZE: usize = 256;

#[derive(Debug)]
pub enum Request {
    PlaySong(Arc<Vec<u8>>, ListSongID),
    GetProgress(ListSongID), // Should give ID?
    GetVolume(KillableTask),
    IncreaseVolume(i8, TaskID),
    Stop,
    PausePlay,
}

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    Paused(ListSongID),
    Playing(ListSongID),
    Stopped,
    ProgressUpdate(f64, ListSongID),
    VolumeUpdate(u8, TaskID), // Should be Percentage
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
    pub async fn handle_request(&self, request: Request) {
        self.msg_tx.send(request).await;
    }
}

pub fn spawn_rodio_thread(
    mut msg_rx: mpsc::Receiver<Request>,
    response_tx: mpsc::Sender<super::Response>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        // Rodio can produce output to stderr when we don't want it to, so we use Gag to suppress stdout/stderr.
        // The downside is that even though this runs in a seperate thread all stderr for the whole app may be is gagged.
        // Also seems to spew out characters?
        // TODO: also handle the errors from Rodio, or write to a file.
        let _gag_sterr = gag::Gag::stderr().unwrap();

        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        let mut last_tick_time;
        let mut cur_song_elapsed = std::time::Duration::default();
        // Hopefully someone else can't create a song with the same ID?!
        let mut cur_song_id = ListSongID::default();
        let mut thinks_is_playing = false;
        loop {
            if let Ok(msg) = msg_rx.try_recv() {
                match msg {
                    Request::PlaySong(song_pointer, id) => {
                        // XXX: Perhaps should let the state know that we are playing.
                        info!("Got message to play song");
                        // TODO: remove allocation
                        let owned_song =
                            Arc::try_unwrap(song_pointer).unwrap_or_else(|arc| (*arc).clone());
                        let cur = std::io::Cursor::new(owned_song);
                        let source = rodio::Decoder::new(cur).unwrap();
                        if !sink.empty() {
                            sink.stop()
                        }
                        sink.append(source);
                        trace!("Now playing {:?}", id);
                        cur_song_elapsed = Duration::default();
                        cur_song_id = id;
                        thinks_is_playing = true;
                    }
                    Request::Stop => {
                        sink.stop();
                        // No need to send a message - will be triggered below.
                    }
                    Request::PausePlay => {
                        if sink.is_paused() {
                            sink.play();
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::Playing(cur_song_id)),
                            );
                        // We don't want to pause if sink is empty (but case could be handled in Playlist also)
                        } else if !sink.is_paused() && !sink.empty() {
                            sink.pause();
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::Paused(cur_song_id)),
                            );
                        }
                    }
                    Request::GetProgress(id) => {
                        info!("Got message to provide song progress update");
                        if cur_song_id == id {
                            blocking_send_or_error(
                                &response_tx,
                                super::Response::Player(Response::ProgressUpdate(
                                    cur_song_elapsed.as_secs_f64(),
                                    id,
                                )),
                            );
                            info!("Sending song progress update");
                        }
                    }
                    Request::GetVolume(task) => {
                        // TODO: Implment ability to kill this task using kill_rx.
                        let KillableTask { id, .. } = task;
                        info!("Received get volume message");
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::VolumeUpdate(
                                (sink.volume() * 100.0).round() as u8,
                                id,
                            )),
                        );
                        info!("Sending volume update");
                    }
                    Request::IncreaseVolume(vol_inc, id) => {
                        info!("Received {:?}", msg);
                        sink.set_volume(sink.volume() + vol_inc as f32 / 100.0);
                        blocking_send_or_error(
                            &response_tx,
                            super::Response::Player(Response::VolumeUpdate(
                                (sink.volume() * 100.0) as u8,
                                id,
                            )),
                        );
                        info!("Sending volume update");
                    }
                }
            }
            if !sink.empty() && !sink.is_paused() {
                last_tick_time = std::time::Instant::now();
                std::thread::sleep(POLL_INTERVAL);
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
