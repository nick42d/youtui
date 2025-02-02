use crate::config::ApiKey;
use std::sync::Arc;

pub use messages::*;
mod messages;

pub mod api;
pub mod api_error_handler;
pub mod downloader;
pub mod player;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.
const MAX_RETRIES: usize = 5;
const AUDIO_QUALITY: rusty_ytdl::VideoQuality = rusty_ytdl::VideoQuality::HighestAudio;

pub type ArcServer = Arc<Server>;

/// Application backend that is capable of spawning concurrent tasks in response
/// to requests. Tasks each receive a handle to respond back to the caller.
pub struct Server {
    pub api: api::Api,
    pub player: player::Player,
    pub downloader: downloader::Downloader,
    pub api_error_handler: api_error_handler::ApiErrorHandler,
}

impl Server {
    pub fn new(api_key: ApiKey, po_token: Option<String>) -> Server {
        let api = api::Api::new(api_key);
        let player = player::Player::new();
        let downloader = downloader::Downloader::new(po_token);
        let json_logger = api_error_handler::ApiErrorHandler::new();
        Server {
            api,
            player,
            downloader,
            api_error_handler: json_logger,
        }
    }
}
