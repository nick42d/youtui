use super::library::{get_sort_order_params, GetLibrarySortOrder};
use super::{PostMethod, PostQuery, PostQueryCustom, Query};
use crate::auth::LoggedIn;
use crate::client::Body;
use crate::common::{
    ApiOutcome, UploadAlbumID, UploadArtistID, UploadEntityID, UploadUrl, YoutubeID,
};
use crate::parse::{
    GetLibraryUploadAlbum, ParseFrom, TableListUploadSong, UploadAlbum, UploadArtist,
};
use crate::utils::constants::DEFAULT_X_GOOG_AUTHUSER;
use crate::ProcessedResult;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::Deserialize;
use serde_json::json;
use std::borrow::Cow;
use std::convert::Into;
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::path::Path;

#[derive(Default, Clone)]
pub struct GetLibraryUploadSongsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default, Clone)]
pub struct GetLibraryUploadArtistsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default, Clone)]
pub struct GetLibraryUploadAlbumsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Clone)]
pub struct GetLibraryUploadArtistQuery<'a> {
    upload_artist_id: UploadArtistID<'a>,
}
#[derive(Clone)]
pub struct GetLibraryUploadAlbumQuery<'a> {
    upload_album_id: UploadAlbumID<'a>,
}
#[derive(Clone)]
/// Deletes a previously uploaded song or album.
pub struct DeleteUploadEntityQuery<'a> {
    upload_entity_id: UploadEntityID<'a>,
}

impl GetLibraryUploadSongsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryUploadArtistsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryUploadAlbumsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl<'a> GetLibraryUploadArtistQuery<'a> {
    pub fn new(upload_artist_id: UploadArtistID<'a>) -> Self {
        Self { upload_artist_id }
    }
}
impl<'a> GetLibraryUploadAlbumQuery<'a> {
    pub fn new(upload_album_id: UploadAlbumID<'a>) -> Self {
        Self { upload_album_id }
    }
}
impl<'a> DeleteUploadEntityQuery<'a> {
    pub fn new(upload_entity_id: UploadEntityID<'a>) -> Self {
        Self { upload_entity_id }
    }
}
impl GetUploadSongQuery {
    pub async fn new(file_path: impl AsRef<Path>) -> Option<Self> {
        let upload_filename = file_path
            .as_ref()
            .file_stem()
            .and_then(OsStr::to_str)
            // "Filename required for GetUploadSongQuery"
            .unwrap()
            .into();
        let upload_fileext: String = file_path
            .as_ref()
            .extension()
            .and_then(OsStr::to_str)
            // "Fileext required for GetUploadSongQuery"
            .unwrap()
            .into();
        if !ALLOWED_UPLOAD_EXTENSIONS
            .iter()
            .any(|ext| upload_fileext.as_str() == *ext)
        {
            panic!(
                "Fileext not in allowed list. Allowed values: {:?}",
                ALLOWED_UPLOAD_EXTENSIONS
            );
        }
        let song_file = tokio::fs::File::open(file_path).await.unwrap();
        let upload_filesize_bytes = song_file.metadata().await.unwrap().len();
        const MAX_UPLOAD_FILESIZE_MB: u64 = 300;
        if upload_filesize_bytes > MAX_UPLOAD_FILESIZE_MB * (1024 * 1024) {
            panic!(
                "Unable to upload song greater than {} MB, size is {} MB",
                MAX_UPLOAD_FILESIZE_MB,
                upload_filesize_bytes / (1024 * 1024)
            );
        }
        Some(Self {
            upload_filename,
            upload_fileext,
            song_file,
            upload_filesize_bytes,
        })
    }
    pub fn get_filename_as_string(&self) -> String {
        format!("{}.{}", self.upload_filename, self.upload_fileext)
    }
    pub fn get_filename_and_ext(&self) -> (&str, &str) {
        (&self.upload_filename, &self.upload_fileext)
    }
    /// Don't include the extension when renaming the file.
    pub fn rename_file(&mut self, s: impl Into<String>) {
        self.upload_filename = s.into();
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for GetLibraryUploadAlbumQuery<'_> {
    type Output = GetLibraryUploadAlbum;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadAlbumQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!(self.upload_album_id))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for GetLibraryUploadArtistQuery<'_> {
    type Output = Vec<TableListUploadSong>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadArtistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!(self.upload_artist_id))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for GetLibraryUploadSongsQuery {
    type Output = Vec<TableListUploadSong>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadSongsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let params = get_sort_order_params(&self.sort_order);
        if let Some(params) = params {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_privately_owned_tracks"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_privately_owned_tracks"),
            )])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for GetLibraryUploadAlbumsQuery {
    type Output = Vec<UploadAlbum>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadAlbumsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let params = get_sort_order_params(&self.sort_order);
        if let Some(params) = params {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_privately_owned_releases"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_privately_owned_releases"),
            )])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for GetLibraryUploadArtistsQuery {
    type Output = Vec<UploadArtist>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadArtistsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let params = get_sort_order_params(&self.sort_order);
        if let Some(params) = params {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_privately_owned_artists"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_privately_owned_artists"),
            )])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for DeleteUploadEntityQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for DeleteUploadEntityQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("entityId".to_string(), json!(self.upload_entity_id))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "music/delete_privately_owned_entity"
    }
}
