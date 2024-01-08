//! Type safe queries to pass to the API.
pub use album::*;
pub use artist::*;
pub use library::*;
pub use search::*;
use std::borrow::Cow;

mod artist;
mod library;
mod search;

// TODO: Check visibility.
/// Represents a query that can be passed to Innertube.
pub trait Query {
    // XXX: Consider if this should just return a tuple, Header seems overkill.
    // e.g fn header(&self) -> (Cow<str>, Cow<str>);
    fn header(&self) -> serde_json::Map<String, serde_json::Value>;
    fn params(&self) -> Option<Cow<str>>;
    fn path(&self) -> &str;
}

pub mod album {
    use super::Query;
    use crate::common::{AlbumID, YoutubeID};
    use serde_json::json;
    use std::borrow::Cow;

    pub struct GetAlbumQuery<'a> {
        browse_id: AlbumID<'a>,
    }
    impl<'a> Query for GetAlbumQuery<'a> {
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
        fn params(&self) -> Option<Cow<str>> {
            None
        }
    }
    impl<'a> GetAlbumQuery<'_> {
        pub fn new<T: Into<AlbumID<'a>>>(browse_id: T) -> GetAlbumQuery<'a> {
            GetAlbumQuery {
                browse_id: browse_id.into(),
            }
        }
    }
}

// For future use.
pub mod continuations {
    use std::borrow::Cow;

    use super::{FilteredSearch, FilteredSearchType, Query, SearchQuery};

    pub struct GetContinuationsQuery<Q: Query> {
        c_params: String,
        query: Q,
    }
    impl<'a, F: FilteredSearchType> Query
        for GetContinuationsQuery<SearchQuery<'a, FilteredSearch<F>>>
    {
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            self.query.header()
        }
        fn path(&self) -> &str {
            self.query.path()
        }
        fn params(&self) -> Option<Cow<str>> {
            Some(Cow::Borrowed(&self.c_params))
        }
    }
    impl<Q: Query> GetContinuationsQuery<Q> {
        pub fn new(c_params: String, query: Q) -> GetContinuationsQuery<Q> {
            GetContinuationsQuery { c_params, query }
        }
    }
}

pub mod lyrics {

    use std::borrow::Cow;

    use serde_json::json;

    use crate::common::LyricsID;

    use super::Query;

    pub struct GetLyricsQuery<'a> {
        id: LyricsID<'a>,
    }
    impl<'a> Query for GetLyricsQuery<'a> {
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            let serde_json::Value::Object(map) = json!({
                "browseId": self.id.0.as_ref(),
            }) else {
                unreachable!()
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
    impl<'a> GetLyricsQuery<'a> {
        pub fn new(id: LyricsID<'a>) -> GetLyricsQuery<'a> {
            GetLyricsQuery { id }
        }
    }
}

pub mod watch {

    use std::borrow::Cow;

    use serde_json::json;

    use crate::{
        common::{PlaylistID, YoutubeID},
        VideoID,
    };

    use super::Query;

    pub struct VideoAndPlaylistID<'a> {
        video_id: VideoID<'a>,
        playlist_id: PlaylistID<'a>,
    }

    pub struct GetWatchPlaylistQuery<T> {
        id: T,
    }
    impl<'a> Query for GetWatchPlaylistQuery<VideoID<'a>> {
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            let serde_json::Value::Object(map) = json!({
                "enablePersistentPlaylistPanel": true,
                "isAudioOnly": true,
                "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
                "videoId" : self.id.get_raw(),
                "playlistId" : format!("RDAMVM{}",self.id.get_raw()),
            }) else {
                unreachable!()
            };
            map
        }
        fn path(&self) -> &str {
            "next"
        }
        fn params(&self) -> Option<Cow<str>> {
            None
        }
    }
    impl<'a> GetWatchPlaylistQuery<VideoID<'a>> {
        pub fn new_from_video_id(id: VideoID<'a>) -> GetWatchPlaylistQuery<VideoID<'a>> {
            GetWatchPlaylistQuery { id }
        }
        pub fn with_playlist_id(
            self,
            playlist_id: PlaylistID<'a>,
        ) -> GetWatchPlaylistQuery<VideoAndPlaylistID> {
            GetWatchPlaylistQuery {
                id: VideoAndPlaylistID {
                    video_id: self.id,
                    playlist_id,
                },
            }
        }
    }
    impl<'a> GetWatchPlaylistQuery<PlaylistID<'a>> {
        pub fn new_from_playlist_id(id: PlaylistID<'a>) -> GetWatchPlaylistQuery<PlaylistID<'a>> {
            GetWatchPlaylistQuery { id }
        }
        pub fn with_video_id(
            self,
            video_id: VideoID<'a>,
        ) -> GetWatchPlaylistQuery<VideoAndPlaylistID> {
            GetWatchPlaylistQuery {
                id: VideoAndPlaylistID {
                    video_id,
                    playlist_id: self.id,
                },
            }
        }
    }
}
