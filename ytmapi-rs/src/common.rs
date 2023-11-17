// Intended to be for structures that are also suitable to be reused by other libraries.
// As opposed to simply part of the interface.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::Error;

/// A search suggestion containing a list of TextRuns.
/// May be a history suggestion.

#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct SearchSuggestion {
    runs: Vec<TextRun>,
    suggestion_type: SuggestionType,
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
    pub fn take_text(self) -> String {
        match self {
            TextRun::Bold(s) => s,
            TextRun::Normal(s) => s,
        }
    }
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
            .fold(String::new(), |acc, r| acc + &r.get_text())
    }
    pub fn get_runs(&self) -> &[TextRun] {
        &self.runs
    }
    pub fn get_type(&self) -> SuggestionType {
        self.suggestion_type
    }
    pub fn new(suggestion_type: SuggestionType, runs: Vec<TextRun>) -> Self {
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

// Should this be a trait?
pub struct _Artist {
    name: String,
    browse_id: String,
    shuffle_id: String,
    radio_id: String,
}

#[derive(Debug, Clone)]
pub enum Explicit {
    IsExplicit,
    NotExplicit,
}

// Note, library album will also have artists field. How do we handle - are these two different
// types?
// Or, is Album a trait?
// XXX: I think this should become a trait.
#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub playlist_id: Option<String>,
    pub browse_id: String,
    pub category: Option<String>, // TODO change to enum
    pub thumbnails: Vec<Thumbnail>,
    pub year: Option<String>,
}

pub trait YoutubeID<'a> {
    fn get_raw(&self) -> &str;
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self;
}
pub trait BrowseID<'a>: YoutubeID<'a> {}

#[derive(Debug, Clone)]
pub enum AlbumType {
    Single,
    Album,
    EP,
}

#[derive(Debug, Clone)]
pub enum PlaylistType {
    CommunityPlaylists,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct BrowseParams<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct AlbumID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ChannelID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoID<'a>(Cow<'a, str>);
#[derive(PartialEq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct LyricsID<'a>(pub Cow<'a, str>);

impl<'a> YoutubeID<'a> for AlbumID<'a> {
    fn get_raw(&self) -> &str {
        &self.0
    }
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self {
        Self(raw_str.into())
    }
}
impl<'a> YoutubeID<'a> for VideoID<'a> {
    fn get_raw(&self) -> &str {
        &self.0
    }
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self {
        Self(raw_str.into())
    }
}
impl<'a> BrowseID<'a> for PlaylistID<'a> {}
impl<'a> YoutubeID<'a> for PlaylistID<'a> {
    fn get_raw(&self) -> &str {
        &self.0
    }
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self {
        Self(raw_str.into())
    }
}
impl<'a> BrowseID<'a> for ChannelID<'a> {}
impl<'a> YoutubeID<'a> for ChannelID<'a> {
    fn get_raw(&self) -> &str {
        &self.0
    }
    fn from_raw<S: Into<Cow<'a, str>>>(raw_str: S) -> Self {
        Self(raw_str.into())
    }
}

impl<'a> BrowseParams<'a> {
    pub fn from_raw<S>(raw_str: S) -> BrowseParams<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        Self(raw_str.into())
    }
    pub fn get_raw(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AlbumType {
    type Error = crate::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Album" => Ok(AlbumType::Album),
            "EP" => Ok(AlbumType::EP),
            "Single" => Ok(AlbumType::Single),
            x => Err(Error::other(format!("Error parsing AlbumType from {x}"))),
        }
    }
}

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
    impl Lyrics {
        pub fn get_lyrics(&self) -> &str {
            self.lyrics.as_str()
        }
        pub fn get_source(&self) -> &str {
            self.source.as_str()
        }
        pub fn new(lyrics: String, source: String) -> Self {
            Self { lyrics, source }
        }
    }
}
pub mod youtuberesult {
    use crate::{ChannelID, Thumbnail};

    use super::PlaylistID;

    pub trait YoutubeResult {
        fn get_core(&self) -> &ResultCore;
        // Note, mandatory for Song but not some others.
        fn get_set_video_id(&self) -> &Option<String> {
            &self.get_core().set_video_id
        }
        fn get_duration(&self) -> &Option<String> {
            &self.get_core().duration
        }
        fn get_feedback_tok_add(&self) -> &Option<String> {
            &self.get_core().feedback_tok_add
        }
        fn get_feedback_tok_rem(&self) -> &Option<String> {
            &self.get_core().feedback_tok_rem
        }
        fn get_title(&self) -> &String {
            &self.get_core().title
        }
        fn get_like_status(&self) -> &Option<String> {
            &self.get_core().like_status
        }
        fn get_thumbnails(&self) -> &Vec<Thumbnail> {
            &self.get_core().thumbnails
        }
        fn get_is_available(&self) -> &bool {
            &self.get_core().is_available
        }
        fn get_is_explicit(&self) -> &bool {
            &self.get_core().is_explicit
        }
        fn get_video_type(&self) -> &Option<String> {
            &self.get_core().video_type
        }
        fn get_channel_id(&self) -> &Option<ChannelID> {
            &self.get_core().browse_id
        }
        fn get_playlist_id(&self) -> &Option<PlaylistID> {
            &self.get_core().playlist_id
        }
        fn get_playlist_subtitle(&self) -> &Option<String> {
            &self.get_core().playlist_subtitle
        }
    }
    #[derive(Debug, Clone)]
    pub struct ResultCore {
        // video_id: VideoID<'static>, //Note this is mandatory for Song but not some others, this is a weakness of
        //this genericised approach.
        set_video_id: Option<String>,
        duration: Option<String>,
        feedback_tok_add: Option<String>,
        feedback_tok_rem: Option<String>,
        title: String,
        // albums don't contain track_no
        // track_no: usize,
        // songs don't contain artists.
        // artists: Vec<super::ParsedSongArtist>,
        // albums don't contain albums.
        // album: Option<ParsedSongAlbum>,
        like_status: Option<String>,
        thumbnails: Vec<super::Thumbnail>,
        is_available: bool,
        is_explicit: bool,
        video_type: Option<String>,
        // year: Option<String>,
        // Songs don't contain a year.
        // Should this be optional?
        browse_id: Option<ChannelID<'static>>,
        playlist_id: Option<PlaylistID<'static>>,
        playlist_subtitle: Option<String>, // Consider difference between None and Never for these
                                           // Options. Most likely is a better way to do this.
    }

    impl ResultCore {
        pub fn new(
            set_video_id: Option<String>,
            duration: Option<String>,
            feedback_tok_add: Option<String>,
            feedback_tok_rem: Option<String>,
            title: String,
            like_status: Option<String>,
            thumbnails: Vec<super::Thumbnail>,
            is_available: bool,
            is_explicit: bool,
            video_type: Option<String>,
            browse_id: Option<ChannelID<'static>>,
            playlist_id: Option<PlaylistID<'static>>,
            playlist_subtitle: Option<String>,
        ) -> Self {
            Self {
                set_video_id,
                duration,
                feedback_tok_add,
                feedback_tok_rem,
                title,
                like_status,
                thumbnails,
                is_available,
                is_explicit,
                video_type,
                browse_id,
                playlist_id,
                playlist_subtitle,
            }
        }
    }
}
