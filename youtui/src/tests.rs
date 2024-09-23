//! Tests for key components, to allow for automated checking of 3rd party api
//! changes.
use rusty_ytdl::{Video, VideoOptions};
use std::{env, path::Path};
use tokio::sync::OnceCell;
use ytmapi_rs::{auth::BrowserToken, common::YoutubeID, YtMusic, YtMusicBuilder};

const COOKIE_PATH: &str = "../ytmapi-rs/cookie.txt";
// From Downloader
const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.

static API: OnceCell<YtMusic<BrowserToken>> = OnceCell::const_new();

async fn get_api() -> &'static YtMusic<BrowserToken> {
    API.get_or_init(|| async {
        if let Ok(cookie) = env::var("youtui_test_cookie") {
            YtMusicBuilder::new_rustls_tls()
                .with_browser_token_cookie(cookie)
                .build()
                .await
                .unwrap()
        } else {
            YtMusicBuilder::new_rustls_tls()
                .with_browser_token_cookie_file(Path::new(COOKIE_PATH))
                .build()
                .await
                .unwrap()
        }
    })
    .await
}

// This should be the same video options that the app itself uses.
fn get_video_options() -> VideoOptions {
    VideoOptions {
        quality: rusty_ytdl::VideoQuality::LowestAudio,
        filter: rusty_ytdl::VideoSearchOptions::Audio,
        download_options: rusty_ytdl::DownloadOptions {
            dl_chunk_size: Some(DL_CALLBACK_CHUNK_SIZE),
        },
        request_options: rusty_ytdl::RequestOptions {
            client: Some(
                rusty_ytdl::reqwest::Client::builder()
                    .use_rustls_tls()
                    .build()
                    .expect("Expect client build to succeed"),
            ),
            ..Default::default()
        },
    }
}

#[tokio::test]
#[ignore = "Ignored by default due to cost"]
async fn test_downloads() {
    let songs = get_api().await.search_songs("Beatles").await.unwrap();
    futures::future::join_all(songs.into_iter().take(5).map(|s| async move {
        eprintln!("Downloading {} {}", s.video_id.get_raw(), s.title);
        let video = Video::new_with_options(s.video_id.get_raw(), get_video_options()).unwrap();
        let stream = video.stream().await.unwrap();
        while stream.chunk().await.unwrap().is_some() {}
    }))
    .await;
}
