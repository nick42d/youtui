use crate::youtube_downloader::{SongInformation, YoutubeDownloader};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Child;

#[derive(Clone)]
pub struct YtDlpDownloader {}

#[derive(Debug)]
pub struct YtDlpDownloaderError;

impl std::fmt::Display for YtDlpDownloaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl YtDlpDownloader {
    pub fn new() -> Self {
        Self {}
    }
}

impl YoutubeDownloader for YtDlpDownloader {
    type Error = YtDlpDownloaderError;

    fn download_song(
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
        let song_video_id: String = song_video_id.into();
        async move {
            const DOCKER: &str = "docker";
            let song_url = format!("https://www.youtube.com/watch?v={song_video_id}");
            let args = vec![
                "run",
                "thr3a/yt-dlp",
                "--print",
                "filesize",
                // Force download the song even though print mode is used
                "--no-simulate",
                "-q",
                "--no-warnings",
                "-f",
                "ba[ext=m4a]",
                "-o",
                "-",
                // "%(id)s.%(ext)s",
                // &song_video_id,
                // "--exec",
                // "echo",
            ];
            tracing::info!("runnning cmd");
            let proc = tokio::process::Command::new(DOCKER)
                .args(args)
                .arg(song_url)
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let Child { stderr, stdout, .. } = proc;
            let stdout = stdout.unwrap();
            let stderr = BufReader::new(stderr.unwrap())
                .lines()
                .next_line()
                .await
                .unwrap()
                .unwrap();
            let total_size_bytes = str::parse(&stderr).unwrap();
            tracing::info!("Song total size bytes {total_size_bytes}");
            let stream = tokio_util::io::ReaderStream::new(stdout).map(|maybe_bytes| {
                maybe_bytes
                    .map(|bytes| bytes::Bytes::from(bytes))
                    .map_err(|_| YtDlpDownloaderError)
            });
            Ok((SongInformation { total_size_bytes }, stream))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::youtube_downloader::yt_dlp::YtDlpDownloader;
    use crate::youtube_downloader::YoutubeDownloader;
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
