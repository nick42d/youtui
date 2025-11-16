use bytes::Bytes;
use futures::Stream;
use std::future::Future;

pub mod native;
pub mod yt_dlp;

pub struct SongInformation {
    pub total_size_bytes: usize,
}

pub trait YoutubeDownloader {
    type Error;
    fn download_song(
        &self,
        song_video_id: impl Into<String> + Send,
    ) -> impl Future<
        Output = Result<
            (
                SongInformation,
                impl Stream<Item = Result<Bytes, Self::Error>> + Send + 'static,
            ),
            Self::Error,
        >,
    > + Send;
}
