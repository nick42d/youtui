//! Type safe queries to pass to the API, and the traits to allow you to
//! implement new ones.
//! # Implementation example
//! Note, to implement Query, you must also meet the trait bounds for
//! QueryMethod. In practice, this means you must implement both Query and
//! PostQuery when using PostMethod, and Query and GetQuery when using
//! GetMethod.
//! In addition, note that your output type will need to implement ParseFrom -
//! see [`crate::parse`] for implementation notes.
//! ```no_run
//! # #[derive(Debug)]
//! # struct Date;
//! # impl ytmapi_rs::parse::ParseFrom<GetDateQuery> for Date {
//! #     fn parse_from(_: ytmapi_rs::parse::ProcessedResult<GetDateQuery>) -> ytmapi_rs::Result<Self> {todo!()}
//! # }
//! struct GetDateQuery;
//! impl ytmapi_rs::query::Query<ytmapi_rs::auth::BrowserToken> for GetDateQuery {
//!     type Output = Date;
//!     type Method = ytmapi_rs::query::PostMethod;
//! }
//! // Note that this is not a real Innertube endpoint - example for reference only!
//! impl ytmapi_rs::query::PostQuery for GetDateQuery {
//!     fn header(&self) -> serde_json::Map<String, serde_json::Value> {
//!         serde_json::Map::from_iter([("get_date".to_string(), serde_json::json!("YYYYMMDD"))])
//!     }
//!     fn params(&self) -> Vec<(&str, std::borrow::Cow<str>)> {
//!         vec![]
//!     }
//!     fn path(&self) -> &str {
//!         "date"
//!     }
//! }
//! ```
use crate::auth::AuthToken;
use crate::parse::ParseFrom;
use crate::{RawResult, Result};
use std::borrow::Cow;
use std::future::Future;

use private::Sealed;

#[doc(inline)]
pub use album::GetAlbumQuery;
#[doc(inline)]
pub use artist::{GetArtistAlbumsQuery, GetArtistQuery};
#[doc(inline)]
pub use continuations::GetContinuationsQuery;
#[doc(inline)]
pub use history::{AddHistoryItemQuery, GetHistoryQuery, RemoveHistoryItemsQuery};
#[doc(inline)]
pub use library::{
    EditSongLibraryStatusQuery, GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery,
    GetLibraryArtistsQuery, GetLibraryPlaylistsQuery, GetLibrarySongsQuery,
};
#[doc(inline)]
pub use lyrics::GetLyricsQuery;
#[doc(inline)]
pub use playlist::{
    AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery, EditPlaylistQuery,
    GetPlaylistQuery, RemovePlaylistItemsQuery,
};
#[doc(inline)]
pub use podcasts::{
    GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery, GetPodcastQuery,
};
#[doc(inline)]
pub use rate::{RatePlaylistQuery, RateSongQuery};
#[doc(inline)]
pub use recommendations::{
    GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery, SetTasteProfileQuery,
};
#[doc(inline)]
pub use search::{GetSearchSuggestionsQuery, SearchQuery};
#[doc(inline)]
pub use song::GetSongTrackingUrlQuery;
#[doc(inline)]
pub use upload::{
    DeleteUploadEntityQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
    GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
};
#[doc(inline)]
pub use watch::GetWatchPlaylistQuery;

pub mod artist;
pub mod continuations;
pub mod history;
pub mod library;
pub mod playlist;
pub mod podcasts;
pub mod recommendations;
pub mod search;
pub mod upload;

mod private {
    pub trait Sealed {}
}

/// Represents a query that can be passed to Innertube.
/// The Output associated type describes how to parse a result from the query,
/// and the Method associated type describes how to call the query.
pub trait Query<A: AuthToken>: Sized {
    type Output: ParseFrom<Self>;
    type Method: QueryMethod<Self, A, Self::Output>;
}

/// Represents a plain POST query that can be sent to Innertube.
pub trait PostQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value>;
    fn params(&self) -> Vec<(&str, Cow<str>)>;
    fn path(&self) -> &str;
}
/// Represents a plain GET query that can be sent to Innertube.
pub trait GetQuery {
    fn url(&self) -> &str;
    fn params(&self) -> Vec<(&str, Cow<str>)>;
}

/// The GET query method
pub struct GetMethod;
/// The POST query method
pub struct PostMethod;

/// Represents a method of calling an query, using a query, client and auth
/// token. Not intended to be implemented by api users, the pre-implemented
/// GetMethod and PostMethod structs should be sufficient, and in addition,
/// async methods are required currently.
// Allow async_fn_in_trait required, as trait currently sealed.
#[allow(async_fn_in_trait)]
pub trait QueryMethod<Q, A, O>: Sealed
where
    Q: Query<A>,
    A: AuthToken,
{
    async fn call<'a>(
        query: &'a Q,
        client: &crate::client::Client,
        tok: &A,
    ) -> Result<RawResult<'a, Q, A>>;
}

impl Sealed for GetMethod {}
impl<Q, A, O> QueryMethod<Q, A, O> for GetMethod
where
    Q: GetQuery + Query<A, Output = O>,
    A: AuthToken,
{
    fn call<'a>(
        query: &'a Q,
        client: &crate::client::Client,
        tok: &A,
    ) -> impl Future<Output = Result<RawResult<'a, Q, A>>>
    where
        Self: Sized,
    {
        tok.raw_query_get(client, query)
    }
}

