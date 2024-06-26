//! Type safe queries to pass to the API.
use crate::auth::AuthToken;
use crate::parse::ParseFrom;
use crate::{Result, YtMusic};
pub use album::*;
pub use artist::*;
pub use library::*;
pub use playlist::*;
pub use search::*;
use std::borrow::Cow;
use std::future::Future;

mod artist;
mod library;
mod playlist;
mod search;

// TODO: Check visibility.
/// Represents a query that can be passed to Innertube.
pub trait Query {
    // TODO: Consider if it's possible to remove the Self: Sized restriction to turn
    // this into a trait object.
    type Output: ParseFrom<Self>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value>;
    fn params(&self) -> Option<Cow<str>>;
    fn path(&self) -> &str;
    fn call<A: AuthToken>(self, yt: &YtMusic<A>) -> impl Future<Output = Result<Self::Output>>
    where
        Self: Sized,
    {
        async { Self::Output::parse_from(yt.processed_query(self).await?) }
    }
}

pub mod album {
    use super::Query;
    use crate::{
        common::{AlbumID, YoutubeID},
        parse::AlbumParams,
    };
    use serde_json::json;
    use std::borrow::Cow;

    pub struct GetAlbumQuery<'a> {
        browse_id: AlbumID<'a>,
    }
    impl<'a> Query for GetAlbumQuery<'a> {
        type Output = AlbumParams;
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
    use crate::parse::{ParseFrom, ProcessedResult};

    use super::{BasicSearch, Query, SearchQuery};
    use std::borrow::Cow;

    pub struct GetContinuationsQuery<Q: Query> {
        c_params: String,
        query: Q,
    }
    impl<'a> ParseFrom<GetContinuationsQuery<SearchQuery<'a, BasicSearch>>> for () {
        fn parse_from(
            p: ProcessedResult<GetContinuationsQuery<SearchQuery<'a, BasicSearch>>>,
        ) -> crate::Result<<GetContinuationsQuery<SearchQuery<'a, BasicSearch>> as Query>::Output>
        {
            todo!()
        }
    }
    // TODO: Output type
    impl<'a> Query for GetContinuationsQuery<SearchQuery<'a, BasicSearch>> {
        type Output = ();
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
    use super::Query;
    use crate::common::{browsing::Lyrics, LyricsID, YoutubeID};
    use serde_json::json;
    use std::borrow::Cow;

    pub struct GetLyricsQuery<'a> {
        id: LyricsID<'a>,
    }
    impl<'a> Query for GetLyricsQuery<'a> {
        type Output = Lyrics;
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            let serde_json::Value::Object(map) = json!({
                "browseId": self.id.get_raw(),
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
    use super::Query;
    use crate::{
        common::{watch::WatchPlaylist, PlaylistID, YoutubeID},
        VideoID,
    };
    use serde_json::json;
    use std::borrow::Cow;

    pub struct VideoAndPlaylistID<'a> {
        video_id: VideoID<'a>,
        playlist_id: PlaylistID<'a>,
    }

    pub struct GetWatchPlaylistQuery<T> {
        id: T,
    }
    impl<'a> Query for GetWatchPlaylistQuery<VideoID<'a>> {
        type Output = WatchPlaylist;
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
