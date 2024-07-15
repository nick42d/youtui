use super::{get_sort_order_params, GetLibrarySortOrder, Query};
use crate::{
    common::{UploadAlbumID, UploadArtistID, UploadEntityID},
    parse::{ApiSuccess, GetLibraryUploadAlbum, TableListUploadSong, UploadAlbum, UploadArtist},
};
use serde_json::json;

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
// Auth required
impl<'a> Query for GetLibraryUploadAlbumQuery<'a> {
    type Output = GetLibraryUploadAlbum
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!(self.upload_album_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<'a> Query for GetLibraryUploadArtistQuery<'a> {
    type Output = Vec<TableListUploadSong>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!(self.upload_artist_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl Query for GetLibraryUploadSongsQuery {
    type Output = Vec<TableListUploadSong>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "browseId".to_string(),
            json!("FEmusic_library_privately_owned_tracks"),
        )])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        get_sort_order_params(&self.sort_order).map(Into::into)
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl Query for GetLibraryUploadAlbumsQuery {
    type Output = Vec<UploadAlbum>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "browseId".to_string(),
            json!("FEmusic_library_privately_owned_releases"),
        )])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        get_sort_order_params(&self.sort_order).map(Into::into)
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl Query for GetLibraryUploadArtistsQuery {
    type Output = Vec<UploadArtist>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "browseId".to_string(),
            json!("FEmusic_library_privately_owned_artists"),
        )])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        get_sort_order_params(&self.sort_order).map(Into::into)
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Auth required
impl<'a> Query for DeleteUploadEntityQuery<'a> {
    type Output = ApiSuccess
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("entityId".to_string(), json!(self.upload_entity_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "music/delete_privately_owned_entity"
    }
}
