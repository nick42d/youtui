use super::{AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::{
    app::{
        server::MAX_RETRIES,
        structures::{ListSongID, Percentage},
        CALLBACK_CHANNEL_SIZE,
    },
    core::send_or_error,
};
use futures::{Stream, StreamExt, TryStreamExt};
use rusty_ytdl::{DownloadOptions, RequestOptions, Video, VideoOptions};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
// use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use utils::into_futures_stream;
use ytmapi_rs::common::{VideoID, YoutubeID};

mod utils;

#[derive(Debug)]
pub struct DownloadProgressUpdate {
    pub kind: DownloadProgressUpdateType,
    pub id: ListSongID,
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
#[derive(PartialEq)]
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
    let (tx, rx) = tokio::sync::mpsc::channel(CALLBACK_CHANNEL_SIZE);
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
        while retries <= MAX_RETRIES {
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
            let content_length = stream.content_length();
            let stream = into_futures_stream(stream);
            let song = futures::StreamExt::enumerate(stream)
                .then(|(idx, chunk)| {
                    let tx = tx.clone();
                    async move {
                        let progress =
                            (idx * DL_CALLBACK_CHUNK_SIZE as usize) * 100 / content_length;
                        info!("Sending song progress update");
                        send_or_error(
                            tx,
                            DownloadProgressUpdate {
                                kind: DownloadProgressUpdateType::Downloading(Percentage(
                                    progress as u8,
                                )),
                                id: song_playlist_id,
                            },
                        )
                        .await;
                        chunk
                    }
                })
                .flat_map(|chunk| match chunk {
                    Ok(chunk) => futures::future::Either::Left(futures::stream::iter(
                        chunk.into_iter().map(Ok),
                    )),
                    Err(e) => {
                        futures::future::Either::Right(futures::stream::once(async { Err(e) }))
                    }
                })
                .try_collect::<Vec<u8>>()
                .await;
            match song {
                Ok(song) => {
                    info!("Song downloaded");
                    send_or_error(
                        &tx,
                        DownloadProgressUpdate {
                            kind: DownloadProgressUpdateType::Completed(InMemSong(song)),
                            id: song_playlist_id,
                        },
                    )
                    .await;
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
                }
            }
        }
    });
    ReceiverStream::new(rx)
}
