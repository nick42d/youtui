use crate::config::{ApiKey, Config};
pub use messages::*;
use rusty_ytdl::reqwest;
use std::sync::Arc;
mod messages;

pub mod api;
pub mod api_error_handler;
pub mod player;
pub mod song_downloader;
pub mod song_thumbnail_downloader;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.
const MAX_RETRIES: usize = 5;
const AUDIO_QUALITY: rusty_ytdl::VideoQuality = rusty_ytdl::VideoQuality::HighestAudio;

pub type ArcServer = Arc<Server>;

/// Application backend that is capable of spawning concurrent tasks in response
/// to requests. Tasks each receive a handle to respond back to the caller.
pub struct Server {
    pub api: api::Api,
    pub player: player::Player,
    pub song_downloader: song_downloader::SongDownloader,
    pub song_thumbnail_downloader: song_thumbnail_downloader::SongThumbnailDownloader,
    pub api_error_handler: api_error_handler::ApiErrorHandler,
}

impl Server {
    pub fn new(api_key: ApiKey, po_token: Option<String>, config: &Config) -> Server {
        // Cheaply cloneable reqwest client to share amongst services.
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Expected reqwest client build to succeed");
        let api = api::Api::new(api_key, client.clone());
        let player = player::Player::new();
        let song_downloader =
            song_downloader::SongDownloader::new(po_token, client.clone(), config);
        let song_thumbnail_downloader =
            song_thumbnail_downloader::SongThumbnailDownloader::new(client);
        let api_error_handler = api_error_handler::ApiErrorHandler::new();
        Server {
            api,
            player,
            song_downloader,
            api_error_handler,
            song_thumbnail_downloader,
        }
    }
}
