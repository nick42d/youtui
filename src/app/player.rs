// Currently this is not set up like the rest of our libraries with spawned handles and instead runs on the main thread.
// This is because the player library we are using wasn't conducive to this pattern.
// Full switch to Rodio will resolve this.
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::mpsc;
use tracing::warn;

use tracing::info;
use tracing::trace;

use crate::core::blocking_send_or_error;
use crate::core::send_or_error;
use crate::Result;

use super::ui::structures::ListSongID;

const INITIAL_VOLUME: u8 = 50;
const POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(200);

#[derive(Debug)]
pub enum Request {
    PlaySong(Arc<Vec<u8>>, ListSongID),
    GetProgress(ListSongID), // Should give ID?
    GetVolume,
    IncreaseVolume(i8),
    Stop,
}

#[derive(Debug)]
pub enum RodioMsg {
    PlaySongMem(Arc<Vec<u8>>, ListSongID),
}
#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    ProgressUpdate(f64, ListSongID),
    VolumeUpdate(u8),
}

pub struct PlayerManager {
    response_tx: mpsc::Sender<Response>,
    request_rx: mpsc::Receiver<Request>,
    rodio: RodioManager,
}

pub struct RodioManager {
    tx: mpsc::Sender<RodioMsg>,
    rodio: JoinHandle<()>,
}

impl RodioManager {
    fn new(
        tx: mpsc::Sender<RodioMsg>,
        mut rx: mpsc::Receiver<RodioMsg>,
        mgr_tx: mpsc::Sender<Response>,
    ) -> Self {
        Self {
            tx,
            // Rodio OutputStream is not Send and therefore we must spawn a standard thread and use blocking code here.
            rodio: std::thread::spawn(move || {
                let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
                loop {
                    if let Ok(msg) = rx.try_recv() {
                        match msg {
                            RodioMsg::PlaySongMem(song_pointer, id) => {
                                // XXX: Perhaps should let the state know that we are playing.
                                info!("Got message to play song");
                                // TODO: remove allocation
                                let owned_song = Arc::try_unwrap(song_pointer)
                                    .unwrap_or_else(|arc| (*arc).clone());
                                let cur = std::io::Cursor::new(owned_song);
                                let source = rodio::Decoder::new(cur).unwrap();
                                let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                                sink.append(source);
                                trace!("Now playing {:?}", id);
                                let play_start_time = std::time::Instant::now();
                                // Hack to implement song duration until Rodio implements elapsed.
                                while !sink.empty() {
                                    let now = std::time::Instant::now();
                                    let passed = now - play_start_time;
                                    let passed_secs = passed.as_secs_f64();
                                    blocking_send_or_error(
                                        &mgr_tx,
                                        Response::ProgressUpdate(passed_secs, id),
                                    );
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }

                                sink.sleep_until_end();
                                blocking_send_or_error(&mgr_tx, Response::DonePlaying(id));
                                trace!("Finished playing {:?}", id);
                            }
                        }
                    }
                }
            }),
        }
    }
}

impl PlayerManager {
    pub fn new(
        response_tx: mpsc::Sender<Response>,
        request_rx: mpsc::Receiver<Request>,
    ) -> Result<Self> {
        let (tx2, rx2) = mpsc::channel(256);
        Ok(Self {
            request_rx,
            response_tx: response_tx.clone(),
            rodio: RodioManager::new(tx2, rx2, response_tx),
        })
    }
    pub async fn handle_message(&mut self) {
        // Note - we are only processing these on each event.
        // This means the Get Volume is a little laggy as it does not ask UI to refresh after sending.
        if let Ok(msg) = self.request_rx.try_recv() {
            match msg {
                Request::PlaySong(song_pointer, id) => {
                    info!("Got message to play song");
                    self.rodio
                        .tx
                        .send(RodioMsg::PlaySongMem(song_pointer, id))
                        .await;
                    trace!("Now playing {:?}", id);
                }
                Request::GetProgress(id) => {
                    info!("Got message to provide song progress update");
                    warn!("Unhandled");
                }
                Request::GetVolume => {
                    info!("Received {:?}", msg);
                    send_or_error(&self.response_tx, Response::VolumeUpdate(50)).await;
                    warn!("Unhandled - always sends 50");
                }
                Request::IncreaseVolume(vol_inc) => {
                    info!("Received {:?}", msg);
                    warn!("Unhandled");
                }
                Request::Stop => {
                    // XXX: Perhaps should let the state know that we are stopping.
                    trace!("Received stop message");
                    warn!("Unhandled");
                }
            }
        }
    }
}
