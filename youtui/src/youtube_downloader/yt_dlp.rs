use crate::youtube_downloader::{SongInformation, YoutubeDownloader};
use bytes::Bytes;
use futures::Stream;

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
