use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{AlbumID, YoutubeID};
use crate::parse::GetAlbum;
use serde_json::json;

#[derive(Clone)]
pub struct GetAlbumQuery<'a> {
    browse_id: AlbumID<'a>,
}
impl<A: AuthToken> Query<A> for GetAlbumQuery<'_> {
    type Output = GetAlbum;
    type Method = PostMethod;
}
impl PostQuery for GetAlbumQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
             "browseId" : self.browse_id.get_raw(),
        }) else {
            unreachable!("Created a map");
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}
impl<'a> GetAlbumQuery<'_> {
    pub fn new<T: Into<AlbumID<'a>>>(browse_id: T) -> GetAlbumQuery<'a> {
        GetAlbumQuery {
            browse_id: browse_id.into(),
        }
    }
}
