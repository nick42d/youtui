use super::downloader::InMemSong;
use crate::app::structures::ListSongID;
use crate::async_rodio_sink::rodio::{decoder::DecoderError, Decoder};
use crate::async_rodio_sink::{self, AsyncRodio};
use futures::Stream;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

pub struct DecodedInMemSong(Decoder<Cursor<ArcInMemSong>>);
struct ArcInMemSong(Arc<InMemSong>);

// Derive to assist with debub printing tasks
impl std::fmt::Debug for DecodedInMemSong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DecodedInMemSong").field(&"..").finish()
    }
}

impl AsRef<[u8]> for ArcInMemSong {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().0.as_ref()
    }
}

pub struct Player {
    rodio_handle: AsyncRodio<Decoder<Cursor<ArcInMemSong>>, ListSongID>,
}

// Consider if this can be managed by Server.
impl Player {
    pub fn new() -> Self {
        let rodio_handle = AsyncRodio::new();
        Self { rodio_handle }
    }
    pub fn autoplay_song(
        &self,
        song: DecodedInMemSong,
        song_id: ListSongID,
    ) -> impl Stream<Item = async_rodio_sink::AutoplayUpdate<ListSongID>> {
        self.rodio_handle.autoplay_song(song.0, song_id)
    }
    pub fn play_song(
        &self,
        song: DecodedInMemSong,
        song_id: ListSongID,
    ) -> impl Stream<Item = async_rodio_sink::PlayUpdate<ListSongID>> {
        self.rodio_handle.play_song(song.0, song_id)
    }
    pub fn queue_song(
        &self,
        song: DecodedInMemSong,
        song_id: ListSongID,
    ) -> impl Stream<Item = async_rodio_sink::QueueUpdate<ListSongID>> {
        self.rodio_handle.queue_song(song.0, song_id)
    }
    pub async fn seek(
        &self,
        duration: Duration,
        direction: async_rodio_sink::SeekDirection,
    ) -> Option<async_rodio_sink::ProgressUpdate<ListSongID>> {
        self.rodio_handle.seek(duration, direction).await
    }
    pub async fn stop(&self, song_id: ListSongID) -> Option<async_rodio_sink::Stopped<ListSongID>> {
        self.rodio_handle.stop(song_id).await
    }
    pub async fn pause_play(
        &self,
        song_id: ListSongID,
    ) -> Option<async_rodio_sink::PausePlayResponse<ListSongID>> {
        self.rodio_handle.pause_play(song_id).await
    }
    pub async fn increase_volume(&self, vol_inc: i8) -> Option<async_rodio_sink::VolumeUpdate> {
        self.rodio_handle.increase_volume(vol_inc).await
    }
    pub async fn try_decode(
        song: Arc<InMemSong>,
    ) -> std::result::Result<DecodedInMemSong, DecoderError> {
        tokio::task::spawn_blocking(move || try_decode(song))
            .await
            .expect("Try decode should not panic")
    }
}

/// Try to decode bytes into Source.
fn try_decode(song: Arc<InMemSong>) -> std::result::Result<DecodedInMemSong, DecoderError> {
    let song = ArcInMemSong(song);
    let cur = std::io::Cursor::new(song);
    Ok(DecodedInMemSong(async_rodio_sink::rodio::Decoder::new(
        cur,
    )?))
}
