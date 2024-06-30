use super::Query;
use crate::common::library::{LibraryArtist, Playlist};
use serde_json::json;
use std::borrow::Cow;

// NOTE: Authentication is required to use the queries in this module.
// Currently, all queries are implemented with authentication however in future
// this could be scaled back.

#[derive(Default)]
pub enum GetLibrarySortOrder {
    NameAsc,
    NameDesc,
    MostSongs,
    RecentlySaved,
    #[default]
    Default,
}

pub struct GetLibraryPlaylistsQuery;
#[derive(Default)]
// TODO: Method to add sort order
pub struct GetLibrarySongsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
// TODO: Method to add sort order
pub struct GetLibraryAlbumsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
// TODO: Method to add sort order
pub struct GetLibrarySubscriptionsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
// TODO: Method to add sort order
pub struct GetLibraryArtistsQuery {
    sort_order: GetLibrarySortOrder,
}

impl Query for GetLibraryPlaylistsQuery {
    type Output = Vec<Playlist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
             "browseId" : "FEmusic_liked_playlists"
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
impl Query for GetLibraryArtistsQuery {
    type Output = Vec<LibraryArtist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
             "browseId" : "FEmusic_library_corpus_track_artists"
        }) else {
            unreachable!("Created a map");
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Option<Cow<str>> {
        get_sort_order_params(&self.sort_order).map(|s| s.into())
    }
}

impl Query for GetLibrarySongsQuery {
    type Output = ()
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_liked_videos"))])
    }
    fn params(&self) -> Option<Cow<str>> {
        get_sort_order_params(&self.sort_order).map(|s| s.into())
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl Query for GetLibraryAlbumsQuery {
    type Output = ()
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_liked_albums"))])
    }
    fn params(&self) -> Option<Cow<str>> {
        get_sort_order_params(&self.sort_order).map(|s| s.into())
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl Query for GetLibrarySubscriptionsQuery {
    type Output = ()
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "browseId".to_string(),
            json!("FEmusic_library_corpus_artists"),
        )])
    }
    fn params(&self) -> Option<Cow<str>> {
        get_sort_order_params(&self.sort_order).map(|s| s.into())
    }
    fn path(&self) -> &str {
        "browse"
    }
}

fn get_sort_order_params(o: &GetLibrarySortOrder) -> Option<&'static str> {
    // determine order_params via
    // `.contents.singleColumnBrowseResultsRenderer.tabs[0] .tabRenderer.
    // content.sectionListRenderer.contents[1] .itemSectionRenderer.header.
    // itemSectionTabbedHeaderRenderer.endItems[1] .dropdownRenderer.
    // entries[].dropdownItemRenderer.onSelectCommand.browseEndpoint.params`
    // of `/youtubei/v1/browse` response
    match o {
        GetLibrarySortOrder::NameAsc => Some("ggMGKgQIARAA"),
        GetLibrarySortOrder::NameDesc => Some("ggMGKgQIABAB"),
        GetLibrarySortOrder::MostSongs => todo!(),
        GetLibrarySortOrder::RecentlySaved => Some("ggMGKgQIABAB"),
        GetLibrarySortOrder::Default => None,
    }
}
