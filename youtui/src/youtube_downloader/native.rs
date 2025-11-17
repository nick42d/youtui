use crate::youtube_downloader::{SongInformation, YoutubeMusicDownloader};
use bytes::Bytes;
use futures::Stream;
use rusty_ytdl::{
    reqwest, DownloadOptions, RequestOptions, Video, VideoError, VideoOptions, VideoQuality,
};
use std::future::Future;
use std::sync::Arc;
use tokio_stream::StreamExt;

#[derive(Clone)]
/// # Note
/// Cheap to clone due to use of Arc to store internals.
pub struct NativeYoutubeDownloader {
    options: Arc<VideoOptions>,
    // hardcode dl_chunk_size in this struct to be non-optional
    dl_chunk_size: u64,
}

impl NativeYoutubeDownloader {
    pub fn new(
        dl_chunk_size: u64,
        quality: VideoQuality,
        po_token: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        let options = Arc::new(VideoOptions {
            quality,
            filter: rusty_ytdl::VideoSearchOptions::Audio,
            download_options: DownloadOptions {
                dl_chunk_size: Some(dl_chunk_size),
            },
            request_options: RequestOptions {
                client: Some(client),
                po_token,
                ..Default::default()
            },
        });
        Self {
            options,
            dl_chunk_size,
        }
    }
}

impl YoutubeMusicDownloader for NativeYoutubeDownloader {
    type Error = rusty_ytdl::VideoError;
    fn stream_song(
        &self,
        song_video_id: impl Into<String>,
    ) -> impl Future<
        Output = Result<
            (
                SongInformation,
                impl Stream<Item = Result<Bytes, Self::Error>> + Send + 'static,
            ),
            Self::Error,
        >,
    > + Send
           + 'static {
        let options = self.options.clone();
        let song_video_id: String = song_video_id.into();
        async move {
            let video = Video::new_with_options(song_video_id, options.as_ref())?;
            // NOTE: This can ony fail if rusty_ytdl fails to build a reqwest::Client.
            let stream = video.stream().await?;
            let total_size_bytes = stream.content_length();
            let stream = into_futures_stream(stream);
            let song_information = SongInformation { total_size_bytes };
            Ok((song_information, stream))
        }
    }
}

/// Helper function to use rusty_ytdl::stream::Stream is if it were a
/// futures::Stream.
// NOTE: Potentially could be upstreamed: https://github.com/Mithronn/rusty_ytdl/issues/34.
pub fn into_futures_stream(
    youtube_stream: Box<dyn rusty_ytdl::stream::Stream + Send>,
) -> impl futures::Stream<Item = Result<Bytes, VideoError>> + Send {
    // Second value of initialisation tuple represents if the previous iteration of
    // the stream errored. If so, stream will close, as no future iterations of
    // the stream are expected to return Ok.
    futures::stream::unfold((youtube_stream, false), |(state, err)| async move {
        if err {
            return None;
        };
        let chunk = state.chunk().await;
        match chunk {
            // Return error value on this iteration, on the next iteration return None.
            Err(e) => Some((Err(e), (state, true))),
            // Happy path
            Ok(Some(bytes)) => Some((Ok(bytes), (state, false))),
            // Stream has closed.
            Ok(None) => None,
        }
    })
}
