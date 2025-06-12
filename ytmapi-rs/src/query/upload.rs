use super::library::{get_sort_order_params, GetLibrarySortOrder};
use super::{PostFileMethod, PostFileQuery, PostMethod, PostQuery, Query};
use crate::auth::LoggedIn;
use crate::common::{ApiOutcome, UploadAlbumID, UploadArtistID, UploadEntityID, UploadUrl};
use crate::parse::{
    GetLibraryUploadAlbum, ParseFrom, TableListUploadSong, UploadAlbum, UploadArtist,
};
use crate::ProcessedResult;
use serde_json::json;
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::path::Path;

const ALLOWED_UPLOAD_EXTENSIONS: &[&str] = &["mp3", "m4a", "wma", "flac", "ogg"];

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
pub struct GetUploadSongQuery {
    upload_filename: String,
    upload_fileext: String,
    song_file: tokio::fs::File,
}
#[derive(Debug)]
// TODO: Custom debug due to the Bytes.
pub struct UploadSongQuery<'a> {
    upload_url: UploadUrl<'static>,
    upload_filename: String,
    song_file: &'a tokio::fs::File,
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
        Some(Self {
            upload_filename,
            upload_fileext,
            song_file,
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
impl<'a> UploadSongQuery<'a> {
    pub fn new(upload_url: UploadUrl<'a>, song_file: &'a tokio::fs::File) -> Self {
        Self {
            upload_url,
            song_file,
            upload_filename: todo!(),
        }
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
// Auth required
impl<'a, A: LoggedIn> Query<A> for &'a GetUploadSongQuery {
    type Output = UploadSongQuery<'a>;
    type Method = PostMethod;
}
impl PostQuery for &GetUploadSongQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!()
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        todo!()
    }
    fn path(&self) -> &str {
        todo!()
    }
}
// Auth required
impl<A: LoggedIn> Query<A> for UploadSongQuery<'_> {
    type Output = ApiOutcome;
    type Method = PostFileMethod;
}
impl PostFileQuery for UploadSongQuery<'_> {
    fn file(&self) -> tokio::fs::File {
        todo!()
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        todo!()
    }
    fn path(&self) -> &str {
        todo!()
    }
}
impl<'a> ParseFrom<&'a GetUploadSongQuery> for UploadSongQuery<'a> {
    fn parse_from(p: crate::ProcessedResult<&'a GetUploadSongQuery>) -> crate::Result<Self> {
        let ProcessedResult {
            query,
            source,
            json,
        } = p;
        Ok(UploadSongQuery {
            upload_url: todo!(),
            upload_filename: GetUploadSongQuery::get_filename_as_string(query),
            song_file: &query.song_file,
        })
    }
}
impl ParseFrom<UploadSongQuery<'_>> for ApiOutcome {
    fn parse_from(p: crate::ProcessedResult<UploadSongQuery<'_>>) -> crate::Result<Self> {
        let ProcessedResult {
            query,
            source,
            json,
        } = p;
        todo!()
    }
}
