use super::{messages::ServerResponse, KillableTask, AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::{
    app::{
        server::MAX_RETRIES,
        structures::{ListSongID, Percentage},
        taskmanager::TaskID,
    },
    core::send_or_error,
};
use rusty_ytdl::{DownloadOptions, RequestOptions, Video, VideoOptions};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use ytmapi_rs::common::{VideoID, YoutubeID};

#[derive(Debug)]
pub enum KillableServerRequest {
    DownloadSong(VideoID<'static>, ListSongID),
}
#[derive(Debug)]
pub enum UnkillableServerRequest {}

#[derive(Debug)]
pub enum Response {
    DownloadProgressUpdate(DownloadProgressUpdateType, ListSongID),
}

/// Representation of a song in memory - an array of bytes.
/// Newtype pattern is used to provide a cleaner Debug display.
pub struct InMemSong(pub Vec<u8>);

// Custom derive - otherwise will be displaying 3MB array of bytes...
impl std::fmt::Debug for InMemSong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Song").field(&"Vec<..>").finish()
    }
}

#[derive(Debug)]
pub enum DownloadProgressUpdateType {
    Started,
    Downloading(Percentage),
    Completed(InMemSong),
    Error,
    Retrying { times_retried: usize },
}

pub struct Downloader {
    /// Shared by tasks.
    options: Arc<VideoOptions>,
}
impl Downloader {
    pub fn new() -> Self {
        let options = Arc::new(VideoOptions {
            quality: AUDIO_QUALITY,
            filter: rusty_ytdl::VideoSearchOptions::Audio,
            download_options: DownloadOptions {
                dl_chunk_size: Some(DL_CALLBACK_CHUNK_SIZE),
            },
            request_options: RequestOptions {
                client: Some(
                    rusty_ytdl::reqwest::Client::builder()
                        .use_rustls_tls()
                        .build()
                        .expect("Expect client build to succeed"),
                ),
                ..Default::default()
            },
        });
        Self { options }
    }
}

async fn download_song(
    options: Arc<VideoOptions>,
    song_video_id: VideoID<'static>,
    song_playlist_id: ListSongID,
    task_id: TaskID,
    tx: mpsc::Sender<ServerResponse>,
) {
    tracing::info!("Running download");
    send_or_error(
        &tx,
        ServerResponse::new_downloader(
            task_id,
            Response::DownloadProgressUpdate(DownloadProgressUpdateType::Started, song_playlist_id),
        ),
    )
    .await;
    // Upstream issue to remove allocation
    // https://github.com/Mithronn/rusty_ytdl/issues/38
    let options = (*options).clone();
    let Ok(video) = Video::new_with_options(song_video_id.get_raw(), options) else {
        error!("Error received finding song");
        send_or_error(
            &tx,
            ServerResponse::new_downloader(
                task_id,
                Response::DownloadProgressUpdate(
                    DownloadProgressUpdateType::Error,
                    song_playlist_id,
                ),
            ),
        )
        .await;
        return;
    };
    let mut retries = 0;
    let mut download_succeeded = false;
    let mut songbuffer = Vec::new();
    while retries <= MAX_RETRIES && !download_succeeded {
        // NOTE: This can ony fail if rusty_ytdl fails to build a reqwest::Client.
        let stream = match video.stream().await {
            Ok(s) => s,
            Err(e) => {
                error!("Error <{e}> received converting song to stream");
                send_or_error(
                    &tx,
                    ServerResponse::new_downloader(
                        task_id,
                        Response::DownloadProgressUpdate(
                            DownloadProgressUpdateType::Error,
                            song_playlist_id,
                        ),
                    ),
                )
                .await;
                return;
            }
        };
        let mut i = 0;
        songbuffer.clear();
        loop {
            match stream.chunk().await {
                Ok(Some(chunk)) => {
                    i += 1;
                    songbuffer.append(&mut chunk.into());
                    let progress =
                        (i * DL_CALLBACK_CHUNK_SIZE) * 100 / stream.content_length() as u64;
                    info!("Sending song progress update");
                    send_or_error(
                        &tx,
                        ServerResponse::new_downloader(
                            task_id,
                            Response::DownloadProgressUpdate(
                                DownloadProgressUpdateType::Downloading(Percentage(progress as u8)),
                                song_playlist_id,
                            ),
                        ),
                    )
                    .await;
                }
                // SUCCESS
                Ok(None) => {
                    download_succeeded = true;
                    break;
                }
                Err(e) => {
                    warn!("Error <{e}> received downloading song");
                    retries += 1;
                    if retries > MAX_RETRIES {
                        error!("Max retries exceeded");
                        send_or_error(
                            &tx,
                            ServerResponse::new_downloader(
                                task_id,
                                Response::DownloadProgressUpdate(
                                    DownloadProgressUpdateType::Error,
                                    song_playlist_id,
                                ),
                            ),
                        )
                        .await;
                        return;
                    }
                    warn!("Retrying - {} tries left", MAX_RETRIES - retries);
                    send_or_error(
                        &tx,
                        ServerResponse::new_downloader(
                            task_id,
                            Response::DownloadProgressUpdate(
                                DownloadProgressUpdateType::Retrying {
                                    times_retried: retries,
                                },
                                song_playlist_id,
                            ),
                        ),
                    )
                    .await;
                    break;
                }
            }
        }
    }
    info!("Song downloaded");
    send_or_error(
        &tx,
        ServerResponse::new_downloader(
            task_id,
            Response::DownloadProgressUpdate(
                DownloadProgressUpdateType::Completed(InMemSong(songbuffer)),
                song_playlist_id,
            ),
        ),
    )
    .await;
}
