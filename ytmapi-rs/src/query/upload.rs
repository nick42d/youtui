use super::{get_sort_order_params, GetLibrarySortOrder, Query};
use crate::{
    common::{UploadAlbumID, UploadArtistID},
    parse::{TableListUploadSong, UploadAlbum},
};
use serde_json::json;

#[derive(Default)]
pub struct GetLibraryUploadSongsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryUploadArtistsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryUploadAlbumsQuery {
    sort_order: GetLibrarySortOrder,
}
pub struct GetLibraryUploadArtistQuery<'a> {
    upload_artist_id: UploadArtistID<'a>,
}
pub struct GetLibraryUploadAlbumQuery<'a> {
    upload_album_id: UploadAlbumID<'a>,
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
// Auth required
impl<'a> Query for GetLibraryUploadAlbumQuery<'a> {
    type Output = ()
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
    type Output = ()
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
    type Output = ()
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
