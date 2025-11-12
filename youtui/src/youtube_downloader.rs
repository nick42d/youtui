use bytes::Bytes;
use futures::Stream;

mod native;
mod yt_dlp;

pub struct SongInformation {
    total_size_bytes: usize,
    chunk_size_bytes: u64,
}

pub trait YoutubeDownloader {
    type Error;
    async fn download_song<'a>(
        &'a self,
        song_video_id: impl Into<String>,
    ) -> Result<
        (
            SongInformation,
            impl Stream<Item = Result<Bytes, Self::Error>> + 'a,
        ),
        Self::Error,
    >;
}
