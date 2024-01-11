use super::*;
use std::borrow::Cow;

const SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS: &str = "BagwQDhAKEAMQBBAJEAU%3D";
const SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS: &str = "BQgIIAWoMEA4QChADEAQQCRAF";
const SPECIALIZED_PLAYLIST_PREFIX_PARAMS: &str = "EgeKAQQoA";

/// An API search query.
#[derive(PartialEq, Debug, Clone)]
pub struct SearchQuery<'a, S: SearchType> {
    query: Cow<'a, str>,
    spelling_mode: SpellingMode,
    searchtype: S,
}

// TODO Seal
// TODO: Add relevant parameters.
// Implements Default to allow simple implementation of Into<SearchQuery<S>>
pub trait SearchType: Default {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<str>>;
}
// TODO Seal
// TODO: Add param bits
// Implements Default to allow simple implementation of Into<SearchQuery<FilteredSearch<F>>>
pub trait FilteredSearchType: Default {
    fn filtered_param_bits(&self) -> Cow<str>;
    // By implementing a default method, we can specialize for cases were these params are incorrect.
    fn filtered_spelling_param(&self, spelling_mode: &SpellingMode) -> Cow<str> {
        match spelling_mode {
            SpellingMode::ExactMatch => "AWoMEA4QChADEAQQCRAF".into(),
            SpellingMode::WithSuggestions => "AUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".into(),
        }
    }
    // By implementing a default method, we can specialize for cases were these params are incorrect.
    fn filtered_prefix_param(&self) -> Cow<str> {
        "EgWKAQ".into()
    }
}

/// Whether or not to allow Google to attempt to auto correct spelling as part of the results.
/// Has no affect on Uploads or Library.
// XXX: May actually affect Library. To confirm.
#[derive(PartialEq, Debug, Clone, Default)]
pub enum SpellingMode {
    // My personal preference is to use ExactMatch by default, so that's what I've set.
    // Google's is WithSuggestions.
    #[default]
    ExactMatch,
    WithSuggestions,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct BasicSearch;
#[derive(Default, Debug, Clone, PartialEq)]
pub struct LibrarySearch;
#[derive(Default, Debug, Clone, PartialEq)]
pub struct UploadSearch;
#[derive(Default, Debug, Clone, PartialEq)]
pub struct FilteredSearch<F: FilteredSearchType> {
    filter: F,
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
impl SearchType for BasicSearch {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<str>> {
        match spelling_mode {
            SpellingMode::ExactMatch => return Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D".into()),
            SpellingMode::WithSuggestions => return None,
        }
    }
}
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
impl SearchType for UploadSearch {
    fn specialised_params(&self, _: &SpellingMode) -> Option<Cow<str>> {
        // TODO: Investigate if spelling suggestions take affect here.
        Some("agIYAw%3D%3D".into())
    }
}
impl SearchType for LibrarySearch {
    fn specialised_params(&self, _: &SpellingMode) -> Option<Cow<str>> {
        // XXX: It may be possible to actually filter these, see sigma67/ytmusicapi for details.
        // TODO: Investigate if spelling suggestions take affect here.
        Some("agIYBA%3D%3D".into())
    }
}
impl<'a, S: SearchType> Query for SearchQuery<'a, S> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let value = self.query.as_ref().into();
        serde_json::Map::from_iter([("query".to_string(), value)])
    }
    fn path(&self) -> &str {
        "search"
    }
    fn params(&self) -> Option<Cow<str>> {
        self.searchtype.specialised_params(&self.spelling_mode)
    }
}

// This currently requires type annotations.
// By default, uses SpellingMode exactmatch.
impl<'a, Q: Into<Cow<'a, str>>, S: SearchType> From<Q> for SearchQuery<'a, S> {
    fn from(value: Q) -> SearchQuery<'a, S> {
        SearchQuery {
            query: value.into(),
            spelling_mode: SpellingMode::default(),
            searchtype: S::default(),
        }
    }
}

// By default, uses SpellingMode exactmatch.
impl<'a> SearchQuery<'a, BasicSearch> {
    pub fn new<Q: Into<Cow<'a, str>>>(q: Q) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: q.into(),
            spelling_mode: SpellingMode::default(),
            searchtype: BasicSearch {},
        }
    }
}

impl<'a, S: SearchType> SearchQuery<'a, S> {
    /// Set spelling mode.
    pub fn with_spelling_mode(mut self, spelling_mode: SpellingMode) -> Self {
        self.spelling_mode = spelling_mode;
        self
    }
    /// Chnage the set query.
    pub fn with_query<Q: Into<Cow<'a, str>>>(mut self, query: Q) -> Self {
        self.query = query.into();
        self
    }
}

impl<'a> SearchQuery<'a, BasicSearch> {
    /// Apply a filter to the search. May change type of results returned.
    pub fn with_filter<F: FilteredSearchType>(
        self,
        filter: F,
    ) -> SearchQuery<'a, FilteredSearch<F>> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: FilteredSearch { filter },
        }
    }
    /// Search only uploads.
    pub fn uploads(self) -> SearchQuery<'a, UploadSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: UploadSearch,
        }
    }
    /// Search only library.
    pub fn library(self) -> SearchQuery<'a, LibrarySearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: LibrarySearch,
        }
    }
}

impl<'a, F: FilteredSearchType> SearchQuery<'a, FilteredSearch<F>> {
    /// Apply a filter to the search. May change type of results returned.
    pub fn with_filter<F2: FilteredSearchType>(
        self,
        filter: F2,
    ) -> SearchQuery<'a, FilteredSearch<F2>> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: FilteredSearch { filter },
        }
    }
    /// Remove filter from the query.
    pub fn unfiltered(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: BasicSearch,
        }
    }
}

impl<'a> SearchQuery<'a, UploadSearch> {
    /// Change scope to search generally instead of Uploads.
    pub fn with_scope_public(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: BasicSearch,
        }
    }
}
impl<'a> SearchQuery<'a, LibrarySearch> {
    /// Change scope to search generally instead of Library.
    pub fn with_scope_public(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            searchtype: BasicSearch,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct GetSearchSuggestionsQuery<'a> {
    query: Cow<'a, str>,
}

impl<'a> GetSearchSuggestionsQuery<'a> {
    fn new<S: Into<Cow<'a, str>>>(value: S) -> GetSearchSuggestionsQuery<'a> {
        GetSearchSuggestionsQuery {
            query: value.into(),
        }
    }
}

impl<'a, S: Into<Cow<'a, str>>> From<S> for GetSearchSuggestionsQuery<'a> {
    fn from(value: S) -> GetSearchSuggestionsQuery<'a> {
        GetSearchSuggestionsQuery::new(value)
    }
}

impl<'a> Query for GetSearchSuggestionsQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let value = self.query.as_ref().into();
        serde_json::Map::from_iter([("input".into(), value)])
    }
    fn path(&self) -> &str {
        "music/get_search_suggestions"
    }
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}
