use super::{spawn_run_or_kill, KillableTask, DL_CALLBACK_CHUNK_SIZE};
use crate::{
    app::{
        structures::{ListSongID, Percentage},
        taskmanager::TaskID,
    },
    core::send_or_error,
};
use rusty_ytdl::{DownloadOptions, RequestOptions, Video, VideoOptions};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use ytmapi_rs::common::{VideoID, YoutubeID};

const MAX_RETRIES: usize = 5;

pub enum Request {
    DownloadSong(VideoID<'static>, ListSongID, KillableTask),
}
#[derive(Debug)]
pub enum Response {
    DownloadProgressUpdate(DownloadProgressUpdateType, ListSongID, TaskID),
}

#[derive(Debug)]
pub enum DownloadProgressUpdateType {
    Started,
    Downloading(Percentage),
    Completed(Vec<u8>),
    Error,
    Retrying { times_retried: usize },
}
pub struct Downloader {
    options: VideoOptions,
    response_tx: mpsc::Sender<super::Response>,
}
impl Downloader {
    pub fn new(response_tx: mpsc::Sender<super::Response>) -> Self {
        Self {
            options: VideoOptions {
                quality: rusty_ytdl::VideoQuality::LowestAudio,
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
            },
            response_tx,
        }
    }
    pub async fn handle_request(&self, request: Request) {
        match request {
            Request::DownloadSong(s_id, p_id, task) => {
                self.handle_download_song(s_id, p_id, task).await
            }
        }
    }
    async fn handle_download_song(
        &self,
        song_video_id: VideoID<'static>,
        playlist_id: ListSongID,
        task: KillableTask,
    ) {
        let KillableTask { id, kill_rx } = task;
        let tx = self.response_tx.clone();
        // TODO: Find way to avoid clone of options here.
        let options = self.options.clone();
        let _ = spawn_run_or_kill(
            async move {
                tracing::info!("Running download");
                send_or_error(
                    &tx,
                    super::Response::Downloader(Response::DownloadProgressUpdate(
                        DownloadProgressUpdateType::Started,
                        playlist_id,
                        id,
                    )),
                )
                .await;
                let Ok(video) = Video::new_with_options(song_video_id.get_raw(), options) else {
                    error!("Error received finding song");
                    send_or_error(
                        &tx,
                        super::Response::Downloader(Response::DownloadProgressUpdate(
                            DownloadProgressUpdateType::Error,
                            playlist_id,
                            id,
                        )),
                    )
                    .await;
                    return;
                };
                let mut retries = 0;
                let mut download_succeeded = false;
                let mut songbuffer = Vec::new();
                while retries <= 5 && !download_succeeded {
                    // NOTE: This can ony fail if rusty_ytdl fails to build a reqwest::Client.
                    let stream = match video.stream().await {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Error <{e}> received converting song to stream");
                            send_or_error(
                                &tx,
                                super::Response::Downloader(Response::DownloadProgressUpdate(
                                    DownloadProgressUpdateType::Error,
                                    playlist_id,
                                    id,
                                )),
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
                                let progress = (i * DL_CALLBACK_CHUNK_SIZE) * 100
                                    / stream.content_length() as u64;
                                info!("Sending song progress update");
                                send_or_error(
                                    &tx,
                                    super::Response::Downloader(Response::DownloadProgressUpdate(
                                        DownloadProgressUpdateType::Downloading(Percentage(
                                            progress as u8,
                                        )),
                                        playlist_id,
                                        id,
                                    )),
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
                                        super::Response::Downloader(
                                            Response::DownloadProgressUpdate(
                                                DownloadProgressUpdateType::Error,
                                                playlist_id,
                                                id,
                                            ),
                                        ),
                                    )
                                    .await;
                                    return;
                                }
                                warn!("Retrying - {} tries left", MAX_RETRIES - retries);
                                send_or_error(
                                    &tx,
                                    super::Response::Downloader(Response::DownloadProgressUpdate(
                                        DownloadProgressUpdateType::Retrying {
                                            times_retried: retries,
                                        },
                                        playlist_id,
                                        id,
                                    )),
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
                    super::Response::Downloader(Response::DownloadProgressUpdate(
                        DownloadProgressUpdateType::Completed(songbuffer),
                        playlist_id,
                        id,
                    )),
                )
                .await;
            },
            kill_rx,
        )
        .await;
    }
}
