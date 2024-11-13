use super::downloader::InMemSong;
use crate::app::structures::ListSongID;
use async_rodio_sink::AsyncRodio;
use futures::Stream;
use std::sync::Arc;
use std::time::Duration;

const PLAYER_MSG_QUEUE_SIZE: usize = 256;

struct ArcInMemSong(Arc<InMemSong>);

impl AsRef<[u8]> for ArcInMemSong {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().0.as_ref()
    }
}

pub struct Player {
    rodio_handle: AsyncRodio<ArcInMemSong, ListSongID>,
}

// Consider if this can be managed by Server.
impl Player {
    pub fn new() -> Self {
        let rodio_handle = AsyncRodio::new(PLAYER_MSG_QUEUE_SIZE);
        Self { rodio_handle }
    }
    pub fn autoplay_song(
        &self,
        song: Arc<InMemSong>,
        song_id: ListSongID,
    ) -> std::result::Result<impl Stream<Item = async_rodio_sink::AutoplayUpdate<ListSongID>>, ()>
    {
        let song = ArcInMemSong(song);
        self.rodio_handle.autoplay_song(song, song_id)
    }
    pub fn play_song(
        &self,
        song: Arc<InMemSong>,
        song_id: ListSongID,
    ) -> std::result::Result<impl Stream<Item = async_rodio_sink::PlayUpdate<ListSongID>>, ()> {
        let song = ArcInMemSong(song);
        self.rodio_handle.play_song(song, song_id)
    }
    pub fn queue_song(
        &self,
        song: Arc<InMemSong>,
        song_id: ListSongID,
    ) -> std::result::Result<impl Stream<Item = async_rodio_sink::QueueUpdate<ListSongID>>, ()>
    {
        let song = ArcInMemSong(song);
        self.rodio_handle.queue_song(song, song_id)
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
}
