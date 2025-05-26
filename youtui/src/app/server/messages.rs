use super::api::GetArtistSongsProgressUpdate;
use super::player::{DecodedInMemSong, Player};
use super::song_downloader::{DownloadProgressUpdate, InMemSong};
use super::ArcServer;
use crate::app::structures::ListSongID;
use crate::async_rodio_sink::rodio::decoder::DecoderError;
use crate::async_rodio_sink::{
    AllStopped, AutoplayUpdate, PausePlayResponse, Paused, PlayUpdate, ProgressUpdate, QueueUpdate,
    Resumed, SeekDirection, Stopped, VolumeUpdate,
};
use anyhow::{Error, Result};
use async_callback_manager::{BackendStreamingTask, BackendTask};
use futures::{Future, Stream};
use std::sync::Arc;
use std::time::Duration;
use ytmapi_rs::common::{ArtistChannelID, SearchSuggestion, VideoID};
use ytmapi_rs::parse::{SearchResultArtist, SearchResultSong};

#[derive(PartialEq, Debug)]
pub enum TaskMetadata {
    PlayingSong,
    PlayPause,
}

#[derive(Debug)]
pub struct HandleApiError {
    pub error: Error,
    pub message: String,
}

#[derive(Debug)]
pub struct GetSearchSuggestions(pub String);
#[derive(Debug)]
pub struct SearchArtists(pub String);
#[derive(Debug)]
pub struct SearchSongs(pub String);
#[derive(Debug)]
pub struct GetArtistSongs(pub ArtistChannelID<'static>);

#[derive(Debug)]
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
// New note - 2025:
// SetVolume should be able to kill IncreaseVolume however...
#[derive(Debug)]
pub struct IncreaseVolume(pub i8);
#[derive(Debug)]
pub struct SetVolume(pub u8);
/// Seek forwards or backwards a duration in a song.
#[derive(Debug)]
pub struct Seek {
    pub duration: Duration,
    pub direction: SeekDirection,
}
/// Seek to a target position in a song.
#[derive(Debug)]
pub struct SeekTo {
    pub position: Duration,
    // Unlike seeking forward or back, it would be odd if user was expecting to seek to pos x in
    // song a but due to a race condition seek applied to song b.
    pub id: ListSongID,
}
/// Stop a song if it is still currently playing.
#[derive(Debug)]
pub struct Stop(pub ListSongID);
/// Stop the player, regardless of what song is playing.
#[derive(Debug)]
pub struct StopAll;
#[derive(Debug)]
pub struct PausePlay(pub ListSongID);
#[derive(Debug)]
pub struct Resume(pub ListSongID);
#[derive(Debug)]
pub struct Pause(pub ListSongID);
/// Decode a song into a format that can be played.
#[derive(Debug)]
pub struct DecodeSong(pub Arc<InMemSong>);
/// Play a song, starting from the start, regardless what's queued.
#[derive(Debug)]
pub struct PlaySong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}
/// Play a song, unless it's already queued.
#[derive(Debug)]
pub struct AutoplaySong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}
/// Queue a song to play next.
#[derive(Debug)]
pub struct QueueSong {
    pub song: DecodedInMemSong,
    pub id: ListSongID,
}

impl BackendTask<ArcServer> for HandleApiError {
    // Infallible - assumption is that even if this task fails, caller won't care.
    type Output = ();
    // TODO: Review if TaskMetadata needs new enum cases.
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let Self { error, message } = self;
        let backend = backend.clone();
        async move {
            backend.api_error_handler.handle_error(error, message).await;
        }
    }
}

impl BackendTask<ArcServer> for GetSearchSuggestions {
    // TODO: Consider alternative where the text isn't returned back to the caller.
    type Output = Result<(Vec<SearchSuggestion>, String)>;
    type MetadataType = TaskMetadata;
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
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.api.search_artists(self.0).await }
    }
}
impl BackendTask<ArcServer> for SearchSongs {
    type Output = Result<Vec<SearchResultSong>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.api.search_songs(self.0).await }
    }
}
impl BackendStreamingTask<ArcServer> for GetArtistSongs {
    type Output = GetArtistSongsProgressUpdate;
    type MetadataType = TaskMetadata;
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
    type MetadataType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.song_downloader.download_song(self.0, self.1)
    }
}
impl BackendTask<ArcServer> for Seek {
    type Output = Option<ProgressUpdate<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.seek(self.duration, self.direction).await }
    }
}
impl BackendTask<ArcServer> for SeekTo {
    type Output = Option<ProgressUpdate<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.seek_to(self.position, self.id).await }
    }
}
impl BackendTask<ArcServer> for DecodeSong {
    type Output = std::result::Result<DecodedInMemSong, DecoderError>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        _backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        Player::try_decode(self.0)
    }
}
impl BackendTask<ArcServer> for IncreaseVolume {
    type Output = Option<VolumeUpdate>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.increase_volume(self.0).await }
    }
}
impl BackendTask<ArcServer> for SetVolume {
    type Output = Option<VolumeUpdate>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.set_volume(self.0).await }
    }
}
impl BackendTask<ArcServer> for Stop {
    type Output = Option<Stopped<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.stop(self.0).await }
    }
}
impl BackendTask<ArcServer> for StopAll {
    type Output = Option<AllStopped>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.stop_all().await }
    }
}
impl BackendTask<ArcServer> for PausePlay {
    type Output = Option<PausePlayResponse<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.pause_play(self.0).await }
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayPause]
    }
}
impl BackendTask<ArcServer> for Resume {
    type Output = Option<Resumed<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.resume(self.0).await }
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayPause]
    }
}
impl BackendTask<ArcServer> for Pause {
    type Output = Option<Paused<ListSongID>>;
    type MetadataType = TaskMetadata;
    fn into_future(
        self,
        backend: &ArcServer,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move { backend.player.pause(self.0).await }
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayPause]
    }
}

impl BackendStreamingTask<ArcServer> for PlaySong {
    type Output = PlayUpdate<ListSongID>;
    type MetadataType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.play_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayingSong]
    }
}
impl BackendStreamingTask<ArcServer> for AutoplaySong {
    type Output = AutoplayUpdate<ListSongID>;
    type MetadataType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.autoplay_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayingSong]
    }
}
impl BackendStreamingTask<ArcServer> for QueueSong {
    type Output = QueueUpdate<ListSongID>;
    type MetadataType = TaskMetadata;
    fn into_stream(
        self,
        backend: &ArcServer,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        backend.player.queue_song(self.song, self.id)
    }
    fn metadata() -> Vec<Self::MetadataType> {
        vec![TaskMetadata::PlayingSong]
    }
}
