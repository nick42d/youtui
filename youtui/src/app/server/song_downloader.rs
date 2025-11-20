use super::{AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::app::server::MAX_RETRIES;
use crate::app::structures::{ListSongID, Percentage};
use crate::app::CALLBACK_CHANNEL_SIZE;
use crate::config::{Config, DownloaderType};
use crate::core::send_or_error;
use crate::youtube_downloader::native::NativeYoutubeDownloader;
use crate::youtube_downloader::yt_dlp::YtDlpDownloader;
use crate::youtube_downloader::{YoutubeMusicDownload, YoutubeMusicDownloader};
use futures::{Stream, StreamExt, TryStreamExt};
use rusty_ytdl::reqwest;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info, warn};
use ytmapi_rs::common::{VideoID, YoutubeID};

// Minimum tick in song progress that is reported to UI - prevents frequent UI
// updates.
const MIN_SONG_PROGRESS_INTERVAL: usize = 3;

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

pub enum SongDownloader {
    YtDlp(YtDlpDownloader),
    Native(NativeYoutubeDownloader),
}

impl SongDownloader {
    pub fn new(po_token: Option<String>, client: reqwest::Client, config: &Config) -> Self {
        match config.downloader_type {
            DownloaderType::Native => SongDownloader::Native(NativeYoutubeDownloader::new(
                DL_CALLBACK_CHUNK_SIZE,
                AUDIO_QUALITY,
                po_token,
                client,
            )),
            DownloaderType::YtDlp => {
                SongDownloader::YtDlp(YtDlpDownloader::new(config.yt_dlp_command.clone()))
            }
        }
    }
    pub fn download_song(
        &self,
        song_video_id: VideoID<'static>,
        song_playlist_id: ListSongID,
    ) -> impl Stream<Item = DownloadProgressUpdate> {
        match self {
            SongDownloader::YtDlp(yt_dlp_downloader) => futures::future::Either::Left(
                download_song(yt_dlp_downloader.clone(), song_video_id, song_playlist_id),
            ),
            SongDownloader::Native(native_youtube_downloader) => {
                futures::future::Either::Right(download_song(
                    native_youtube_downloader.clone(),
                    song_video_id,
                    song_playlist_id,
                ))
            }
        }
    }
}

fn download_song<T: YoutubeMusicDownloader + Send + 'static>(
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
        let mut retries = 0;
        while retries <= MAX_RETRIES {
            let YoutubeMusicDownload {
                total_size_bytes,
                song: stream,
            } = match downloader.stream_song(song_video_id.get_raw()).await {
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
            let song = stream
                .scan((0, 0), |(bytes_streamed, last_progress_reported), chunk| {
                    let tx = tx.clone();
                    let chunk_bytes = match &chunk {
                        Ok(chunk) => chunk.len(),
                        Err(_) => 0,
                    };
                    *bytes_streamed += chunk_bytes;
                    let bytes_streamed_clone = *bytes_streamed;
                    let progress = bytes_streamed_clone * 100 / total_size_bytes;
                    let report_progress =
                        progress >= *last_progress_reported + MIN_SONG_PROGRESS_INTERVAL;
                    if report_progress {
                        *last_progress_reported = progress;
                    }
                    async move {
                        if report_progress {
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
                        }
                        Some(chunk)
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
