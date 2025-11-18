use super::{
    search_query_header, AuthToken, PostMethod, PostQuery, Query, SearchQuery, SearchType,
    SpellingMode, SEARCH_QUERY_PATH, SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS,
    SPECIALIZED_PLAYLIST_PREFIX_PARAMS, SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
};
use crate::parse::{
    SearchResultAlbum, SearchResultArtist, SearchResultEpisode, SearchResultFeaturedPlaylist,
    SearchResultPlaylist, SearchResultPodcast, SearchResultProfile, SearchResultSong,
    SearchResultVideo,
};
use std::borrow::Cow;

// TODO Seal
// TODO: Add param bits
// Implements Default to allow simple implementation of
// Into<SearchQuery<FilteredSearch<F>>>
pub trait FilteredSearchType: Default {
    fn filtered_param_bits(&self) -> Cow<'_, str>;
    // By implementing a default method, we can specialize for cases were these
    // params are incorrect.
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<'_, str> {
        match spelling_mode {
            SpellingMode::ExactMatch => "AWoMEA4QChADEAQQCRAF".into(),
            SpellingMode::WithSuggestions => "AUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".into(),
        }
    }
    // By implementing a default method, we can specialize for cases were these
    // params are incorrect.
    fn filtered_prefix_param(&self) -> Cow<'_, str> {
        "EgWKAQ".into()
    }
}
/// Helper struct for SearchQuery
#[derive(Default, Debug, Clone, PartialEq)]
pub struct FilteredSearch<F: FilteredSearchType> {
    pub(crate) filter: F,
}
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct SongsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct VideosFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct AlbumsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ArtistsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct PlaylistsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct CommunityPlaylistsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct FeaturedPlaylistsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct EpisodesFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct PodcastsFilter;
/// Helper struct for FilteredSearch type state pattern.
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ProfilesFilter;

impl<F: FilteredSearchType> SearchType for FilteredSearch<F> {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<'_, str>> {
        Some(
            format!(
                "{}{}{}",
                self.filter.filtered_prefix_param(),
                self.filter.filtered_param_bits(),
                self.filter.filtered_spelling_param(spelling_mode),
            )
            .into(),
        )
    }
}

// Implementations of FilteredSearchType
impl FilteredSearchType for SongsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "II".into()
    }
}
impl FilteredSearchType for VideosFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "IQ".into()
    }
}
impl FilteredSearchType for AlbumsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "IY".into()
    }
}
impl FilteredSearchType for ArtistsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "Ig".into()
    }
}
impl FilteredSearchType for PlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        // When filtering for Library params should be "Io"...
        "".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<'_, str> {
        match spelling_mode {
            SpellingMode::ExactMatch => "MABCAggBagoQBBADEAkQBRAK",
            SpellingMode::WithSuggestions => "MABqChAEEAMQCRAFEAo%3D",
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<'_, str> {
        "Eg-KAQwIABAAGAAgACgB".into()
    }
}
impl FilteredSearchType for CommunityPlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "EA".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<'_, str> {
        match spelling_mode {
            SpellingMode::ExactMatch => SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS,
            SpellingMode::WithSuggestions => SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<'_, str> {
        SPECIALIZED_PLAYLIST_PREFIX_PARAMS.into()
    }
}
impl FilteredSearchType for FeaturedPlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "Dg".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<'_, str> {
        match spelling_mode {
            SpellingMode::ExactMatch => SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS,
            SpellingMode::WithSuggestions => SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<'_, str> {
        SPECIALIZED_PLAYLIST_PREFIX_PARAMS.into()
    }
}
impl FilteredSearchType for EpisodesFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "JI".into()
    }
}
impl FilteredSearchType for PodcastsFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "JQ".into()
    }
}
impl FilteredSearchType for ProfilesFilter {
    fn filtered_param_bits(&self) -> Cow<'_, str> {
        "JY".into()
    }
}
// Implementations of Query
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<SongsFilter>> {
    type Output = Vec<SearchResultSong>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<SongsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<PlaylistsFilter>> {
    type Output = Vec<SearchResultPlaylist>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<PlaylistsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<CommunityPlaylistsFilter>> {
    type Output = Vec<SearchResultPlaylist>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<CommunityPlaylistsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<AlbumsFilter>> {
    type Output = Vec<SearchResultAlbum>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<AlbumsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<ArtistsFilter>> {
    type Output = Vec<SearchResultArtist>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<ArtistsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<FeaturedPlaylistsFilter>> {
    type Output = Vec<SearchResultFeaturedPlaylist>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<FeaturedPlaylistsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<EpisodesFilter>> {
    type Output = Vec<SearchResultEpisode>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<EpisodesFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<PodcastsFilter>> {
    type Output = Vec<SearchResultPodcast>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<PodcastsFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<VideosFilter>> {
    type Output = Vec<SearchResultVideo>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<VideosFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
impl<A: AuthToken> Query<A> for SearchQuery<'_, FilteredSearch<ProfilesFilter>> {
    type Output = Vec<SearchResultProfile>;
    type Method = PostMethod;
}
impl PostQuery for SearchQuery<'_, FilteredSearch<ProfilesFilter>> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}
