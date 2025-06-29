//! Results from parsing Innertube queries.
//! # Implementation example
//! ```no_run
//! # struct GetDateQuery;
//! use serde::Deserialize;
//! #[derive(Debug, Deserialize)]
//! struct Date {
//!     date_string: String,
//!     date_timestamp: usize,
//! }
//! impl ytmapi_rs::parse::ParseFrom<GetDateQuery> for Date {
//!     fn parse_from(
//!         p: ytmapi_rs::parse::ProcessedResult<GetDateQuery>,
//!     ) -> ytmapi_rs::Result<Self> {
//!         ytmapi_rs::json::from_json(p.json)
//!     }
//! }
//! ```
//! # Alternative implementation
//! An alternative to working directly with [`crate::json::Json`] is to add
//! `json-crawler` as a dependency and use the provided
//! `From<ProcessedResult> for JsonCrawlerOwned` implementation.
use crate::auth::AuthToken;
use crate::common::{AlbumID, ArtistChannelID, Thumbnail};
use crate::json::Json;
use crate::nav_consts::*;
use crate::{error, RawResult, Result};
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

mod album;
pub use album::*;
mod artist;
pub use artist::*;
mod history;
pub use history::*;
mod library;
pub use library::*;
mod playlist;
pub use playlist::*;
mod podcasts;
pub use podcasts::*;
mod rate;
// Whilst rate doesn't define anything - for consistency we still write the `pub
// use` statement.
#[allow(unused_imports)]
pub use rate::*;
mod recommendations;
pub use recommendations::*;
mod search;
pub use search::*;
mod song;
pub use song::*;
mod upload;
pub use upload::*;

/// Describes how to parse the ProcessedResult from a Query into the target
/// type.
// By requiring ParseFrom to also implement Debug, this simplifies our Query ->
// String API.
pub trait ParseFrom<Q>: Debug + Sized {
    fn parse_from(p: ProcessedResult<Q>) -> crate::Result<Self>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EpisodeDate {
    Live,
    Recorded { date: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EpisodeDuration {
    Live,
    Recorded { duration: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct ParsedSongArtist {
    pub name: String,
    pub id: Option<ArtistChannelID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct ParsedSongAlbum {
    pub name: String,
    pub id: AlbumID<'static>,
}

/// A result from the api that has been checked for errors and processed into
/// JSON.
pub struct ProcessedResult<'a, Q> {
    pub query: &'a Q,
    /// The raw string output returned from the web request to YouTube.
    pub source: String,
    /// The result once it has been deserialized from Json and processed to
    /// remove errors.
    pub json: Json,
}

impl<'a, Q, A: AuthToken> TryFrom<RawResult<'a, Q, A>> for ProcessedResult<'a, Q> {
    type Error = crate::Error;
    fn try_from(value: RawResult<'a, Q, A>) -> Result<Self> {
        let RawResult {
            json: source,
            query,
            ..
        } = value;
        let json = match source.as_str() {
            // Workaround for Get request returning empty string.
            "" => serde_json::Value::Null,
            other => serde_json::from_str(other)
                .map_err(|e| error::Error::response(format!("{:?}", e)))?,
        };
        let json = Json::new(json);
        Ok(Self {
            query,
            source,
            json,
        })
    }
}

impl<'a, Q> ProcessedResult<'a, Q> {
    pub(crate) fn destructure(self) -> (&'a Q, String, serde_json::Value) {
        let ProcessedResult {
            query,
            source,
            json,
        } = self;
        (query, source, json.inner)
    }
    pub(crate) fn get_json(&self) -> &serde_json::Value {
        &self.json.inner
    }
}

impl<Q> ProcessedResult<'_, Q> {
    pub fn parse_into<O: ParseFrom<Q>>(self) -> Result<O> {
        O::parse_from(self)
    }
}

impl<Q> From<ProcessedResult<'_, Q>> for JsonCrawlerOwned {
    fn from(value: ProcessedResult<Q>) -> Self {
        let (_, source, crawler) = value.destructure();
        JsonCrawlerOwned::new(source, crawler)
    }
}

fn fixed_column_item_pointer(col_idx: usize) -> String {
    format!("/fixedColumns/{col_idx}/musicResponsiveListItemFixedColumnRenderer")
}

fn flex_column_item_pointer(col_idx: usize) -> String {
    format!("/flexColumns/{col_idx}/musicResponsiveListItemFlexColumnRenderer")
}

// Should take FlexColumnItem? or Data?. Regular serde_json::Value could tryInto
// fixedcolumnitem also. Not sure if this should error.
// XXX: I think this should return none instead of error.
fn parse_song_artists(
    data: &mut impl JsonCrawler,
    col_idx: usize,
) -> Result<Vec<ParsedSongArtist>> {
    data.borrow_pointer(format!("{}/text/runs", flex_column_item_pointer(col_idx)))?
        .try_into_iter()?
        .step_by(2)
        .map(|mut item| parse_song_artist(&mut item))
        .collect()
}

fn parse_song_artist(data: &mut impl JsonCrawler) -> Result<ParsedSongArtist> {
    Ok(ParsedSongArtist {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
    })
}

fn parse_song_album(data: &mut impl JsonCrawler, col_idx: usize) -> Result<ParsedSongAlbum> {
    Ok(ParsedSongAlbum {
        name: parse_flex_column_item(data, col_idx, 0)?,
        id: data.take_value_pointer(format!(
            "{}/text/runs/0{}",
            flex_column_item_pointer(col_idx),
            NAVIGATION_BROWSE_ID
        ))?,
    })
}

fn parse_flex_column_item<T: DeserializeOwned>(
    item: &mut impl JsonCrawler,
    col_idx: usize,
    run_idx: usize,
) -> Result<T> {
    let pointer = format!(
        "{}/text/runs/{run_idx}/text",
        flex_column_item_pointer(col_idx)
    );
    Ok(item.take_value_pointer(pointer)?)
}

fn parse_fixed_column_item<T: DeserializeOwned>(
    item: &mut impl JsonCrawler,
    col_idx: usize,
) -> Result<T> {
    let pointer = format!("{}/text/runs/0/text", fixed_column_item_pointer(col_idx));
    Ok(item.take_value_pointer(pointer)?)
}
