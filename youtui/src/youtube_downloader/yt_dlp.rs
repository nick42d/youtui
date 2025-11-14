use crate::youtube_downloader::{SongInformation, YoutubeDownloader};
use bytes::Bytes;
use futures::Stream;
use std::future::Future;
use std::path::PathBuf;

#[derive(Clone)]
pub struct YtDlpDownloader {
    song_storage_dir: std::path::PathBuf,
}

#[derive(Debug)]
pub struct YtDlpDownloaderError;

impl std::fmt::Display for YtDlpDownloaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl YtDlpDownloader {
    pub fn new(song_storage_dir: impl Into<std::path::PathBuf>) -> Self {
        let song_storage_dir = song_storage_dir.into();
        Self { song_storage_dir }
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
        let song_storage_dir = self.song_storage_dir.clone();
        let song_video_id: String = song_video_id.into();
        async move {
            const DOCKER: &str = "docker";
            let docker_volume_mount = format!("{}:/app", &song_storage_dir.to_string_lossy());
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
                "ba,m4a",
                "-o",
                "%(id)s.%(ext)s",
                &song_video_id,
                "--exec",
                "echo",
            ];
            eprintln!("runnning cmd");
            let output = tokio::process::Command::new(DOCKER)
                .args(args)
                .arg(song_url)
                .output()
                .await
                .unwrap();
            let docker_file_path = PathBuf::from(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap(),
            );
            let docker_file_name = docker_file_path.file_name().unwrap();
            eprintln!("output: {}", docker_file_name.to_string_lossy());
            let output = String::from_utf8(
                tokio::process::Command::new("ls")
                    .arg(&song_storage_dir)
                    .output()
                    .await
                    .unwrap()
                    .stdout,
            )
            .unwrap();
            eprintln!(
                "contents of {}\n{output}",
                &song_storage_dir.to_string_lossy()
            );
            let filepath = &song_storage_dir.join(docker_file_name);
            eprintln!("Looking for filename {}", filepath.to_string_lossy());
            let file_u8 = fs_err::tokio::read(filepath).await.unwrap();
            let file_bytes = Bytes::from(file_u8);
            Ok((
                SongInformation {
                    total_size_bytes: 1,
                    chunk_size_bytes: 1,
                },
                futures::stream::once(async { Ok(file_bytes) }),
            ))
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
