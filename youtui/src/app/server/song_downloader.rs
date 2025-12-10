use super::{AUDIO_QUALITY, DL_CALLBACK_CHUNK_SIZE};
use crate::app::CALLBACK_CHANNEL_SIZE;
use crate::app::server::MAX_RETRIES;
use crate::app::structures::{ListSongID, Percentage};
use crate::config::{Config, DownloaderType};
use crate::core::send_or_error;
use crate::youtube_downloader::native::NativeYoutubeDownloader;
use crate::youtube_downloader::yt_dlp::YtDlpDownloader;
use crate::youtube_downloader::{YoutubeMusicDownload, YoutubeMusicDownloader};
use futures::{Stream, StreamExt, TryStreamExt};
use rusty_ytdl::reqwest;
use std::future::Future;
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
            DownloaderType::Native => {
                info!("Initiating native downloader");
                SongDownloader::Native(NativeYoutubeDownloader::new(
                    DL_CALLBACK_CHUNK_SIZE,
                    AUDIO_QUALITY,
                    po_token,
                    client,
                ))
            }
            DownloaderType::YtDlp => {
                info!("Initiating yt-dlp downloader");
                let downloader = YtDlpDownloader::new(config.yt_dlp_command.clone());
                let downloader_clone = YtDlpDownloader::new(config.yt_dlp_command.clone());
                tokio::task::spawn(async {
                    let output = downloader_clone.get_version().await;
                    match output {
                        Ok(output) => {
                            info!("Output of 'yt-dlp --version': {:?}", output);
                        }
                        Err(e) => error!("Unable to run 'yt-dlp --version', error: <{e}>"),
                    }
                });
                SongDownloader::YtDlp(downloader)
            }
        }
    }
    pub fn download_song(
        &self,
        song_video_id: VideoID<'static>,
        song_playlist_id: ListSongID,
    ) -> impl Stream<Item = DownloadProgressUpdate> + use<> {
        match self {
            SongDownloader::YtDlp(yt_dlp_downloader) => {
                futures::future::Either::Left(download_song_using_downloader(
                    yt_dlp_downloader.clone(),
                    song_video_id,
                    song_playlist_id,
                ))
            }
            SongDownloader::Native(native_youtube_downloader) => {
                futures::future::Either::Right(download_song_using_downloader(
                    native_youtube_downloader.clone(),
                    song_video_id,
                    song_playlist_id,
                ))
            }
        }
    }
}

fn download_song_using_downloader<T>(
    downloader: T,
    song_video_id: VideoID<'static>,
    song_playlist_id: ListSongID,
) -> impl Stream<Item = DownloadProgressUpdate>
where
    T: YoutubeMusicDownloader + Send + Sync + 'static,
    T::Error: std::fmt::Display + Send,
{
    let (tx, rx) = tokio::sync::mpsc::channel(CALLBACK_CHANNEL_SIZE);
    tokio::spawn(async move {
        tracing::info!("Running download");
        send_or_error(
            &tx.clone(),
            DownloadProgressUpdate {
                kind: DownloadProgressUpdateType::Started,
                id: song_playlist_id,
            },
        )
        .await;
        let song_download = || {
            let tx = tx.clone();
            download_song_with_progress_update_callback(
                &downloader,
                song_video_id.clone(),
                MIN_SONG_PROGRESS_INTERVAL,
                move |p| {
                    let tx_clone = tx.clone();
                    info!("Sending song progress update");
                    send_or_error(
                        tx_clone,
                        DownloadProgressUpdate {
                            kind: DownloadProgressUpdateType::Downloading(p),
                            id: song_playlist_id,
                        },
                    )
                },
            )
        };
        let song = run_future_with_retries_and_retry_callback(
            song_download,
            |times_retried| {
                let tx = tx.clone();
                warn!("Retrying - {} tries left", MAX_RETRIES - times_retried);
                send_or_error(
                    tx,
                    DownloadProgressUpdate {
                        kind: DownloadProgressUpdateType::Retrying { times_retried },
                        id: song_playlist_id,
                    },
                )
            },
            MAX_RETRIES,
        )
        .await;

        match song {
            Some(song) => {
                info!("Song downloaded");
                send_or_error(
                    &tx,
                    DownloadProgressUpdate {
                        kind: DownloadProgressUpdateType::Completed(song),
                        id: song_playlist_id,
                    },
                )
                .await;
            }
            None => {
                error!("Max retries exceeded");
                send_or_error(
                    &tx,
                    DownloadProgressUpdate {
                        kind: DownloadProgressUpdateType::Error,
                        id: song_playlist_id,
                    },
                )
                .await;
            }
        };
    });
    ReceiverStream::new(rx)
}

/// Parameter for run_on_retry callback is "times retried"
async fn run_future_with_retries_and_retry_callback<Fut1, Fut2, T, E>(
    future_generator: impl Fn() -> Fut1 + Send,
    run_on_retry: impl Fn(usize) -> Fut2 + Send,
    max_retries: usize,
) -> Option<T>
where
    Fut1: Future<Output = Result<T, E>> + Send,
    Fut2: Future<Output = ()> + Send,
    E: Send,
    T: Send,
{
    let mut retries = 0;
    while retries <= max_retries {
        let output = future_generator().await;
        if let Ok(output) = output {
            return Some(output);
        }
        retries += 1;
        if retries <= max_retries {
            run_on_retry(retries).await;
        }
    }
    None
}

async fn download_song_with_progress_update_callback<T, Fut>(
    downloader: &T,
    song_video_id: VideoID<'static>,
    min_song_progress_interval: usize,
    run_on_progress_interval: impl Fn(Percentage) -> Fut + Send + Sync,
) -> Result<InMemSong, T::Error>
where
    Fut: Future<Output = ()> + Send,
    T: YoutubeMusicDownloader + Send + 'static,
    T::Error: std::fmt::Display + Send,
{
    let song_video_id = song_video_id.get_raw();
    let stream_future = downloader.stream_song(song_video_id);
    let callback = run_on_progress_interval;
    let YoutubeMusicDownload {
        total_size_bytes,
        song: stream,
    } = match stream_future.await {
        Err(e) => {
            error!("Error received finding song: <{e}>");
            return Err(e);
        }
        Ok(x) => x,
    };
    let song = stream
        .scan((0, 0), |(bytes_streamed, last_progress_reported), chunk| {
            let chunk_bytes = match &chunk {
                Ok(chunk) => chunk.len(),
                Err(_) => 0,
            };
            *bytes_streamed += chunk_bytes;
            let bytes_streamed_clone = *bytes_streamed;
            let progress = bytes_streamed_clone * 100 / total_size_bytes;
            let report_progress = progress >= *last_progress_reported + min_song_progress_interval;
            if report_progress {
                *last_progress_reported = progress;
            }
            let callback = callback(Percentage(progress as u8));
            async move {
                if report_progress {
                    callback.await;
                }
                Some(chunk)
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
        Ok(song) => Ok(InMemSong(song)),
        Err(e) => {
            error!("Error received downloading song: <{e}>");
            Err(e)
        }
    }
}
