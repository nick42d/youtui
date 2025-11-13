use bytes::Bytes;
use futures::Stream;
use std::future::Future;

pub mod native;
pub mod yt_dlp;

pub struct SongInformation {
    pub total_size_bytes: usize,
    pub chunk_size_bytes: u64,
}

pub trait YoutubeDownloader {
    type Error;
    fn download_song<'a>(
        &'a self,
        song_video_id: impl Into<String>,
    ) -> impl Future<
        Output = Result<
            (
                SongInformation,
                impl Stream<Item = Result<Bytes, Self::Error>> + Send + 'a,
            ),
            Self::Error,
        >,
    > + Send;
}
