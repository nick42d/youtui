use anyhow::Result;
use player::{Guard, Player, PlayerOptions, StreamError};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::error;

use tracing::info;
use tracing::trace;

use crate::core::send_or_error;

use super::ui::structures::ListSongID;

const INITIAL_VOLUME: u8 = 50;
const POLL_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_millis(200);

#[derive(Debug)]
pub enum Request {
    PlaySong(std::path::PathBuf, ListSongID),
    PlaySongMem(Arc<Vec<u8>>, ListSongID),
    GetProgress(ListSongID), // Should give ID?
    GetVolume,
    IncreaseVolume(i8),
    Stop,
}

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    ProgressUpdate(f64, ListSongID),
    VolumeUpdate(u8),
}

pub struct PlayerManager {
    player: Player,
    guard: Arc<Mutex<Guard>>,
    response_tx: mpsc::Sender<Response>,
    request_rx: mpsc::Receiver<Request>,
    _stream: rodio::OutputStream,
    stream_handle: rodio::OutputStreamHandle,
}

impl PlayerManager {
    pub fn new(
        response_tx: mpsc::Sender<Response>,
        request_rx: mpsc::Receiver<Request>,
    ) -> Result<Self> {
        let opts = PlayerOptions { initial_volume: 50 };
        let (tx, _rx) = flume::unbounded::<StreamError>();
        let (player, guard) = Player::new(std::sync::Arc::new(tx), opts)?;
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        Ok(Self {
            player,
            guard: Arc::new(Mutex::new(guard)),
            request_rx,
            response_tx,
            _stream,
            stream_handle,
        })
    }
    pub async fn handle_message(&mut self) {
        // Note - we are only processing these on each event.
        // This means the Get Volume is a little laggy as it does not ask UI to refresh after sending.
        let player = &mut self.player;
        if let Ok(msg) = self.request_rx.try_recv() {
            match msg {
                Request::PlaySongMem(song_pointer, _id) => {
                    // XXX: Perhaps should let the state know that we are playing.
                    info!("Got message to play song");
                    // TODO: remove allocation
                    let owned_song = Arc::unwrap_or_clone(song_pointer);
                    let cur = std::io::Cursor::new(owned_song);
                    let source = rodio::Decoder::new(cur).unwrap();
                    let sink = rodio::Sink::try_new(&self.stream_handle).unwrap();
                    sink.append(source);
                    trace!("Now playing {:?}", _id);
                    sink.sleep_until_end();
                }
                Request::PlaySong(path, _id) => {
                    // XXX: Perhaps should let the state know that we are playing.
                    info!("Got message to play song");
                    let guard = self.guard.lock().unwrap();
                    player
                        .play(&path, &guard)
                        .unwrap_or_else(|e| error!("Error <{e}> playing song"));
                    trace!("Now playing {:?}", path);
                }
                Request::GetProgress(id) => {
                    info!("Got message to provide song progress update");
                    let progress = player.elapsed().as_secs_f64();
                    // send_or_error(&self.response_tx, Response::ProgressUpdate(progress, id)).await;
                    // if player.is_finished() {
                    //     send_or_error(&self.response_tx, Response::DonePlaying(id)).await;
                    //     info!("Song finished");
                    // }
                }
                Request::GetVolume => {
                    info!("Received {:?}", msg);
                    let vol = player.volume_percent();
                    send_or_error(&self.response_tx, Response::VolumeUpdate(vol)).await;
                    info!("Sending volume update message");
                }
                Request::IncreaseVolume(vol_inc) => {
                    info!("Received {:?}", msg);
                    let vol = player.volume_percent();
                    let new_vol_perc = vol.checked_add_signed(vol_inc).unwrap_or(0).min(100);
                    player.set_volume(new_vol_perc as i32);
                    send_or_error(&self.response_tx, Response::VolumeUpdate(new_vol_perc)).await;
                    info!("Sending volume update message");
                }
                Request::Stop => {
                    // XXX: Perhaps should let the state know that we are stopping.
                    trace!("Received stop message");
                    let guard = self.guard.lock().unwrap();
                    player.stop(&guard).unwrap();
                }
            }
        }
    }
}
