use crate::youtube_downloader::{YoutubeMusicDownload, YoutubeMusicDownloader};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use std::ffi::OsString;
use std::ops::Deref;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

#[derive(Clone)]
/// # Note
/// Cheap to clone due to use of Arc to store internals.
pub struct YtDlpDownloader {
    yt_dlp_command: Arc<OsString>,
}

#[derive(Debug)]
pub enum YtDlpDownloaderError {
    ErrorSpawningYtDlp { message: String },
    ErrorRunningYtDlp,
    InvalidYtDlpOutput { output: String },
}

impl std::fmt::Display for YtDlpDownloaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YtDlpDownloaderError::ErrorSpawningYtDlp { message } => {
                write!(f, "Error running yt-dlp - <{message}>")
            }
            YtDlpDownloaderError::ErrorRunningYtDlp => {
                write!(f, "Error running yt-dlp - no or invalid output")
            }
            YtDlpDownloaderError::InvalidYtDlpOutput { output } => write!(
                f,
                "Error running yt-dlp - received <{output}> instead of filesize on stderr"
            ),
        }
    }
}

impl YtDlpDownloader {
    pub fn new(yt_dlp_command: String) -> Self {
        Self {
            yt_dlp_command: Arc::new(yt_dlp_command.into()),
        }
    }
    pub async fn get_version(self) -> Result<String, YtDlpDownloaderError> {
        let output = tokio::process::Command::new(self.yt_dlp_command.deref())
            .arg("--version")
            .output()
            .await
            .map_err(|e| YtDlpDownloaderError::ErrorSpawningYtDlp {
                message: format!("{e}"),
            })?;
        String::from_utf8(output.stdout).map_err(|e| YtDlpDownloaderError::InvalidYtDlpOutput {
            output: e.to_string(),
        })
    }
}

impl YoutubeMusicDownloader for YtDlpDownloader {
    type Error = YtDlpDownloaderError;

    async fn stream_song(
        &self,
        song_video_id: impl AsRef<str> + Send,
    ) -> Result<
        YoutubeMusicDownload<impl Stream<Item = Result<Bytes, Self::Error>> + Send>,
        Self::Error,
    > {
        let command = self.yt_dlp_command.clone();
        async move {
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
                song_video_id.as_ref(),
            ];
            let proc = tokio::process::Command::new(command.deref())
                .args(args)
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| YtDlpDownloaderError::ErrorSpawningYtDlp {
                    message: format!("{e}"),
                })?;
            // Consider no stdout and/or no stdout to be an error.
            // In future, could consider reading stderr if no stdout as it may contain an
            // error message.
            let Child {
                stderr: Some(stderr),
                stdout: Some(stdout),
                ..
            } = proc
            else {
                return Err(YtDlpDownloaderError::ErrorRunningYtDlp);
            };
            let stderr = BufReader::new(stderr)
                .lines()
                .next_line()
                .await
                .ok()
                .flatten()
                .ok_or(YtDlpDownloaderError::ErrorRunningYtDlp)?;
            let total_size_bytes = str::parse(&stderr)
                .map_err(|_| YtDlpDownloaderError::InvalidYtDlpOutput { output: stderr })?;
            let stream = tokio_util::io::ReaderStream::new(stdout)
                .map_err(|_| YtDlpDownloaderError::ErrorRunningYtDlp);
            Ok(YoutubeMusicDownload {
                total_size_bytes,
                song: stream,
            })
        }
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::youtube_downloader::yt_dlp::YtDlpDownloader;
    use crate::youtube_downloader::{YoutubeMusicDownload, YoutubeMusicDownloader};
    use bytes::Bytes;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore = "Network and yt-dlp required"]
    async fn test_downloading_a_song_with_ytdlp() {
        let downloader = YtDlpDownloader::new("yt-dlp".to_string());
        let YoutubeMusicDownload { song: stream, .. } =
            downloader.stream_song("lYBUbBu4W08").await.unwrap();
        stream
            .map(|item| item.unwrap())
            .collect::<Vec<Bytes>>()
            .await;
    }
}