impl Sealed for PostMethod {}
impl<Q, A, O> QueryMethod<Q, A, O> for PostMethod
where
    Q: PostQuery + Query<A, Output = O>,
    A: AuthToken,
{
    fn call<'a>(
        query: &'a Q,
        client: &crate::client::Client,
        tok: &A,
    ) -> impl Future<Output = Result<RawResult<'a, Q, A>>>
    where
        Self: Sized,
    {
        tok.raw_query_post(client, query)
    }
}

pub mod album {
    use super::{PostMethod, PostQuery, Query};
    use crate::{
        auth::AuthToken,
        common::{AlbumID, YoutubeID},
        parse::GetAlbum,
    };
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
}

pub mod lyrics {
    use super::{PostMethod, PostQuery, Query};
    use crate::{
        auth::AuthToken,
        common::{LyricsID, YoutubeID},
        parse::Lyrics,
    };
    use serde_json::json;

    pub struct GetLyricsQuery<'a> {
        id: LyricsID<'a>,
    }
    impl<A: AuthToken> Query<A> for GetLyricsQuery<'_> {
        type Output = Lyrics;
        type Method = PostMethod;
    }
    impl PostQuery for GetLyricsQuery<'_> {
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
        fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
            vec![]
        }
    }
    impl<'a> GetLyricsQuery<'a> {
        pub fn new(id: LyricsID<'a>) -> GetLyricsQuery<'a> {
            GetLyricsQuery { id }
        }
    }
}

pub mod watch {
    use super::{PostMethod, PostQuery, Query};
    use crate::{
        auth::AuthToken,
        common::{PlaylistID, VideoID, YoutubeID},
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

    impl GetWatchPlaylistQueryID for VideoAndPlaylistID<'_> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            Some(self.video_id.get_raw().into())
        }

        fn get_playlist_id(&self) -> Cow<str> {
            self.playlist_id.get_raw().into()
        }
    }
    impl GetWatchPlaylistQueryID for VideoID<'_> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            Some(self.get_raw().into())
        }

        fn get_playlist_id(&self) -> Cow<str> {
            format!("RDAMVM{}", self.get_raw()).into()
        }
    }
    impl GetWatchPlaylistQueryID for PlaylistID<'_> {
        fn get_video_id(&self) -> Option<Cow<str>> {
            None
        }
        fn get_playlist_id(&self) -> Cow<str> {
            self.get_raw().into()
        }
    }

    impl<T: GetWatchPlaylistQueryID, A: AuthToken> Query<A> for GetWatchPlaylistQuery<T> {
        type Output = crate::parse::WatchPlaylist;
        type Method = PostMethod;
    }
    impl<T: GetWatchPlaylistQueryID> PostQuery for GetWatchPlaylistQuery<T> {
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
        fn params(&self) -> Vec<(&str, Cow<str>)> {
            vec![]
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
        ) -> GetWatchPlaylistQuery<VideoAndPlaylistID<'a>> {
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
        ) -> GetWatchPlaylistQuery<VideoAndPlaylistID<'a>> {
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
    use std::borrow::Cow;

    use super::{PostMethod, PostQuery, Query};
    use crate::{
        auth::LoggedIn,
        common::{LikeStatus, PlaylistID, VideoID, YoutubeID},
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

    impl<A: LoggedIn> Query<A> for RateSongQuery<'_> {
        type Output = ();
        type Method = PostMethod;
    }
    impl PostQuery for RateSongQuery<'_> {
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            serde_json::Map::from_iter([(
                "target".to_string(),
                json!({"videoId" : self.video_id.get_raw()} ),
            )])
        }
        fn params(&self) -> Vec<(&str, Cow<str>)> {
            vec![]
        }
        fn path(&self) -> &str {
            like_endpoint(&self.rating)
        }
    }

    impl<A: LoggedIn> Query<A> for RatePlaylistQuery<'_> {
        type Output = ();
        type Method = PostMethod;
    }

    impl PostQuery for RatePlaylistQuery<'_> {
        fn header(&self) -> serde_json::Map<String, serde_json::Value> {
            serde_json::Map::from_iter([(
                "target".to_string(),
                json!({"playlistId" : self.playlist_id.get_raw()} ),
            )])
        }
        fn params(&self) -> Vec<(&str, Cow<str>)> {
            vec![]
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
    use super::{PostMethod, PostQuery, Query};
    use crate::common::VideoID;
    use crate::{auth::AuthToken, common::SongTrackingUrl, Result};
    use serde_json::json;
    use std::borrow::Cow;
    use std::time::SystemTime;

    pub struct GetSongTrackingUrlQuery<'a> {
        video_id: VideoID<'a>,
        signature_timestamp: u64,
    }

    impl GetSongTrackingUrlQuery<'_> {
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

    impl<A: AuthToken> Query<A> for GetSongTrackingUrlQuery<'_> {
        type Output = SongTrackingUrl<'static>;
        type Method = PostMethod;
    }
    impl PostQuery for GetSongTrackingUrlQuery<'_> {
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
        fn params(&self) -> Vec<(&str, Cow<str>)> {
            vec![]
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
