//! Type safe queries to pass to the API.
use crate::auth::AuthToken;
use crate::parse::ParseFrom;
use crate::{RawResult, Result};
use std::borrow::Cow;
use std::future::Future;

pub use album::*;
pub use artist::*;
pub use history::*;
pub use library::*;
pub use playlist::*;
pub use recommendations::*;
pub use search::*;
pub use upload::*;

mod artist;
mod history;
mod library;
mod playlist;
mod recommendations;
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
    fn call(&self, tok: &A) -> Self::Output;
}
// TODO: Check visibility.
/// Represents a query that can be passed to Innertube.
pub trait QueryNew<A: AuthToken> {
    // TODO: Consider if it's possible to remove the Self: Sized restriction to turn
    // this into a trait object.
    type Output: ParseFrom<Self>
    where
        Self: Sized;
    fn call(&self, client: &reqwest::Client, tok: &A)
        -> impl Future<Output = Result<Self::Output>>;
}
pub trait QueryPost {
    fn header(&self) -> serde_json::Map<String, serde_json::Value>;
    fn params(&self) -> Option<Cow<str>>;
    fn path(&self) -> &str;
}
/// Represents a plain GET query that can be sent to Innertube.
pub trait QueryGet {
    fn url(&self) -> &str;
    fn params(&self) -> Vec<(&str, Cow<str>)>;
}
fn call<Q: QueryNew<A>, A: AuthToken>(
    s: Q,
    client: &reqwest::Client,
    tok: &A,
) -> impl Future<Output = Result<Q::Output>> {
    async {
        let raw = tok.raw_query_get(client, *self).await?;
        let proc = A::deserialize_json(raw)?;
        let ret = Q::Output::parse_from(proc);
        ret
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
    use super::Query;
    use crate::{
        auth::AuthToken,
        common::{PlaylistID, YoutubeID},
        parse::LikeStatus,
        VideoID,
    };
    use serde_json::json;

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
        type Output = ()
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
        type Output = ()
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

// Potentially better belongs within another module.
pub mod song {
    use super::Query;
    use crate::{auth::AuthToken, common::SongTrackingUrl, Result, VideoID};
    use serde_json::json;
    use std::time::SystemTime;

    pub struct GetSongTrackingUrlQuery<'a> {
        video_id: VideoID<'a>,
        signature_timestamp: u64,
    }

    impl<'a> GetSongTrackingUrlQuery<'a> {
        /// # NOTE
        /// A GetSongTrackingUrlQuery stores a timestamp, it's not recommended
        /// to store these for a long period of time. The constructor can fail
        /// due to a System Time error.
        pub fn new(video_id: VideoID) -> Result<GetSongTrackingUrlQuery<'_>> {
            let signature_timestamp = get_signature_timestamp()?;
            Ok(GetSongTrackingUrlQuery {
                video_id,
                signature_timestamp,
            })
        }
    }

    impl<'a, A: AuthToken> Query<A> for GetSongTrackingUrlQuery<'a> {
        type Output = SongTrackingUrl<'static>
        where
            Self: Sized;
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            serde_json::Map::from_iter([
                (
                    "playbackContext".to_string(),
                    json!(
                        {
                            "contentPlaybackContext": {
                                "signatureTimestamp": self.signature_timestamp
                            }
                        }
                    ),
                ),
                ("video_id".to_string(), json!(self.video_id)),
            ])
        }
        fn params(&self) -> Option<std::borrow::Cow<str>> {
            None
        }
        fn path(&self) -> &str {
            "player"
        }
    }

    // Original: https://github.com/sigma67/ytmusicapi/blob/a15d90c4f356a530c6b2596277a9d70c0b117a0c/ytmusicapi/mixins/_utils.py#L42
    /// Approximation for google's signatureTimestamp which would normally be
    /// extracted from base.js.
    fn get_signature_timestamp() -> Result<u64> {
        const SECONDS_IN_DAY: u64 = 60 * 60 * 24;
        Ok(SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            // SAFETY: SECONDS_IN_DAY is nonzero.
            .saturating_div(SECONDS_IN_DAY))
    }
}
