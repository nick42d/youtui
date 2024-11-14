use super::structures::ListSongID;
use crate::{config::ApiKey, Result};
use api::GetArtistSongsProgressUpdate;
use async_callback_manager::{BackendStreamingTask, BackendTask};
use async_rodio_sink::rodio::decoder::DecoderError;
use async_rodio_sink::AutoplayUpdate;
use async_rodio_sink::PausePlayResponse;
use async_rodio_sink::PlayUpdate;
use async_rodio_sink::ProgressUpdate;
use async_rodio_sink::QueueUpdate;
use async_rodio_sink::SeekDirection;
use async_rodio_sink::Stopped;
use async_rodio_sink::VolumeUpdate;
use downloader::DownloadProgressUpdate;
use downloader::InMemSong;
use futures::Future;
use futures::Stream;
use player::DecodedInMemSong;
use player::Player;
use std::sync::Arc;
use std::time::Duration;
use ytmapi_rs::common::VideoID;
use ytmapi_rs::common::{ArtistChannelID, SearchSuggestion};
use ytmapi_rs::parse::SearchResultArtist;

pub mod api;
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
}
#[derive(PartialEq, Debug)]
pub enum TaskMetadata {
    PlayingSong,
    PlayPause,
}

pub struct GetSearchSuggestions(pub String);
pub struct SearchArtists(pub String);
pub struct GetArtistSongs(pub ArtistChannelID<'static>);

pub struct DownloadSong(pub VideoID<'static>, pub ListSongID);

// Player Requests documentation:
// NOTE: I considered giving player more control of the playback than playlist,
// and increasing message size. However this seems to be more combinatorially
// difficult without a well defined data structure.

// XXX: This should be programmed to be unkillable.
// Case:
// Cur volume: 5
// Send IncreaseVolume(5)
// Send IncreaseVolume(5), killing previous task
// Volume will now be 10 - should be 15, should not allow caller to cause this.
pub struct IncreaseVolume(pub i8);
pub struct Seek {
    pub duration: Duration,
    pub direction: SeekDirection,
}
pub struct Stop(pub ListSongID);
pub struct PausePlay(pub ListSongID);
/// Decode a song into a format that can be played.
pub struct DecodeSong(pub Arc<InMemSong>);
// Play a song, starting from the start, regardless what's queued.
pub struct PlaySong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}
// Play a song, unless it's already queued.
pub struct AutoplaySong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}
// Queue a song to play next.
pub struct QueueSong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}

impl BackendTask<ArcServer> for GetSearchSuggestions {
    // TODO: Consider alternative where the text isn't returned back to the caller.
    type Output = Result<(Vec<SearchSuggestion>, String)>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.api.get_search_suggestions(self.0).await }
    }
}
impl BackendTask<ArcServer> for SearchArtists {
    type Output = Result<Vec<SearchResultArtist>>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.api.search_artists(self.0).await }
    }
}
impl BackendStreamingTask<ArcServer> for GetArtistSongs {
    type Output = GetArtistSongsProgressUpdate;
    type ConstraintType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.api.get_artist_songs(self.0)
    }
}

impl BackendStreamingTask<ArcServer> for DownloadSong {
    type Output = DownloadProgressUpdate;
    type ConstraintType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.downloader.download_song(self.0, self.1)
    }
}
impl BackendTask<ArcServer> for Seek {
    type Output = Option<ProgressUpdate<ListSongID>>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.seek(self.duration, self.direction).await }
    }
}
impl BackendTask<ArcServer> for DecodeSong {
    type Output = std::result::Result<DecodedInMemSong, DecoderError>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        _backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        Player::try_decode(self.0)
    }
}
impl BackendTask<ArcServer> for IncreaseVolume {
    type Output = Option<VolumeUpdate>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.increase_volume(self.0).await }
    }
}
impl BackendTask<ArcServer> for Stop {
    type Output = Option<Stopped<ListSongID>>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.stop(self.0).await }
    }
}
impl BackendTask<ArcServer> for PausePlay {
    type Output = Option<PausePlayResponse<ListSongID>>;
    type ConstraintType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.pause_play(self.0).await }
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![TaskMetadata::PlayPause]
    }
}

impl BackendStreamingTask<ArcServer> for PlaySong {
    type Output = PlayUpdate<ListSongID>;
    type ConstraintType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.play_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![TaskMetadata::PlayingSong]
    }
}
impl BackendStreamingTask<ArcServer> for AutoplaySong {
    type Output = AutoplayUpdate<ListSongID>;
    type ConstraintType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.autoplay_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![TaskMetadata::PlayingSong]
    }
}
impl BackendStreamingTask<ArcServer> for QueueSong {
    type Output = QueueUpdate<ListSongID>;
    type ConstraintType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.queue_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![TaskMetadata::PlayingSong]
    }
}
