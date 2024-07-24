//! Re-usable core structures.
// Intended to be for structures that are also suitable to be reused by other
// libraries. As opposed to simply part of the interface.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A search suggestion containing a list of TextRuns.
/// May be a history suggestion.
#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct SearchSuggestion {
    pub runs: Vec<TextRun>,
    pub suggestion_type: SuggestionType,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Copy)]
pub enum SuggestionType {
    History,
    Prediction,
}

/// A block of text that may be boldened.
#[derive(PartialEq, Debug, Clone, Deserialize)]
pub enum TextRun {
    Bold(String),
    Normal(String),
}

impl TextRun {
    /// Take the text from the run, ignoring format.
    pub fn take_text(self) -> String {
        match self {
            TextRun::Bold(s) => s,
            TextRun::Normal(s) => s,
        }
    }
    /// Get a reference to the text from the run, ignoring format.
    pub fn get_text(&self) -> &str {
        match self {
            TextRun::Bold(s) => s,
            TextRun::Normal(s) => s,
        }
    }
}

impl SearchSuggestion {
    /// Gets the text of the runs concaternated into a String.
    /// Note - allocation required.
    pub fn get_text(&self) -> String {
        self.runs
            .iter()
            .fold(String::new(), |acc, r| acc + r.get_text())
    }
    pub(crate) fn new(suggestion_type: SuggestionType, runs: Vec<TextRun>) -> Self {
        Self {
            runs,
            suggestion_type,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Thumbnail {
    pub height: u64,
    pub width: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Explicit {
    IsExplicit,
    NotExplicit,
}

// Note, library album will also have artists field. How do we handle - are
// these two different types?
// Or, is Album a trait?
// XXX: Consider if this is the same as the Album struct that uses ResultCore.
// XXX: I think this should become a trait.
#[derive(Debug)]
pub struct Album {
    pub title: String,
    // TODO: Use type system
    pub playlist_id: Option<String>,
    // TODO: Use type system
    pub browse_id: AlbumID<'static>,
    pub category: Option<String>, // TODO change to enum
    pub thumbnails: Vec<Thumbnail>,
    pub year: Option<String>,
}

// TODO: Add parsing for YoutubeID's - e.g PlaylistID begining with VL should
// fail.
pub trait YoutubeID<'a> {
    fn get_raw(&self) -> &str;
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self;
}
// Need to confirm behaviour when converting from other IDs.
pub trait BrowseID<'a>: YoutubeID<'a> {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlbumType {
    Single,
    Album,
    EP,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenRemoveFromHistory<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenAddToLibrary<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenRemoveFromLibrary<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct BrowseParams<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct AlbumID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ChannelID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ProfileID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PodcastID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadEntityID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct LyricsID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SetVideoID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct UploadAlbumID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct UploadArtistID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TasteTokenSelection<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TasteTokenImpression<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct MoodCategoryParams<'a>(Cow<'a, str>);

impl_youtube_id!(UploadEntityID<'a>);
impl_youtube_id!(SetVideoID<'a>);
impl_youtube_id!(AlbumID<'a>);
impl_youtube_id!(UploadAlbumID<'a>);
impl_youtube_id!(UploadArtistID<'a>);
impl_youtube_id!(ProfileID<'a>);
impl_youtube_id!(PodcastID<'a>);
impl_youtube_id!(VideoID<'a>);
impl_youtube_id!(PlaylistID<'a>);
impl_youtube_id!(ChannelID<'a>);
impl_youtube_id!(LyricsID<'a>);
impl_youtube_id!(BrowseParams<'a>);
impl_youtube_id!(FeedbackTokenRemoveFromHistory<'a>);
impl_youtube_id!(FeedbackTokenRemoveFromLibrary<'a>);
impl_youtube_id!(FeedbackTokenAddToLibrary<'a>);
impl_youtube_id!(TasteTokenImpression<'a>);
impl_youtube_id!(TasteTokenSelection<'a>);
impl_youtube_id!(MoodCategoryParams<'a>);

impl<'a> BrowseID<'a> for PlaylistID<'a> {}
impl<'a> BrowseID<'a> for ChannelID<'a> {}

pub mod watch {
    use serde::Deserialize;

    use super::{LyricsID, PlaylistID};

    #[derive(PartialEq, Debug, Clone, Deserialize)]
    pub struct WatchPlaylist {
        // TODO: Implement tracks.
        pub _tracks: Vec<()>,
        pub playlist_id: Option<PlaylistID<'static>>,
        pub lyrics_id: LyricsID<'static>,
    }

    impl WatchPlaylist {
        // TODO: implement tracks.
        pub fn new(playlist_id: Option<PlaylistID<'static>>, lyrics_id: LyricsID<'static>) -> Self {
            Self {
                playlist_id,
                lyrics_id,
                _tracks: Default::default(),
            }
        }
    }
}

pub mod library {
    use crate::{ChannelID, Thumbnail};
    use serde::{Deserialize, Serialize};

    use super::PlaylistID;

    #[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
    pub struct Playlist {
        pub playlist_id: PlaylistID<'static>,
        pub title: String,
        pub thumbnails: Vec<Thumbnail>,
        pub count: Option<usize>,
        pub description: Option<String>,
        pub author: Option<String>,
    }
    #[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
    pub struct LibraryArtist {
        pub channel_id: ChannelID<'static>,
        pub artist: String,
        pub byline: String, // e.g 16 songs or 17.8k subscribers
    }
}

pub mod browsing {
    use serde::Deserialize;

    #[derive(PartialEq, Debug, Clone, Deserialize)]
    pub struct Lyrics {
        pub lyrics: String,
        pub source: String,
    }
}

pub mod recomendations {
    use super::{TasteTokenImpression, TasteTokenSelection};
    use serde::{Deserialize, Serialize};

    #[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
    // TODO: constructor
    pub struct TasteToken<'a> {
        pub impression_value: TasteTokenImpression<'a>,
        pub selection_value: TasteTokenSelection<'a>,
    }
}
