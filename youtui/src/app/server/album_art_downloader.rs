use crate::core::create_or_clean_directory;
use crate::get_data_dir;
use anyhow::{anyhow, Context};
use async_cell::sync::AsyncCell;
use futures::future::try_join;
use futures::FutureExt;
use rusty_ytdl::reqwest;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use ytmapi_rs::common::{AlbumID, YoutubeID};

// The directory and prefix are to protect the user - files in this directory
// with this prefix will be monitored by youtui and cleaned up when over a
// certain age.
const ALBUM_ART_DIR_PATH: &str = "album_art";
// "Youtui Album Art" if you were wondering.
const ALBUM_ART_FILENAME_PREFIX: &str = "YAA_";
const ALBUM_ART_IMAGE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(60 * 60 * 24 * 10); //10 days

fn get_album_art_dir() -> anyhow::Result<PathBuf> {
    get_data_dir().map(|dir| dir.join(ALBUM_ART_DIR_PATH))
}

#[derive(PartialEq)]
pub struct AlbumArt {
    pub in_mem_image: image::DynamicImage,
    pub on_disk_path: std::path::PathBuf,
    pub album_id: AlbumID<'static>,
}

// Custom derive - otherwise in_mem_image will be displaying array of bytes...
impl std::fmt::Debug for AlbumArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlbumArt")
            .field("in_mem_image", &"image::DynamicImage")
            .field("on_disk_path", &self.on_disk_path)
            .field("album_id", &self.album_id)
            .finish()
    }
}

pub struct AlbumArtDownloader {
    client: reqwest::Client,
    // For information about why this error is stringly typed, see DynamicApiError
    status: Arc<AsyncCell<Result<(), String>>>,
}

impl AlbumArtDownloader {
    pub fn new(client: reqwest::Client) -> Self {
        let status = AsyncCell::new().into_shared();
        let status_clone = status.clone();
        tokio::spawn(async move {
            info!("Setting up and cleaning album art directory");
            let Ok(album_art_dir) = get_album_art_dir() else {
                status_clone.set(Err("Error getting album art dir".to_string()));
                return;
            };
            match create_or_clean_directory(
                &album_art_dir,
                ALBUM_ART_FILENAME_PREFIX,
                ALBUM_ART_IMAGE_MAX_AGE,
            )
            .await
            {
                Ok(n) => {
                    info!("Cleaned up {n} old album art files");
                    status_clone.set(Ok(()));
                }
                Err(e) => {
                    error!("Error {e} setting up and cleaning album art directory");
                    status_clone.set(Err(format!("{e}")))
                }
            }
        });
        Self { client, status }
    }
    pub async fn download_album_art(
        &self,
        album_id: AlbumID<'static>,
        thumbnail_url: String,
    ) -> anyhow::Result<AlbumArt> {
        // Do not download album art until directory setup and clean has completed.
        self.status.get().await.map_err(|e| anyhow!(e))?;
        let url = reqwest::Url::parse(&thumbnail_url)?;
        let image_bytes = self.client.get(url).send().await?.bytes().await?;
        // `Bytes` is cheap to clone.
        let image_reader = image::ImageReader::new(std::io::Cursor::new(image_bytes.clone()))
            .with_guessed_format()?;
        let image_format = image_reader
            .format()
            .context("Unable to determine album art image format")?;
        let on_disk_path = get_album_art_dir()?
            .join(format!(
                "{}{}",
                ALBUM_ART_FILENAME_PREFIX,
                album_id.get_raw()
            ))
            .with_extension(image_format.extensions_str()[0]);
        let image_decoding_task = tokio::task::spawn_blocking(|| image_reader.decode());
        let (in_mem_image, _) = try_join(
            image_decoding_task.map(|res| res.map_err(anyhow::Error::from)),
            tokio::fs::write(&on_disk_path, image_bytes)
                .map(|res| res.map_err(anyhow::Error::from)),
        )
        .await?;
        Ok(AlbumArt {
            in_mem_image: in_mem_image?,
            on_disk_path,
            album_id,
        })
    }
}
