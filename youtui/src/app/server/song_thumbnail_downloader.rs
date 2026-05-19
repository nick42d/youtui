use crate::app::structures::{AlbumOrUploadAlbumID, ListSong, ListSongAlbum};
use crate::core::create_or_clean_directory;
use crate::get_data_dir;
use anyhow::{Context, anyhow};
use async_cell::sync::AsyncCell;
use fs_err::OpenOptions;
use futures::FutureExt;
use futures::future::try_join;
use rusty_ytdl::reqwest;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use ytmapi_rs::common::{AlbumID, UploadAlbumID, VideoID, YoutubeID};

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

/// Unique identifier for the thumbnail - dependent on the type of song.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum SongThumbnailID<'a> {
    Album(AlbumID<'a>),
    UploadAlbum(UploadAlbumID<'a>),
    Video(VideoID<'a>),
}
impl<'a> From<&'a ListSong> for SongThumbnailID<'a> {
    fn from(song: &'a ListSong) -> SongThumbnailID<'a> {
        match song.album.as_deref() {
            Some(ListSongAlbum {
                id: AlbumOrUploadAlbumID::Album(a),
                ..
            }) => SongThumbnailID::Album(a.into()),
            Some(ListSongAlbum {
                id: AlbumOrUploadAlbumID::UploadAlbum(a),
                ..
            }) => SongThumbnailID::UploadAlbum(a.into()),
            None => SongThumbnailID::Video((&song.video_id).into()),
        }
    }
}
impl std::fmt::Display for SongThumbnailID<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SongThumbnailID::Album(id) => write!(f, "A_{}", id.get_raw()),
            SongThumbnailID::UploadAlbum(id) => write!(f, "U_{}", id.get_raw()),
            SongThumbnailID::Video(id) => write!(f, "V_{}", id.get_raw()),
        }
    }
}
impl<'a> SongThumbnailID<'a> {
    /// Convert the SongThumbnailID to static lifetime (by cloning the
    /// underlying data).
    pub fn into_owned(self) -> SongThumbnailID<'static> {
        match self {
            SongThumbnailID::Album(id) => {
                let id_string = id.get_raw().to_owned();
                SongThumbnailID::Album(AlbumID::from_raw(id_string))
            }
            SongThumbnailID::UploadAlbum(id) => {
                let id_string = id.get_raw().to_owned();
                SongThumbnailID::UploadAlbum(UploadAlbumID::from_raw(id_string))
            }
            SongThumbnailID::Video(id) => {
                let id_string = id.get_raw().to_owned();
                SongThumbnailID::Video(VideoID::from_raw(id_string))
            }
        }
    }
}

#[derive(PartialEq)]
pub struct SongThumbnail {
    pub in_mem_image: image::DynamicImage,
    pub on_disk_path: std::path::PathBuf,
    pub song_thumbnail_id: SongThumbnailID<'static>,
}

// Custom debug format - otherwise in_mem_image will be displaying array of
// bytes...
impl std::fmt::Debug for SongThumbnail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlbumArt")
            .field("in_mem_image", &"image::DynamicImage")
            .field("on_disk_path", &self.on_disk_path)
            .field("song_thumbnail_id", &self.song_thumbnail_id)
            .finish()
    }
}

pub struct SongThumbnailDownloader {
    client: reqwest::Client,
    // For information about why this error is stringly typed, see DynamicApiError
    status: Arc<AsyncCell<Result<(), String>>>,
}

