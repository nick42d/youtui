use super::{get_sort_order_params, GetLibrarySortOrder, PostMethod, PostQuery, Query};
use crate::{
    auth::AuthToken,
    common::{UploadAlbumID, UploadArtistID, UploadEntityID},
    parse::{GetLibraryUploadAlbum, TableListUploadSong, UploadAlbum, UploadArtist},
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
impl<'a, A: AuthToken> Query<A> for GetLibraryUploadAlbumQuery<'a> {
    type Output = GetLibraryUploadAlbum;
    type Method = PostMethod;
}
impl<'a> PostQuery for GetLibraryUploadAlbumQuery<'a> {
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
impl<'a, A: AuthToken> Query<A> for GetLibraryUploadArtistQuery<'a> {
    type Output = Vec<TableListUploadSong>;
    type Method = PostMethod;
}
impl<'a> PostQuery for GetLibraryUploadArtistQuery<'a> {
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
impl<A: AuthToken> Query<A> for GetLibraryUploadSongsQuery {
    type Output = Vec<TableListUploadSong>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadSongsQuery {
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
impl<A: AuthToken> Query<A> for GetLibraryUploadAlbumsQuery {
    type Output = Vec<UploadAlbum>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadAlbumsQuery {
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
impl<A: AuthToken> Query<A> for GetLibraryUploadArtistsQuery {
    type Output = Vec<UploadArtist>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryUploadArtistsQuery {
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
impl<'a, A: AuthToken> Query<A> for DeleteUploadEntityQuery<'a> {
    type Output = ();
    type Method = PostMethod;
}
impl<'a> PostQuery for DeleteUploadEntityQuery<'a> {
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
