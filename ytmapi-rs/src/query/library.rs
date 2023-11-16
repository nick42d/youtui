use serde_json::json;

use super::Query;
use std::borrow::Cow;

pub struct GetLibraryPlaylistQuery {}
impl<'a> Query for GetLibraryPlaylistQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
             "browseId" : "FEmusic_liked_playlists",
        }) else {
            unreachable!("Created a map");
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}
impl<'a> GetLibraryPlaylistQuery {
    pub fn new() -> GetLibraryPlaylistQuery {
        GetLibraryPlaylistQuery {}
    }
}