impl SongThumbnailDownloader {
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
    pub async fn download_song_thumbnail(
        &self,
        thumbnail_id: SongThumbnailID<'static>,
        thumbnail_url: String,
    ) -> anyhow::Result<SongThumbnail> {
        // Do not download album art until directory setup and clean has completed.
        self.status.get().await.map_err(|e| anyhow!(e))?;

        let album_art_dir = get_album_art_dir()?;

        let mut dir_contents = tokio::fs::read_dir(&album_art_dir).await?;
        let valid_dir_contents = tokio_stream::wrappers::ReadDirStream::new(dir_contents).
            filter_map(|maybe_dir_entry| {
                match maybe_dir_entry {
                    Ok(dir_entry) => Some(dir_entry),
                    Err(e) => {
                        warn!("Error <{e}> iterating through files in album art dir {}, ignoring this entry", album_art_dir.display());
                        None
                    },
                }
            });
        let valid_dir_files = futures::stream::StreamExt::filter(
            valid_dir_contents,
            |dir_entry| async {
                match dir_entry.file_type().await {
                    Ok(file_type) => file_type.is_file(),
                    Err(e) => {
                        warn!(
                            "Error <{e}> determining file type of entry {dir_entry:?} in album art dir {}, ignoring this entry",
                            album_art_dir.display()
                        );
                        false
                    }
                }
            },
        );
        let matching_album_art =
            futures::stream::StreamExt::filter_map(valid_dir_files, |dir_file| async {
                if dir_file
                    .path()
                    .file_prefix()
                    .and_then(|dir_file_prefix| dir_file_prefix.to_str())
                    // Youtui album art is valid unicode - ie YAA_{STRING}
                    // Therefore, we can ignore all invalid unicode files in this directory as they
                    // are not from Youtui.
                    .is_none_or(|dir_file_prefix| {
                        dir_file_prefix
                            != format!("{}{}", ALBUM_ART_FILENAME_PREFIX, thumbnail_id).as_str()
                    })
                {
                    warn!(
                        "Detected a file in youtui album art directory with invalid filename {}",
                        dir_file.file_name().display()
                    );
                    return None;
                }
                // Youtui will always write a file extension.
                let Some(file_ext) = dir_file.path().extension() else {
                    warn!(
                        "Detected a file in youtui album art directory with no extension {}",
                        dir_file.file_name().display()
                    );
                    return None;
                };
                // ...and it will be a valid image format extension.
                let Some(image_format) = image::ImageFormat::from_extension(file_ext) else {
                    warn!(
                        "Detected a file in youtui album art directory with invalid extension {}",
                        dir_file.file_name().display()
                    );
                    return None;
                };
                let image_bytes = match tokio::fs::read(dir_file.path()).await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        info!("Unable to read image {dir_file:?}, ignoring");
                        return None;
                    }
                };
                let image_reader = image::ImageReader::with_format(
                    std::io::Cursor::new(image_bytes),
                    image_format,
                );
                let image_decoded =
                    match tokio::task::spawn_blocking(|| image_reader.decode()).await {
                        Ok(Ok(img)) => img,
                        Ok(Err(e)) => {
                            warn!(
                                "Decoding image {} errored with error <{e}>, ignoring",
                                dir_file.file_name().display()
                            );
                            return None;
                        }
                        Err(e) => {
                            error!(
                                "Decoding image {} panicked with error <{e}>, ignoring",
                                dir_file.file_name().display()
                            );
                            return None;
                        }
                    };
                let dir_file_arc = Arc::new(dir_file);
                let dir_file_arc_clone = dir_file_arc.clone();
                match tokio::task::spawn_blocking(move || {
                    let now = SystemTime::now();
                    let times = std::fs::FileTimes::new()
                        .set_accessed(now)
                        .set_modified(now);
                    let file = OpenOptions::new().write(true).open(dir_file_arc.path())?;
                    file.set_times(times)?;
                    Ok::<_, std::io::Error>(())
                })
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => warn!(
                        "Error <{e} whilst trying to update timestamp on image {}",
                        dir_file_arc_clone.path().display()
                    ),
                    Err(e) => error!(
                        "Panicked whilst trying to update timestamp on {} with error <{e}>",
                        dir_file_arc_clone.path().display()
                    ),
                };
                Some(image_decoded)
            });
        let mut matching_album_art = std::pin::pin!(matching_album_art);
        matching_album_art.next().await;

        let url = reqwest::Url::parse(&thumbnail_url)?;
        let image_bytes = self.client.get(url).send().await?.bytes().await?;
        // `Bytes` is cheap to clone.
        let image_reader = image::ImageReader::new(std::io::Cursor::new(image_bytes.clone()))
            .with_guessed_format()?;
        let image_format = image_reader
            .format()
            .context("Unable to determine album art image format")?;
        let on_disk_path = get_album_art_dir()?
            .join(format!("{}{}", ALBUM_ART_FILENAME_PREFIX, thumbnail_id))
            .with_extension(image_format.extensions_str()[0]);
        let image_decoding_task = tokio::task::spawn_blocking(|| image_reader.decode());
        let (in_mem_image, _) = try_join(
            image_decoding_task.map(|res| res.map_err(anyhow::Error::from)),
            tokio::fs::write(&on_disk_path, image_bytes)
                .map(|res| res.map_err(anyhow::Error::from)),
        )
        .await?;
        Ok(SongThumbnail {
            in_mem_image: in_mem_image?,
            on_disk_path,
            song_thumbnail_id: thumbnail_id,
        })
    }
}
