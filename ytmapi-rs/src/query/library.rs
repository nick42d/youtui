use super::{PostMethod, PostQuery, Query};
use crate::auth::LoggedIn;
use crate::common::{
    ApiOutcome, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary, YoutubeID,
};
use crate::parse::{
    LibraryArtist, LibraryArtistSubscription, LibraryChannel, LibraryPlaylist, LibraryPodcast,
    SearchResultAlbum, TableListSong,
};
use serde_json::json;
use std::borrow::Cow;

// NOTE: Authentication is required to use the queries in this module.
// Currently, all queries are implemented with authentication however in future
// this could be scaled back.

#[derive(Default, Clone)]
pub enum GetLibrarySortOrder {
    NameAsc,
    NameDesc,
    RecentlySaved,
    #[default]
    Default,
}

pub struct GetLibraryPlaylistsQuery;
#[derive(Default)]
pub struct GetLibrarySongsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryAlbumsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryArtistSubscriptionsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryArtistsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryPodcastsQuery {
    sort_order: GetLibrarySortOrder,
}
#[derive(Default)]
pub struct GetLibraryChannelsQuery {
    sort_order: GetLibrarySortOrder,
}
pub struct EditSongLibraryStatusQuery<'a> {
    add_to_library_feedback_tokens: Vec<FeedbackTokenAddToLibrary<'a>>,
    remove_from_library_feedback_tokens: Vec<FeedbackTokenRemoveFromLibrary<'a>>,
}

impl GetLibrarySongsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryAlbumsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryArtistSubscriptionsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryArtistsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryPodcastsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl GetLibraryChannelsQuery {
    pub fn new(sort_order: GetLibrarySortOrder) -> Self {
        Self { sort_order }
    }
}
impl<'a> EditSongLibraryStatusQuery<'a> {
    pub fn new_from_add_to_library_feedback_tokens(
        add_to_library_feedback_tokens: impl IntoIterator<Item = FeedbackTokenAddToLibrary<'a>>,
    ) -> Self {
        Self {
            add_to_library_feedback_tokens: add_to_library_feedback_tokens.into_iter().collect(),
            remove_from_library_feedback_tokens: vec![],
        }
    }
    pub fn new_from_remove_from_library_feedback_tokens(
        remove_from_library_feedback_tokens: impl IntoIterator<
            Item = FeedbackTokenRemoveFromLibrary<'a>,
        >,
    ) -> Self {
        Self {
            add_to_library_feedback_tokens: vec![],
            remove_from_library_feedback_tokens: remove_from_library_feedback_tokens
                .into_iter()
                .collect(),
        }
    }
    pub fn with_add_to_library_feedback_tokens(
        mut self,
        add_to_library_feedback_tokens: impl IntoIterator<Item = FeedbackTokenAddToLibrary<'a>>,
    ) -> Self {
        self.add_to_library_feedback_tokens = add_to_library_feedback_tokens.into_iter().collect();
        self
    }
    pub fn with_remove_from_library_feedback_tokens(
        mut self,
        remove_from_library_feedback_tokens: impl IntoIterator<
            Item = FeedbackTokenRemoveFromLibrary<'a>,
        >,
    ) -> Self {
        self.remove_from_library_feedback_tokens =
            remove_from_library_feedback_tokens.into_iter().collect();
        self
    }
}

impl<A: LoggedIn> Query<A> for GetLibraryPlaylistsQuery {
    type Output = Vec<LibraryPlaylist>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryPlaylistsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("browseId".to_string(), json!("FEmusic_liked_playlists"))])
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}
impl<A: LoggedIn> Query<A> for GetLibraryArtistsQuery {
    type Output = Vec<LibraryArtist>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryArtistsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            FromIterator::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_corpus_track_artists"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            FromIterator::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_corpus_track_artists"),
            )])
        }
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

impl<A: LoggedIn> Query<A> for GetLibrarySongsQuery {
    type Output = Vec<TableListSong>;
    type Method = PostMethod;
}
impl PostQuery for GetLibrarySongsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            serde_json::Map::from_iter([
                ("browseId".to_string(), json!("FEmusic_liked_videos")),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_liked_videos"))])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<A: LoggedIn> Query<A> for GetLibraryAlbumsQuery {
    type Output = Vec<SearchResultAlbum>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryAlbumsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            serde_json::Map::from_iter([
                ("browseId".to_string(), json!("FEmusic_liked_albums")),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_liked_albums"))])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<A: LoggedIn> Query<A> for GetLibraryArtistSubscriptionsQuery {
    type Output = Vec<LibraryArtistSubscription>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryArtistSubscriptionsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_corpus_artists"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_corpus_artists"),
            )])
        }
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// NOTE: Does not work on brand accounts
impl<A: LoggedIn> Query<A> for EditSongLibraryStatusQuery<'_> {
    type Output = Vec<ApiOutcome>;
    type Method = PostMethod;
}
impl PostQuery for EditSongLibraryStatusQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let add_feedback_tokens_raw = self
            .add_to_library_feedback_tokens
            .iter()
            .map(|t| t.get_raw());
        let feedback_tokens = self
            .remove_from_library_feedback_tokens
            .iter()
            .map(|t| t.get_raw())
            .chain(add_feedback_tokens_raw)
            .collect::<Vec<_>>();
        serde_json::Map::from_iter([("feedbackTokens".to_string(), json!(feedback_tokens))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "feedback"
    }
}
impl<A: LoggedIn> Query<A> for GetLibraryPodcastsQuery {
    type Output = Vec<LibraryPodcast>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryPodcastsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_non_music_audio_list"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_non_music_audio_list"),
            )])
        }
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}
impl<A: LoggedIn> Query<A> for GetLibraryChannelsQuery {
    type Output = Vec<LibraryChannel>;
    type Method = PostMethod;
}
impl PostQuery for GetLibraryChannelsQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        if let Some(params) = get_sort_order_params(&self.sort_order) {
            serde_json::Map::from_iter([
                (
                    "browseId".to_string(),
                    json!("FEmusic_library_non_music_audio_channels_list"),
                ),
                ("params".to_string(), json!(params)),
            ])
        } else {
            serde_json::Map::from_iter([(
                "browseId".to_string(),
                json!("FEmusic_library_non_music_audio_channels_list"),
            )])
        }
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

pub(crate) fn get_sort_order_params(o: &GetLibrarySortOrder) -> Option<&'static str> {
    match o {
        GetLibrarySortOrder::NameAsc => Some("ggMGKgQIARAA"),
        GetLibrarySortOrder::NameDesc => Some("ggMGKgQIARAB"),
        GetLibrarySortOrder::RecentlySaved => Some("ggMGKgQIABAB"),
        GetLibrarySortOrder::Default => None,
    }
}
