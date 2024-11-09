use super::structures::ListSongID;
use crate::{config::ApiKey, Result};
use async_callback_manager::ArcBackendTask;
use async_callback_manager::{BackendStreamingTask, BackendTask};
use downloader::DownloadProgressUpdate;
use downloader::Downloader;
use downloader::InMemSong;
use futures::Future;
use futures::Stream;
use std::sync::Arc;
use ytmapi_rs::common::VideoID;
use ytmapi_rs::common::{ArtistChannelID, SearchSuggestion};

pub mod api;
pub mod downloader;
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
    pub fn new(api_key: ApiKey, po_token: Option<String>) -> Server {
        let api = api::Api::new(api_key);
        let player = player::Player::new();
        let downloader = downloader::Downloader::new(po_token);
        Server {
            api,
            player,
            downloader,
        }
    }
    pub async fn get_search_suggestions(&self, query: String) -> Result<Vec<SearchSuggestion>> {
        self.api.get_search_suggestions(query).await
    }
    pub fn download_song(
        &self,
        video_id: VideoID<'static>,
        song_id: ListSongID,
    ) -> impl Stream<Item = DownloadProgressUpdate> {
        self.downloader.download_song(video_id, song_id)
    }
}
pub struct GetSearchSuggestions(String);
pub struct NewArtistSearch(String);
pub struct SearchSelectedArtist(ArtistChannelID<'static>);

pub struct DownloadSong(VideoID<'static>, ListSongID);

// Player Requests documentation:
// NOTE: I considered giving player more control of the playback than playlist,
// and increasing message size. However this seems to be more combinatorially
// difficult without a well defined data structure.

// This should be set as unkillable.
// Case:
// Cur volume: 5
// Send IncreaseVolume(5)
// Send IncreaseVolume(5), killing previous task
// Volume will now be 10 - should be 15, should not allow caller to cause this.
pub struct IncreaseVolume(i8);
pub struct Seek(i8);
pub struct Stop(ListSongID);
pub struct PausePlay(ListSongID);
// Play a song, starting from the start, regardless what's queued.
pub struct PlaySong {
    song: Arc<InMemSong>,
    id: ListSongID,
}
// Play a song, unless it's already queued.
pub struct AutoplaySong {
    song: Arc<InMemSong>,
    id: ListSongID,
}
// Queue a song to play next.
pub struct QueueSong {
    song: Arc<InMemSong>,
    id: ListSongID,
}

impl ArcBackendTask<Server> for GetSearchSuggestions {
    type Output = Result<Vec<SearchSuggestion>>;
    fn into_future(
        self,
        backend: Arc<Server>,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.get_search_suggestions(self.0).await }
    }
}
impl BackendTask<Server> for NewArtistSearch {
    type Output = ();
    fn into_future(self, backend: &Server) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!();
        async {}
    }
}
impl BackendStreamingTask<Server> for SearchSelectedArtist {
    type Output = ();
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        todo!();
        futures::stream::empty()
    }
}

impl BackendStreamingTask<Server> for DownloadSong {
    type Output = DownloadProgressUpdate;
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        backend.download_song(self.0, self.1)
    }
}
impl BackendTask<Downloader> for IncreaseVolume {
    type Output = ();
    fn into_future(
        self,
        backend: &Downloader,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!();
        async {}
    }
}
impl BackendTask<Downloader> for Seek {
    type Output = ();
    fn into_future(
        self,
        backend: &Downloader,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!();
        async {}
    }
}
impl BackendTask<Downloader> for Stop {
    type Output = ();
    fn into_future(
        self,
        backend: &Downloader,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!();
        async {}
    }
}
impl BackendTask<Downloader> for PausePlay {
    type Output = ();
    fn into_future(
        self,
        backend: &Downloader,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        todo!();
        async {}
    }
}

impl BackendStreamingTask<Server> for PlaySong {
    type Output = ();
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        todo!();
        futures::stream::empty()
    }
}
impl BackendStreamingTask<Server> for AutoplaySong {
    type Output = ();
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        todo!();
        futures::stream::empty()
    }
}
impl BackendStreamingTask<Server> for QueueSong {
    type Output = ();
    fn into_stream(
        self,
        backend: &Server,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        todo!();
        futures::stream::empty()
    }
}
