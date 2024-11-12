use super::downloader::InMemSong;
use crate::app::structures::ListSongID;
use crate::app::structures::Percentage;
use crate::core::send_or_error;
use crate::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;

const PLAYER_MSG_QUEUE_SIZE: usize = 256;
const PROGRESS_UPDATE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub enum Response {
    DonePlaying(ListSongID),
    Playing(Option<Duration>, ListSongID),
    Queued(Option<Duration>, ListSongID),
    AutoplayQueued(ListSongID),
    Error(ListSongID),
}

pub struct Player {
    rodio_tx: mpsc::Sender<RodioMessage>,
}

// Consider if this can be managed by Server.
impl Player {
    pub fn new() -> Self {
        let (msg_tx, msg_rx) = mpsc::channel(PLAYER_MSG_QUEUE_SIZE);
        spawn_rodio_thread(msg_rx);
        Self { rodio_tx: msg_tx }
    }
}
