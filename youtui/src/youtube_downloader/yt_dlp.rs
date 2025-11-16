use crate::youtube_downloader::{SongInformation, YoutubeDownloader};
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use std::ffi::OsString;
use std::future::Future;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Child;

#[derive(Clone)]
pub struct YtDlpDownloader {
    yt_dlp_command: Arc<OsString>,
}

#[derive(Debug)]
pub struct YtDlpDownloaderError;

impl std::fmt::Display for YtDlpDownloaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl YtDlpDownloader {
    pub fn new(yt_dlp_command: String) -> Self {
        Self {
            yt_dlp_command: Arc::new(yt_dlp_command.into()),
        }
    }
}

impl YoutubeDownloader for YtDlpDownloader {
    type Error = YtDlpDownloaderError;

    async fn download_song(
        &self,
        song_video_id: impl Into<String> + Send,
    ) -> Result<
        (
            SongInformation,
            impl Stream<Item = Result<Bytes, Self::Error>> + Send + 'static,
        ),
        Self::Error,
    > {
        let song_video_id: String = song_video_id.into();
        let command = self.yt_dlp_command.clone();
        async move {
            let song_url = format!("https://www.youtube.com/watch?v={song_video_id}");
            let args = vec![
                // First, print filesize in bytes to stderr
                "--print",
                "filesize",
                // Force download the song even though print mode is used
                "--no-simulate",
                "-q",
                "--no-warnings",
                // Best Audio, m4a (otherwise downloads unsupported webm format)
                "-f",
                "ba[ext=m4a]",
                // Output song bytes to stdout
                "-o",
                "-",
            ];
            tracing::info!("runnning cmd");
            let proc = tokio::process::Command::new(command.deref())
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
            let stream =
                tokio_util::io::ReaderStream::new(stdout).map_err(|_| YtDlpDownloaderError);
            Ok((SongInformation { total_size_bytes }, stream))
        }
        .await
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
        let downloader = YtDlpDownloader::new("yt-dlp".to_string());
        let (_, stream) = downloader.download_song("lYBUbBu4W08").await.unwrap();
        let song = stream.map(|item| item.unwrap()).collect::<Vec<Bytes>>();
    }
}
