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
use crate::auth::{raw_query_get, raw_query_post, AuthToken};
use crate::parse::ParseFrom;
use crate::{RawResult, Result};
use private::Sealed;
use std::borrow::Cow;
use std::fmt::Debug;
use std::future::Future;

pub mod album;
#[doc(inline)]
pub use album::GetAlbumQuery;
pub mod artist;
#[doc(inline)]
pub use artist::{GetArtistAlbumsQuery, GetArtistQuery};
pub mod continuations;
#[doc(inline)]
pub use continuations::GetContinuationsQuery;
pub mod history;
#[doc(inline)]
pub use history::{AddHistoryItemQuery, GetHistoryQuery, RemoveHistoryItemsQuery};
pub mod library;
#[doc(inline)]
pub use library::{
    EditSongLibraryStatusQuery, GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery,
    GetLibraryArtistsQuery, GetLibraryChannelsQuery, GetLibraryPlaylistsQuery,
    GetLibraryPodcastsQuery, GetLibrarySongsQuery,
};
pub mod playlist;
#[doc(inline)]
pub use playlist::{
    AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery, EditPlaylistQuery,
    GetPlaylistQuery, GetWatchPlaylistQuery, RemovePlaylistItemsQuery,
};
pub mod podcasts;
#[doc(inline)]
pub use podcasts::{
    GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery, GetPodcastQuery,
};
pub mod rate;
#[doc(inline)]
pub use rate::{RatePlaylistQuery, RateSongQuery};
pub mod recommendations;
#[doc(inline)]
pub use recommendations::{
    GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery, SetTasteProfileQuery,
};
pub mod search;
#[doc(inline)]
pub use search::{GetSearchSuggestionsQuery, SearchQuery};
pub mod song;
#[doc(inline)]
pub use song::{GetLyricsIDQuery, GetLyricsQuery, GetSongTrackingUrlQuery};
pub mod upload;
#[doc(inline)]
pub use upload::{
    DeleteUploadEntityQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
    GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
};

mod private {
    pub trait Sealed {}
}

/// Represents a query that can be passed to Innertube.
/// The Output associated type describes how to parse a result from the query,
/// and the Method associated type describes how to call the query.
pub trait Query<A: AuthToken>: Sized {
    type Output: ParseFrom<Self>;
    type Method: QueryMethod<Self, A>;
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
// Use of async fn in trait is OK here, trait is Sealed.
#[allow(async_fn_in_trait)]
pub trait QueryMethod<Q, A>: Sealed
where
    A: AuthToken,
{
    async fn call<'a>(
        query: &'a Q,
        client: &crate::client::Client,
        tok: &A,
    ) -> Result<RawResult<'a, Q, A>>;
}

impl Sealed for GetMethod {}
impl<Q, A> QueryMethod<Q, A> for GetMethod
where
    Q: GetQuery,
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
        raw_query_get(tok, client, query)
    }
}

impl Sealed for PostMethod {}
impl<Q, A> QueryMethod<Q, A> for PostMethod
where
    Q: PostQuery,
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
        raw_query_post(query, tok, client)
    }
}
