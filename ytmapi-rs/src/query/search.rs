use super::*;
use std::borrow::Cow;

// TODO Seal
pub trait SearchType {}

// Should be Enum?
#[derive(Debug, Clone, PartialEq)]
pub struct BasicSearch;
#[derive(Debug, Clone, PartialEq)]
pub struct FilteredSearch;
#[derive(Debug, Clone, PartialEq)]
pub struct UploadSearch;
//TODO Seal
impl SearchType for BasicSearch {}
impl SearchType for FilteredSearch {}
impl SearchType for UploadSearch {}

#[derive(PartialEq, Debug, Clone)]
pub struct SearchQuery<'a, S: SearchType> {
    query: Cow<'a, str>,
    scope: Scope,
    // This is an Option, as we want set_filter to be a different function to unset_filter (not
    // possible if None is an enum variant).
    filter: Option<Filter>,
    spelling_mode: SpellingMode,
    searchtype: S,
}
impl<'a, S: SearchType> Query for SearchQuery<'a, S> {
    fn header(&self) -> Header {
        Header {
            key: "query".into(),
            // TODO: Remove allocation
            value: self.query.as_ref().into(),
        }
    }
    fn path(&self) -> &str {
        "search"
    }
    // Hardcoded for now to Artists, ignore spelling suggestions.
    // https://github.com/sigma67/ytmusicapi/blob/master/ytmusicapi/parsers/search.py#L145
    // TODO: Calculate this.
    fn params(&self) -> Option<Cow<str>> {
        // Start of paramater when filter is not a playlist type.
        let filter_param = "EgWKAQI";
        let param_bits = match &self.filter {
            None => String::new(),
            Some(f) => f.param_bits(),
        };
        match self.scope {
            // Params are fixed in this scenario.
            Scope::Uploads => return Some("agIYAw%3D%3D".into()),
            // Params are fixed in this scenario.
            Scope::All if self.filter == None => match self.spelling_mode {
                SpellingMode::ExactMatch => {
                    return Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D".into())
                }
                SpellingMode::WithSuggestions => return None,
            },
            Scope::All if self.filter == Some(Filter::Playlists) => {
                let filter_param = "Eg-KAQwIABAAGAAgACgB";
                match self.spelling_mode {
                    SpellingMode::ExactMatch => {
                        return Some(format!("{}MABCAggBagoQBBADEAkQBRAK", filter_param).into())
                    }
                    SpellingMode::WithSuggestions => {
                        return Some(format!("{}MABqChAEEAMQCRAFEAo%3D", filter_param).into())
                    }
                }
            }
            Scope::All
                if self.filter == Some(Filter::CommunityPlaylists)
                    || self.filter == Some(Filter::FeaturedPlaylists) =>
            {
                match self.spelling_mode {
                    SpellingMode::ExactMatch => {
                        return Some(
                            format!("EgeKAQQoA{}BagwQDhAKEAMQBBAJEAU%3D", param_bits).into(),
                        )
                    }
                    SpellingMode::WithSuggestions => {
                        return Some(
                            format!("EgeKAQQoA{}BagwQDhAKEAMQBBAJEAU%3D", param_bits).into(),
                        )
                    }
                }
            }
            Scope::All => match self.spelling_mode {
                SpellingMode::ExactMatch => {
                    return Some(
                        format!(
                            "{}{}AUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
                            filter_param, param_bits
                        )
                        .into(),
                    )
                }
                SpellingMode::WithSuggestions => {
                    return Some(
                        format!("{}{}AWoMEA4QChADEAQQCRAF", filter_param, param_bits).into(),
                    )
                }
            },

            Scope::Library => {
                if self.filter == None {
                    return Some("agIYBA%3D%3D".into());
                } else {
                    return Some(
                        format!("{}{}AWoKEAUQCRADEAoYBA%3D%3D", filter_param, param_bits).into(),
                    );
                }
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Filter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
    CommunityPlaylists,
    FeaturedPlaylists,
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Scope {
    Library,
    Uploads,
    All,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SpellingMode {
    ExactMatch,
    WithSuggestions,
}

impl Filter {
    // should be impl display?
    fn param_bits(&self) -> String {
        match self {
            Self::Songs => "I",
            Self::Artists => "g",
            Self::Videos => "Q",
            Self::Albums => "Y",
            Self::Playlists => "o",
            Self::FeaturedPlaylists => "Dg",
            Self::CommunityPlaylists => "EA",
            Self::None => "",
        }
        .into()
    }
}

// should be impl into instead?
// XXX: See if I can get strings or strs to turn into YTSearch more easily.
// This currently requires type annotations.
// By default, uses SpellingMode exactmatch.
impl<'a> From<String> for SearchQuery<'a, BasicSearch> {
    fn from(value: String) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: value.into(),
            scope: Scope::All,
            spelling_mode: SpellingMode::ExactMatch,
            filter: None,
            searchtype: BasicSearch {},
        }
    }
}

// By default, uses SpellingMode exactmatch.
impl<'a> SearchQuery<'a, BasicSearch> {
    // Consider making this take AsRef<str> instead us can give the borrowed str to the Cow.
    // Or, implement both...
    pub fn new<Q: Into<String>>(q: Q) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: q.into().into(),
            spelling_mode: SpellingMode::ExactMatch,
            scope: Scope::All,
            filter: None,
            searchtype: BasicSearch {},
        }
    }
}

impl<'a, S: SearchType> SearchQuery<'a, S> {
    pub fn set_spelling_mode(mut self, spelling_mode: SpellingMode) -> Self {
        self.spelling_mode = spelling_mode;
        self
    }
    pub fn set_query<Q: Into<String>>(mut self, query: Q) -> Self {
        self.query = query.into().into();
        self
    }
}

impl<'a> SearchQuery<'a, BasicSearch> {
    pub fn set_filter(self, filter: Filter) -> SearchQuery<'a, FilteredSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: self.scope,
            filter: Some(filter),
            searchtype: FilteredSearch {},
        }
    }
    pub fn set_scope_uploads(self) -> SearchQuery<'a, UploadSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: Scope::Uploads,
            filter: self.filter,
            searchtype: UploadSearch {},
        }
    }
    pub fn set_scope_library(mut self) -> Self {
        self.scope = Scope::Library;
        self
    }
    pub fn set_scope_public(mut self) -> Self {
        self.scope = Scope::All;
        self
    }
}

impl<'a> SearchQuery<'a, FilteredSearch> {
    pub fn set_filter(self, filter: Filter) -> SearchQuery<'a, FilteredSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: self.scope,
            filter: Some(filter),
            searchtype: FilteredSearch {},
        }
    }
    pub fn unset_filter(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: self.scope,
            filter: None,
            searchtype: BasicSearch {},
        }
    }
    pub fn set_scope_library(mut self) -> Self {
        self.scope = Scope::Library;
        self
    }
    pub fn set_scope_public(mut self) -> Self {
        self.scope = Scope::All;
        self
    }
}

impl<'a> SearchQuery<'a, UploadSearch> {
    pub fn unset_filter(self) -> SearchQuery<'a, BasicSearch> {
        // XXX: Typecasting could save allocations.
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: self.scope,
            filter: None,
            searchtype: BasicSearch {},
        }
    }
    pub fn set_scope_library(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: Scope::Library,
            filter: self.filter,
            searchtype: BasicSearch {},
        }
    }
    pub fn set_scope_public(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            scope: Scope::All,
            filter: self.filter,
            searchtype: BasicSearch {},
        }
    }
}
