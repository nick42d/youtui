use crate::youtube_downloader::{SongInformation, YoutubeDownloader};
use bytes::Bytes;
use futures::Stream;
use rusty_ytdl::{Video, VideoError, VideoOptions};

pub struct NativeYoutubeDownloader {
    options: VideoOptions,
}

impl YoutubeDownloader for NativeYoutubeDownloader {
    type Error = ();
    async fn download_song<'a>(
        &'a self,
        song_video_id: impl Into<String>,
    ) -> Result<
        (
            SongInformation,
            impl Stream<Item = Result<Bytes, Self::Error>> + 'a,
        ),
        Self::Error,
    > {
        let Ok(video) = Video::new_with_options(song_video_id, &self.options) else {
            todo!();
        };
        let stream = video.stream().await.unwrap();
        let chunk_size_bytes = self.options.download_options.dl_chunk_size.unwrap();
        let total_size_bytes = stream.content_length();
        let stream = into_futures_stream_DUPLICATE(stream).map(|item| item.map_err(|_| ()));
        let song_information = SongInformation {
            total_size_bytes,
            chunk_size_bytes,
        };
        Ok((song_information, stream))
    }
}

/// Helper function to use rusty_ytdl::stream::Stream is if it were a
/// futures::Stream.
// NOTE: Potentially could be upstreamed: https://github.com/Mithronn/rusty_ytdl/issues/34.
pub fn into_futures_stream_DUPLICATE(
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
