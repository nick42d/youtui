use super::{AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::{
    app::{
        server::MAX_RETRIES,
        structures::{ListSongID, Percentage},
    },
    core::send_or_error,
};
use futures::Stream;
use rusty_ytdl::{DownloadOptions, RequestOptions, Video, VideoOptions};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info, warn};
use ytmapi_rs::common::{VideoID, YoutubeID};

#[derive(Debug)]
pub struct DownloadProgressUpdate {
    kind: DownloadProgressUpdateType,
    id: ListSongID,
}

#[derive(Debug)]
pub enum DownloadProgressUpdateType {
    Started,
    Downloading(Percentage),
    Completed(InMemSong),
    Error,
    Retrying { times_retried: usize },
}

/// Representation of a song in memory - an array of bytes.
/// Newtype pattern is used to provide a cleaner Debug display.
pub struct InMemSong(pub Vec<u8>);
// Custom derive - otherwise will be displaying 3MB array of bytes...
impl std::fmt::Debug for InMemSong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InMemSong").field(&"Vec<..>").finish()
    }
}

pub struct Downloader {
    /// Shared by tasks.
    options: Arc<VideoOptions>,
}
impl Downloader {
    pub fn new(po_token: Option<String>) -> Self {
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
                po_token,
                ..Default::default()
            },
        });
        Self { options }
    }
    pub fn download_song(
        &self,
        song_video_id: VideoID<'static>,
        song_playlist_id: ListSongID,
    ) -> impl Stream<Item = DownloadProgressUpdate> {
        download_song(self.options.clone(), song_video_id, song_playlist_id)
    }
}

fn download_song(
    options: Arc<VideoOptions>,
    song_video_id: VideoID<'static>,
    song_playlist_id: ListSongID,
) -> impl Stream<Item = DownloadProgressUpdate> {
    // TODO: CHANNEL SIZE
    let (tx, rx) = tokio::sync::mpsc::channel(50);
    tokio::spawn(async move {
        tracing::info!("Running download");
        send_or_error(
            &tx,
            DownloadProgressUpdate {
                kind: DownloadProgressUpdateType::Started,
                id: song_playlist_id,
            },
        )
        .await;
        let Ok(video) = Video::new_with_options(song_video_id.get_raw(), options.as_ref()) else {
            error!("Error received finding song");
            send_or_error(
                &tx,
                DownloadProgressUpdate {
                    kind: DownloadProgressUpdateType::Error,
                    id: song_playlist_id,
                },
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
                        DownloadProgressUpdate {
                            kind: DownloadProgressUpdateType::Error,
                            id: song_playlist_id,
                        },
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
                            DownloadProgressUpdate {
                                kind: DownloadProgressUpdateType::Downloading(Percentage(
                                    progress as u8,
                                )),
                                id: song_playlist_id,
                            },
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
                                DownloadProgressUpdate {
                                    kind: DownloadProgressUpdateType::Error,
                                    id: song_playlist_id,
                                },
                            )
                            .await;
                            return;
                        }
                        warn!("Retrying - {} tries left", MAX_RETRIES - retries);
                        send_or_error(
                            &tx,
                            DownloadProgressUpdate {
                                kind: DownloadProgressUpdateType::Retrying {
                                    times_retried: retries,
                                },
                                id: song_playlist_id,
                            },
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
            DownloadProgressUpdate {
                kind: DownloadProgressUpdateType::Completed(InMemSong(songbuffer)),
                id: song_playlist_id,
            },
        )
        .await;
    });
    ReceiverStream::new(rx)
}
