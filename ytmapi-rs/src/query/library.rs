// NOTE: Authentication is required to use the queries in this module.
// Currently, all queries are implemented with authentication however in future this could be scaled back.
use super::Query;
use serde_json::json;
use std::borrow::Cow;

pub struct GetLibraryPlaylistQuery;
impl Query for GetLibraryPlaylistQuery {
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
#[derive(Default)]
pub enum LibraryArtistsSortOrder {
    NameAsc,
    NameDesc,
    MostSongs,
    RecentlySaved,
    #[default]
    Default,
}

#[derive(Default)]
pub struct GetLibraryArtistsQuery {
    sort_order: LibraryArtistsSortOrder,
}
impl Query for GetLibraryArtistsQuery {
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
        // determine order_params via `.contents.singleColumnBrowseResultsRenderer.tabs[0]
        // .tabRenderer.content.sectionListRenderer.contents[1]
        // .itemSectionRenderer.header.itemSectionTabbedHeaderRenderer.endItems[1]
        // .dropdownRenderer.entries[].dropdownItemRenderer.onSelectCommand.browseEndpoint.params`
        // of `/youtubei/v1/browse` response
        match self.sort_order {
            LibraryArtistsSortOrder::NameAsc => Some("ggMGKgQIARAA"),
            LibraryArtistsSortOrder::NameDesc => Some("ggMGKgQIABAB"),
            LibraryArtistsSortOrder::MostSongs => todo!(),
            LibraryArtistsSortOrder::RecentlySaved => Some("ggMGKgQIABAB"),
            LibraryArtistsSortOrder::Default => None,
        }
    }
}
