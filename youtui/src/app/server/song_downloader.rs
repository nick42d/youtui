use super::{AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::app::server::MAX_RETRIES;
use crate::app::structures::{ListSongID, Percentage};
use crate::app::CALLBACK_CHANNEL_SIZE;
use crate::core::send_or_error;
use crate::get_data_dir;
use crate::youtube_downloader::native::NativeYoutubeDownloader;
use crate::youtube_downloader::yt_dlp::YtDlpDownloader;
use crate::youtube_downloader::YoutubeDownloader;
use futures::{Stream, StreamExt, TryStreamExt};
use rusty_ytdl::{reqwest, DownloadOptions, RequestOptions, Video, VideoOptions};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
// use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use ytmapi_rs::common::{VideoID, YoutubeID};

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

pub struct SongDownloader {
    /// Shared by tasks.
    backend: crate::youtube_downloader::yt_dlp::YtDlpDownloader,
    // backend: crate::youtube_downloader::yt_dlp::FileLoader,
}
impl SongDownloader {
    pub fn new(po_token: Option<String>, client: reqwest::Client) -> Self {
        // let backend =
        //     NativeYoutubeDownloader::new(DL_CALLBACK_CHUNK_SIZE, AUDIO_QUALITY,
        // po_token, client);
        let dir = get_data_dir().unwrap().join("temp_download_dir");
        fs_err::create_dir_all(&dir);
        let backend = YtDlpDownloader::new(dir);
        // let backend = crate::youtube_downloader::yt_dlp::FileLoader {
        //     path_to_file: dir.join("CAtFrU978Xc.webm"),
        // };
        Self { backend }
    }
    pub fn download_song(
        &self,
        song_video_id: VideoID<'static>,
        song_playlist_id: ListSongID,
    ) -> impl Stream<Item = DownloadProgressUpdate> {
        download_song(self.backend.clone(), song_video_id, song_playlist_id)
    }
}

fn download_song<T: YoutubeDownloader + Send + 'static>(
    downloader: T,
    song_video_id: VideoID<'static>,
    song_playlist_id: ListSongID,
) -> impl Stream<Item = DownloadProgressUpdate>
where
    T::Error: std::fmt::Display,
    T::Error: Send,
{
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
        let (song_information, stream) =
            match downloader.download_song(song_video_id.get_raw()).await {
                Err(e) => {
                    error!("Error received finding song: <{e}>");
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
                Ok(x) => x,
            };
        let mut retries = 0;
        // TODO: Re-add loop - but note that each iteration requires access to a fresh
        // stream.
        //
        // while retries <= MAX_RETRIES {
        let song = stream
            .scan(0, |bytes_streamed, chunk| async move { Some(chunk) })
            .then(|(idx, chunk)| {
                let tx = tx.clone();
                async move {
                    tracing::warn!("Currently reporting incorrect progress percentage");
                    let progress = (idx * chunk.as_deref().unwrap_or_default().len() as usize)
                        * 100
                        / song_information.total_size_bytes;
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
                Ok(chunk) => {
                    futures::future::Either::Left(futures::stream::iter(chunk.into_iter().map(Ok)))
                }
                Err(e) => futures::future::Either::Right(futures::stream::once(async { Err(e) })),
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
                // break;
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
        // }
    });
    ReceiverStream::new(rx)
}
