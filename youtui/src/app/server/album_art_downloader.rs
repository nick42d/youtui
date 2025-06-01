use crate::core::create_or_clean_directory;
use crate::get_data_dir;
use anyhow::Context;
use async_cell::sync::AsyncCell;
use futures::future::try_join;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use rusty_ytdl::reqwest;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio_stream::wrappers::ReadDirStream;
use ytmapi_rs::common::{AlbumID, YoutubeID};

// The directory and prefix are to protect the user - files in this directory
// with this prefix will be monitored by youtui and cleaned up when over a
// certain age.
const ALBUM_ART_DIR_PATH: &str = "album_art";
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
            .finish()
    }
}

pub struct AlbumArtDownloader {
    client: reqwest::Client,
    status: Arc<AsyncCell<anyhow::Result<()>>>,
}

impl AlbumArtDownloader {
    pub async fn new(client: reqwest::Client) -> anyhow::Result<Self> {
        let status = AsyncCell::new().into_shared();
        tokio::spawn(async move {
            let album_art_dir = get_album_art_dir()?;
            create_or_clean_directory(
                &album_art_dir,
                ALBUM_ART_FILENAME_PREFIX,
                SystemTime::now(),
                ALBUM_ART_IMAGE_MAX_AGE,
            )
            .await?;
            Ok::<(), anyhow::Error>(())
        });
        Ok(Self { client, status })
    }
    pub async fn download_album_art(
        &self,
        album_id: AlbumID<'static>,
        thumbnail_url: String,
    ) -> anyhow::Result<AlbumArt> {
        let url = reqwest::Url::parse(&thumbnail_url)?;
        let image_bytes = self.client.get(url).send().await?.bytes().await?;
        // `Bytes` is cheap to clone.
        let image_reader = image::ImageReader::new(std::io::Cursor::new(image_bytes.clone()))
            .with_guessed_format()?;
        let image_format = image_reader
            .format()
            .context("Unable to determine album art image format")?;
        let on_disk_path = get_album_art_dir()?
            .join(album_id.get_raw())
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
