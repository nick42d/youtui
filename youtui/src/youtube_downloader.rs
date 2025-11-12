use bytes::Bytes;
use futures::{Stream, StreamExt};
use rusty_ytdl::{Video, VideoError, VideoOptions};

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

pub struct YtDlpDownloader {
    song_storage_dir: std::path::PathBuf,
}

impl YtDlpDownloader {
    fn new(song_storage_dir: impl Into<std::path::PathBuf>) -> Self {
        let song_storage_dir = song_storage_dir.into();
        Self { song_storage_dir }
    }
}

impl YoutubeDownloader for YtDlpDownloader {
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
        let song_video_id: String = song_video_id.into();
        const DOCKER: &str = "docker";
        let current_path = std::env::current_dir().unwrap();
        let docker_volume_mount = format!("{}:/app", self.song_storage_dir.to_string_lossy());
        let song_url = format!("https://www.youtube.com/watch?v={song_video_id}");
        let args = vec![
            "run",
            "--rm",
            "-v",
            &docker_volume_mount,
            "-w",
            "/app",
            "thr3a/yt-dlp",
            "-q",
            "--no-warnings",
            "-f",
            "ba",
            "-o",
            "%(id)s.%(ext)s",
            &song_video_id,
            "--exec",
            r#""echo %(id)s.%(ext)s""#,
        ];
        eprintln!("runnning cmd");
        let output = tokio::process::Command::new(DOCKER)
            .args(args)
            .arg(song_url)
            .output()
            .await
            .unwrap();
        let output_str = String::from_utf8(output.stdout).unwrap();
        eprintln!("output: {output_str}");
        let output = String::from_utf8(
            tokio::process::Command::new("ls")
                .arg(&self.song_storage_dir)
                .output()
                .await
                .unwrap()
                .stdout,
        )
        .unwrap();
        eprintln!("{output}");
        let filepath = self.song_storage_dir.join("your.song.webm");
        eprintln!("Looking for filename {}", filepath.to_string_lossy());
        let file_u8 = tokio::fs::read(filepath).await.unwrap();
        let file_bytes = Bytes::from(file_u8);
        Ok((
            SongInformation {
                total_size_bytes: 0,
                chunk_size_bytes: 0,
            },
            futures::stream::once(async { Ok(file_bytes) }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::youtube_downloader::{YoutubeDownloader, YtDlpDownloader};
    use bytes::Bytes;
    use futures::StreamExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_downloading_a_song() {
        let tempdir = tempdir().unwrap();
        let downloader = YtDlpDownloader::new(tempdir.path());
        let (_, stream) = downloader.download_song("lYBUbBu4W08").await.unwrap();
        let song = stream.map(|item| item.unwrap()).collect::<Vec<Bytes>>();
    }
}
