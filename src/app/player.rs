use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc;

use tracing::info;
use tracing::trace;

use crate::core::blocking_send_or_error;
use crate::Result;

use super::structures::ListSongID;

const INITIAL_VOLUME: u8 = 50;
const POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(200);

#[derive(Debug)]
pub enum Request {
    PlaySong(Arc<Vec<u8>>, ListSongID),
    GetProgress(ListSongID), // Should give ID?
    GetVolume,
    IncreaseVolume(i8),
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
    VolumeUpdate(u8),
}

pub struct PlayerManager {
    response_tx: mpsc::Sender<Response>,
    rodio: JoinHandle<()>,
}

// Consider if this can be managed by Server.
impl PlayerManager {
    pub fn new(
        response_tx: mpsc::Sender<Response>,
        mut request_rx: mpsc::Receiver<Request>,
    ) -> Result<Self> {
        let response_tx_clone = response_tx.clone();
        let rodio = std::thread::spawn(move || {
            // Rodio can produce output to stderr when we don't want it to, so we use Gag to suppress stdout/stderr.
            // The downside is that even though this runs in a seperate thread all stderr for the whole app is gagged.
            // Also seems to spew out characters?
            // TODO: also handle the errors from Rodio, or write to a file.
            let _gag_sterr = gag::Gag::stderr().unwrap();

            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            let mut last_tick_time = std::time::Instant::now();
            let mut cur_song_elapsed = std::time::Duration::default();
            // Hopefully someone else can't create a song with the same ID?!
            let mut cur_song_id = ListSongID::default();
            let mut thinks_is_playing = false;
            loop {
                if let Ok(msg) = request_rx.try_recv() {
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
                                    &response_tx_clone,
                                    Response::Playing(cur_song_id),
                                );
                            // We don't want to pause if sink is empty (but case could be handled in Playlist also)
                            } else if !sink.is_paused() && !sink.empty() {
                                sink.pause();
                                blocking_send_or_error(
                                    &response_tx_clone,
                                    Response::Paused(cur_song_id),
                                );
                            }
                        }
                        Request::GetProgress(id) => {
                            info!("Got message to provide song progress update");
                            if cur_song_id == id {
                                blocking_send_or_error(
                                    &response_tx_clone,
                                    Response::ProgressUpdate(cur_song_elapsed.as_secs_f64(), id),
                                );
                                info!("Sending song progress update");
                            }
                        }
                        Request::GetVolume => {
                            info!("Received {:?}", msg);
                            // Because of the thread delay, these aren't processed immediately.
                            // Need an id in this message to keep track of it.
                            blocking_send_or_error(
                                &response_tx_clone,
                                Response::VolumeUpdate((sink.volume() * 100.0) as u8),
                            );
                            info!("Sending volume update");
                        }
                        Request::IncreaseVolume(vol_inc) => {
                            info!("Received {:?}", msg);
                            sink.set_volume(sink.volume() + vol_inc as f32 / 100.0);
                            blocking_send_or_error(
                                &response_tx_clone,
                                Response::VolumeUpdate((sink.volume() * 100.0) as u8),
                            );
                            info!("Sending volume update");
                        }
                    }
                }
                if !sink.empty() && !sink.is_paused() {
                    last_tick_time = std::time::Instant::now();
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let passed = std::time::Instant::now() - last_tick_time;
                    cur_song_elapsed = cur_song_elapsed + passed;
                }
                if sink.empty() && thinks_is_playing {
                    // NOTE: This simple model won't work if we have multiple songs in the sink.
                    // Instead we should keep track of number of songs and use sink.len().
                    trace!("Finished playing {:?}", cur_song_id);
                    thinks_is_playing = false;
                    blocking_send_or_error(&response_tx_clone, Response::DonePlaying(cur_song_id));
                }
            }
        });
        Ok(Self {
            response_tx: response_tx.clone(),
            rodio,
        })
    }
}
// pub fn handle_message(msg: Request, response_tx: Sender<Response>, sink: &mut rodio::Sink ) {
//     // Note - we are only processing these on each event.
//     // This means the Get Volume is a little laggy as it does not ask UI to refresh after sending.
//     if let Ok(msg) = self.request_rx.try_recv() {
//         match msg {
//             Request::PlaySong(song_pointer, id) => {
//                 info!("Got message to play song");
//                 self.rodio
//                     .tx
//                     .send(RodioMsg::PlaySongMem(song_pointer, id))
//                     .await;
//                 trace!("Now playing {:?}", id);
//             }
//             Request::GetProgress(id) => {
//                 info!("Got message to provide song progress update");
//                 warn!("Unhandled");
//             }
//             Request::GetVolume => {
//                 info!("Received {:?}", msg);
//                 send_or_error(&self.response_tx, Response::VolumeUpdate(50)).await;
//                 warn!("Unhandled - always sends 50");
//             }
//             Request::IncreaseVolume(vol_inc) => {
//                 info!("Received {:?}", msg);
//                 warn!("Unhandled");
//             }
//             Request::Stop => {
//                 // XXX: Perhaps should let the state know that we are stopping.
//                 trace!("Received stop message");
//                 warn!("Unhandled");
//             }
//         }
//     }
// }
