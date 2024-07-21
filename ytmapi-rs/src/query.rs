//! Type safe queries to pass to the API.
use crate::auth::AuthToken;
use crate::parse::ParseFrom;
use crate::{Result, YtMusic};
pub use album::*;
pub use artist::*;
pub use history::*;
pub use library::*;
pub use playlist::*;
pub use search::*;
use std::borrow::Cow;
use std::future::Future;
pub use upload::*;

mod artist;
mod history;
mod library;
mod playlist;
mod search;
mod upload;

// TODO: Check visibility.
/// Represents a query that can be passed to Innertube.
pub trait Query<A: AuthToken> {
    // TODO: Consider if it's possible to remove the Self: Sized restriction to turn
    // this into a trait object.
    type Output: ParseFrom<Self>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value>;
    fn params(&self) -> Option<Cow<str>>;
    fn path(&self) -> &str;
    fn call(self, yt: &YtMusic<A>) -> impl Future<Output = Result<Self::Output>>
    where
        Self: Sized,
    {
        async { Self::Output::parse_from(yt.processed_query(self).await?) }
    }
}

pub mod album {
    use super::Query;
    use crate::{
        auth::AuthToken,
        common::{AlbumID, YoutubeID},
        parse::AlbumParams,
    };
    use serde_json::json;
    use std::borrow::Cow;

    pub struct GetAlbumQuery<'a> {
        browse_id: AlbumID<'a>,
    }
    impl<'a, A: AuthToken> Query<A> for GetAlbumQuery<'a> {
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
    use crate::{
        auth::AuthToken,
        parse::{ParseFrom, ProcessedResult},
    };

    use super::{BasicSearch, Query, SearchQuery};
    use std::borrow::Cow;

    pub struct GetContinuationsQuery<Q> {
        continuation_params: String,
        query: Q,
    }
    impl<'a> ParseFrom<GetContinuationsQuery<SearchQuery<'a, BasicSearch>>> for () {
        fn parse_from(
            _: ProcessedResult<GetContinuationsQuery<SearchQuery<'a, BasicSearch>>>,
        ) -> crate::Result<Self> {
            todo!()
        }
    }
    // TODO: Output type
    impl<'a, A: AuthToken> Query<A> for GetContinuationsQuery<SearchQuery<'a, BasicSearch>>
    where
        SearchQuery<'a, BasicSearch>: Query<A>,
    {
        type Output = ();
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            self.query.header()
        }
        fn path(&self) -> &str {
            self.query.path()
        }
        fn params(&self) -> Option<Cow<str>> {
            Some(Cow::Borrowed(&self.continuation_params))
        }
    }
    impl<Q> GetContinuationsQuery<Q> {
        pub fn new(c_params: String, query: Q) -> GetContinuationsQuery<Q> {
            GetContinuationsQuery {
                continuation_params: c_params,
                query,
            }
        }
    }
}

pub mod lyrics {
    use super::Query;
    use crate::{
        auth::AuthToken,
        common::{browsing::Lyrics, LyricsID, YoutubeID},
    };
    use serde_json::json;
    use std::borrow::Cow;

    pub struct GetLyricsQuery<'a> {
        id: LyricsID<'a>,
    }
    impl<'a, A: AuthToken> Query<A> for GetLyricsQuery<'a> {
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
        auth::AuthToken,
        common::{watch::WatchPlaylist, PlaylistID, YoutubeID},
        VideoID,
    };
    use serde_json::json;
    use std::borrow::Cow;

    pub trait GetWatchPlaylistQueryID {
        fn get_video_id(&self) -> Option<Cow<str>>;
        fn get_playlist_id(&self) -> Cow<str>;
    }

    pub struct GetWatchPlaylistQuery<T: GetWatchPlaylistQueryID> {
        id: T,
    }
    pub struct VideoAndPlaylistID<'a> {
        video_id: VideoID<'a>,
        playlist_id: PlaylistID<'a>,
    }

    impl<'a> GetWatchPlaylistQueryID for VideoAndPlaylistID<'a> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            Some(self.video_id.get_raw().into())
        }

        fn get_playlist_id(&self) -> Cow<str> {
            self.playlist_id.get_raw().into()
        }
    }
    impl<'a> GetWatchPlaylistQueryID for VideoID<'a> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            Some(self.get_raw().into())
        }

        fn get_playlist_id(&self) -> Cow<str> {
            format!("RDAMVM{}", self.get_raw()).into()
        }
    }
    impl<'a> GetWatchPlaylistQueryID for PlaylistID<'a> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            None
        }
        fn get_playlist_id(&self) -> Cow<str> {
            self.get_raw().into()
        }
    }

    impl<T: GetWatchPlaylistQueryID, A: AuthToken> Query<A> for GetWatchPlaylistQuery<T> {
        type Output = WatchPlaylist;
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            let serde_json::Value::Object(mut map) = json!({
                "enablePersistentPlaylistPanel": true,
                "isAudioOnly": true,
                "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
                "playlistId" : self.id.get_playlist_id(),
            }) else {
                unreachable!()
            };
            if let Some(video_id) = self.id.get_video_id() {
                map.insert("videoId".to_string(), json!(video_id));
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
        pub fn new_from_video_id<T: Into<VideoID<'a>>>(
            id: T,
        ) -> GetWatchPlaylistQuery<VideoID<'a>> {
            GetWatchPlaylistQuery { id: id.into() }
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

pub mod rate {
    use serde_json::json;

    use crate::{
        auth::AuthToken,
        common::{PlaylistID, YoutubeID},
        parse::{ApiSuccess, LikeStatus},
        VideoID,
    };

    use super::Query;

    pub struct RateSongQuery<'a> {
        video_id: VideoID<'a>,
        rating: LikeStatus,
    }
    impl<'a> RateSongQuery<'a> {
        pub fn new(video_id: VideoID<'a>, rating: LikeStatus) -> Self {
            Self { video_id, rating }
        }
    }
    pub struct RatePlaylistQuery<'a> {
        playlist_id: PlaylistID<'a>,
        rating: LikeStatus,
    }
    impl<'a> RatePlaylistQuery<'a> {
        pub fn new(playlist_id: PlaylistID<'a>, rating: LikeStatus) -> Self {
            Self {
                playlist_id,
                rating,
            }
        }
    }

    // AUTH REQUIRED
    impl<'a, A: AuthToken> Query<A> for RateSongQuery<'a> {
        type Output = ApiSuccess
        where
            Self: Sized;
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            serde_json::Map::from_iter([(
                "target".to_string(),
                json!({"videoId" : self.video_id.get_raw()} ),
            )])
        }
        fn params(&self) -> Option<std::borrow::Cow<str>> {
            None
        }
        fn path(&self) -> &str {
            like_endpoint(&self.rating)
        }
    }

    // AUTH REQUIRED
    impl<'a, A: AuthToken> Query<A> for RatePlaylistQuery<'a> {
        type Output = ApiSuccess
        where
            Self: Sized;
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            serde_json::Map::from_iter([(
                "target".to_string(),
                json!({"playlistId" : self.playlist_id.get_raw()} ),
            )])
        }
        fn params(&self) -> Option<std::borrow::Cow<str>> {
            None
        }
        fn path(&self) -> &str {
            like_endpoint(&self.rating)
        }
    }

    fn like_endpoint(rating: &LikeStatus) -> &'static str {
        match *rating {
            LikeStatus::Liked => "like/like",
            LikeStatus::Disliked => "like/dislike",
            LikeStatus::Indifferent => "like/removelike",
        }
    }
}
