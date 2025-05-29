use crate::get_data_dir;
use anyhow::{bail, Context};
use futures::future::{join, try_join};
use futures::FutureExt;
use rusty_ytdl::reqwest;
use std::path::PathBuf;
use ytmapi_rs::common::{AlbumID, Thumbnail, YoutubeID};

const ALBUM_ART_DIR_PATH: &str = "album_art";

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
}

impl AlbumArtDownloader {
    pub async fn new(client: reqwest::Client) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(get_album_art_dir()?).await?;
        Ok(Self { client })
    }
    pub async fn download_album_art(
        &self,
        album_id: AlbumID<'static>,
        mut thumbs: Vec<Thumbnail>,
    ) -> anyhow::Result<AlbumArt> {
        let Some(Thumbnail { height, width, url }) = thumbs.pop() else {
            bail!("No thumbnails provided!");
        };
        let url = reqwest::Url::parse(&url)?;
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
            image_decoding_task.map(|res| res.map_err(|e| anyhow::Error::from(e))),
            tokio::fs::write(&on_disk_path, image_bytes)
                .map(|res| res.map_err(|e| anyhow::Error::from(e))),
        )
        .await?;
        Ok(AlbumArt {
            in_mem_image: in_mem_image?,
            on_disk_path,
            album_id,
        })
    }
}
