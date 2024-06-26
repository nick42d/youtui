use super::{
    search_query_header, search_query_params, Query, SearchQuery, SearchType, SpellingMode,
    SEARCH_QUERY_PATH, SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS, SPECIALIZED_PLAYLIST_PREFIX_PARAMS,
    SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
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
    fn filtered_param_bits(&self) -> Cow<str>;
    // By implementing a default method, we can specialize for cases were these
    // params are incorrect.
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<str> {
        match spelling_mode {
            SpellingMode::ExactMatch => "AWoMEA4QChADEAQQCRAF".into(),
            SpellingMode::WithSuggestions => "AUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".into(),
        }
    }
    // By implementing a default method, we can specialize for cases were these
    // params are incorrect.
    fn filtered_prefix_param(&self) -> Cow<str> {
        "EgWKAQ".into()
    }
}
#[derive(Default, Debug, Clone, PartialEq)]
pub struct FilteredSearch<F: FilteredSearchType> {
    pub filter: F,
}
#[derive(Default, PartialEq, Debug, Clone)]
pub struct SongsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct VideosFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct AlbumsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ArtistsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct PlaylistsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct CommunityPlaylistsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct FeaturedPlaylistsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct EpisodesFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct PodcastsFilter;
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ProfilesFilter;

impl<F: FilteredSearchType> SearchType for FilteredSearch<F> {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<str>> {
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
    fn filtered_param_bits(&self) -> Cow<str> {
        "II".into()
    }
}
impl FilteredSearchType for VideosFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "IQ".into()
    }
}
impl FilteredSearchType for AlbumsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "IY".into()
    }
}
impl FilteredSearchType for ArtistsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "Ig".into()
    }
}
impl FilteredSearchType for PlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        // When filtering for Library params should be "Io"...
        "".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<str> {
        match spelling_mode {
            SpellingMode::ExactMatch => "MABCAggBagoQBBADEAkQBRAK",
            SpellingMode::WithSuggestions => "MABqChAEEAMQCRAFEAo%3D",
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<str> {
        "Eg-KAQwIABAAGAAgACgB".into()
    }
}
impl FilteredSearchType for CommunityPlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "EA".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<str> {
        match spelling_mode {
            SpellingMode::ExactMatch => SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS,
            SpellingMode::WithSuggestions => SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<str> {
        SPECIALIZED_PLAYLIST_PREFIX_PARAMS.into()
    }
}
impl FilteredSearchType for FeaturedPlaylistsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "Dg".into()
    }
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<str> {
        match spelling_mode {
            SpellingMode::ExactMatch => SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS,
            SpellingMode::WithSuggestions => SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS,
        }
        .into()
    }
    fn filtered_prefix_param(&self) -> Cow<str> {
        SPECIALIZED_PLAYLIST_PREFIX_PARAMS.into()
    }
}
impl FilteredSearchType for EpisodesFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "JI".into()
    }
}
impl FilteredSearchType for PodcastsFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "JQ".into()
    }
}
impl FilteredSearchType for ProfilesFilter {
    fn filtered_param_bits(&self) -> Cow<str> {
        "JY".into()
    }
}
// Implementations of Query
impl<'a> Query for SearchQuery<'a, FilteredSearch<SongsFilter>> {
    type Output = Vec<SearchResultSong>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<PlaylistsFilter>> {
    type Output = Vec<SearchResultPlaylist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>> {
    type Output = Vec<SearchResultPlaylist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<AlbumsFilter>> {
    type Output = Vec<SearchResultAlbum>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<ArtistsFilter>> {
    type Output = Vec<SearchResultArtist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>> {
    type Output = Vec<SearchResultFeaturedPlaylist>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<EpisodesFilter>> {
    type Output = Vec<SearchResultEpisode>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<PodcastsFilter>> {
    type Output = Vec<SearchResultPodcast>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<VideosFilter>> {
    type Output = Vec<SearchResultVideo>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
impl<'a> Query for SearchQuery<'a, FilteredSearch<ProfilesFilter>> {
    type Output = Vec<SearchResultProfile>;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(&self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Option<Cow<str>> {
        search_query_params(&self)
    }
}
