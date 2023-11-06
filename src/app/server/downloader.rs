use crate::{
    app::{
        structures::{ListSongID, Percentage},
        taskmanager::TaskID,
    },
    core::send_or_error,
};
use rusty_ytdl::{DownloadOptions, Video, VideoOptions};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};
use ytmapi_rs::{common::YoutubeID, VideoID};

use super::{spawn_run_or_kill, KillRequest, KillableTask, DL_CALLBACK_CHUNK_SIZE};
pub enum Request {
    DownloadSong(VideoID<'static>, ListSongID, KillableTask),
}
pub enum Response {
    SongProgressUpdate(SongProgressUpdateType, ListSongID, TaskID),
}

#[derive(Debug)]
pub enum SongProgressUpdateType {
    Started,
    Downloading(Percentage),
    Completed(Vec<u8>),
    Error,
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
                ..Default::default()
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
                    super::Response::Downloader(Response::SongProgressUpdate(
                        SongProgressUpdateType::Started,
                        playlist_id,
                        id,
                    )),
                )
                .await;
                let Ok(video) = Video::new_with_options(song_video_id.get_raw(), options) else {
                    error!("Error received finding song");
                    send_or_error(
                        &tx,
                        super::Response::Downloader(Response::SongProgressUpdate(
                            SongProgressUpdateType::Error,
                            playlist_id,
                            id,
                        )),
                    )
                    .await;
                    return;
                };
                let stream = match video.stream().await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Error <{e}> received converting song to stream");
                        send_or_error(
                            &tx,
                            super::Response::Downloader(Response::SongProgressUpdate(
                                SongProgressUpdateType::Error,
                                playlist_id,
                                id,
                            )),
                        )
                        .await;
                        return;
                    }
                };
                let mut i = 0;
                let mut songbuffer = Vec::new();
                loop {
                    match stream.chunk().await {
                        Ok(Some(mut chunk)) => {
                            i += 1;
                            songbuffer.append(&mut chunk);
                            let progress =
                                (i * DL_CALLBACK_CHUNK_SIZE) * 100 / stream.content_length() as u64;
                            info!("Sending song progress update");
                            send_or_error(
                                &tx,
                                super::Response::Downloader(Response::SongProgressUpdate(
                                    SongProgressUpdateType::Downloading(Percentage(progress as u8)),
                                    playlist_id,
                                    id,
                                )),
                            )
                            .await;
                        }
                        Err(e) => {
                            error!("Error <{e}> received downloading song");
                            send_or_error(
                                &tx,
                                super::Response::Downloader(Response::SongProgressUpdate(
                                    SongProgressUpdateType::Error,
                                    playlist_id,
                                    id,
                                )),
                            )
                            .await;
                            return;
                        }
                        Ok(None) => break,
                    }
                }
                info!("Song downloaded");
                send_or_error(
                    &tx,
                    super::Response::Downloader(Response::SongProgressUpdate(
                        SongProgressUpdateType::Completed(songbuffer),
                        playlist_id,
                        id,
                    )),
                )
                .await;
            },
            kill,
        )
        .await;
    }
}
