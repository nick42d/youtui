use super::taskmanager::{KillableTask, TaskID};
use crate::{config::ApiKey, Result};
use api::ConcurrentApi;
use futures::{future::Shared, Future};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};
use ytmapi_rs::common::{ArtistChannelID, SearchSuggestion};

pub use messages::*;

pub mod api;
pub mod downloader;
pub mod messages;
pub mod player;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.
const MAX_RETRIES: usize = 5;
const AUDIO_QUALITY: rusty_ytdl::VideoQuality = rusty_ytdl::VideoQuality::HighestAudio;

/// Application backend that is capable of spawning concurrent tasks in response
/// to requests. Tasks each receive a handle to respond back to the caller.
pub struct Server {
    api: api::Api,
    player: player::Player,
    downloader: downloader::Downloader,
}

impl Server {
    pub fn new(api_key: ApiKey) -> Server {
        let api = api::Api::new(api_key);
        let player = player::Player::new();
        let downloader = downloader::Downloader::new();
        Server {
            api,
            player,
            downloader,
        }
    }
    pub async fn get_search_suggestions(&self, query: String) -> Result<Vec<SearchSuggestion>> {
        api::get_search_suggestions(self.api.get_api().await?, query).await
    }
}
pub struct GetSearchSuggestions(String);
pub struct NewArtistSearch(String);
pub struct SearchSelectedArtist(ArtistChannelID<'static>);

impl async_callback_manager::BackendTask<Server> for GetSearchSuggestions {
    type Output = Result<Vec<SearchSuggestion>>;
    fn into_future(self, backend: &Server) -> impl Future<Output = Self::Output> + Send + 'static {
        backend.get_search_suggestions(self.0)
    }
}
impl async_callback_manager::BackendTask<Server> for NewArtistSearch {
    type Output = ();
    fn into_future(self, backend: &Server) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!()
    }
}
impl async_callback_manager::BackendStreamingTask<Server> for SearchSelectedArtist {
    type Output = ();
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        todo!()
    }
}
