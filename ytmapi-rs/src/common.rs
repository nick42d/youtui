//! Re-usable core structures.
// Intended to be for structures that are also suitable to be reused by other
// libraries. As opposed to simply part of the interface.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A search suggestion containing a list of TextRuns.
/// May be a history suggestion.
#[derive(PartialEq, Debug, Clone, Deserialize)]
#[non_exhaustive]
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
    /// Note - allocates a new String.
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

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[must_use]
/// Indicates a result from an API action such as a 'delete playlist'
pub enum ApiOutcome {
    #[serde(alias = "STATUS_SUCCEEDED")]
    Success,
    #[serde(alias = "STATUS_FAILED")]
    Failure,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct Thumbnail {
    pub height: u64,
    pub width: u64,
    pub url: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
/// Set of both taste tokens.
// Intentionally not marked non_exhaustive - not expecting this to change.
// TODO: constructor
pub struct TasteToken<'a> {
    pub impression_value: TasteTokenImpression<'a>,
    pub selection_value: TasteTokenSelection<'a>,
}

/// Collection of required fields to identify and change library status.
// Intentionally not marked non_exhaustive - not expecting this to change.
#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct LibraryManager {
    pub status: LibraryStatus,
    pub add_to_library_token: FeedbackTokenAddToLibrary<'static>,
    pub remove_from_library_token: FeedbackTokenRemoveFromLibrary<'static>,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum LibraryStatus {
    #[serde(rename = "LIBRARY_SAVED")]
    InLibrary,
    #[serde(rename = "LIBRARY_ADD")]
    NotInLibrary,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum LikeStatus {
    #[serde(rename = "LIKE")]
    Liked,
    #[serde(rename = "DISLIKE")]
    Disliked,
    #[serde(rename = "INDIFFERENT")]
    /// Indifferent means that the song has not been liked or disliked.
    Indifferent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Explicit {
    IsExplicit,
    NotExplicit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlbumType {
    Single,
    Album,
    EP,
}

/// Type safe version of API ID used as part of YTM's interface.
pub trait YoutubeID<'a> {
    fn get_raw(&self) -> &str;
    // TODO: Create fallible version for when parsing is required. This could
    // possiby be a seperate trait YoutubeIDFallible
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self;
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenRemoveFromHistory<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenAddToLibrary<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTokenRemoveFromLibrary<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct BrowseParams<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct PodcastChannelParams<'a>(Cow<'a, str>);
// TODO: Add parsing - PlaylistID begining with VL should fail.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct AlbumID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct ArtistChannelID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct PodcastChannelID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationParams<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct ProfileID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct PodcastID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct VideoID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct UploadEntityID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct LyricsID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct SetVideoID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct UploadAlbumID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct UploadArtistID<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct TasteTokenSelection<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct TasteTokenImpression<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct MoodCategoryParams<'a>(Cow<'a, str>);
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct SongTrackingUrl<'a>(Cow<'a, str>);

impl_youtube_id!(UploadEntityID<'a>);
impl_youtube_id!(SetVideoID<'a>);
impl_youtube_id!(AlbumID<'a>);
impl_youtube_id!(UploadAlbumID<'a>);
impl_youtube_id!(UploadArtistID<'a>);
impl_youtube_id!(ProfileID<'a>);
impl_youtube_id!(PodcastID<'a>);
impl_youtube_id!(EpisodeID<'a>);
impl_youtube_id!(VideoID<'a>);
impl_youtube_id!(PlaylistID<'a>);
impl_youtube_id!(ArtistChannelID<'a>);
impl_youtube_id!(PodcastChannelID<'a>);
impl_youtube_id!(LyricsID<'a>);
impl_youtube_id!(BrowseParams<'a>);
impl_youtube_id!(PodcastChannelParams<'a>);
impl_youtube_id!(ContinuationParams<'a>);
impl_youtube_id!(FeedbackTokenRemoveFromHistory<'a>);
impl_youtube_id!(FeedbackTokenRemoveFromLibrary<'a>);
impl_youtube_id!(FeedbackTokenAddToLibrary<'a>);
impl_youtube_id!(TasteTokenImpression<'a>);
impl_youtube_id!(TasteTokenSelection<'a>);
impl_youtube_id!(MoodCategoryParams<'a>);
impl_youtube_id!(SongTrackingUrl<'a>);
