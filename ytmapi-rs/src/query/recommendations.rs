use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{MoodCategoryParams, TasteToken};
use crate::parse::{MoodCategorySection, MoodPlaylistCategory, TasteProfileArtist};
use serde_json::{json, Value};
use std::borrow::Cow;

#[derive(Clone)]
pub struct GetTasteProfileQuery;

#[derive(Clone)]
pub struct SetTasteProfileQuery<'a> {
    taste_tokens: Vec<TasteToken<'a>>,
}

#[derive(Clone)]
pub struct GetMoodCategoriesQuery;

#[derive(Clone)]
pub struct GetMoodPlaylistsQuery<'a> {
    params: MoodCategoryParams<'a>,
}

impl<'a> SetTasteProfileQuery<'a> {
    pub fn new(taste_tokens: impl IntoIterator<Item = TasteToken<'a>>) -> Self {
        let taste_tokens = taste_tokens.into_iter().collect();
        Self { taste_tokens }
    }
}

impl<'a> GetMoodPlaylistsQuery<'a> {
    pub fn new(params: MoodCategoryParams<'a>) -> Self {
        Self { params }
    }
}

impl<A: AuthToken> Query<A> for GetTasteProfileQuery {
    type Output = Vec<TasteProfileArtist>;
    type Method = PostMethod;
}
impl PostQuery for GetTasteProfileQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_tastebuilder"))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<'a, A> Query<A> for SetTasteProfileQuery<'a>
where
    A: AuthToken,
{
    type Output = ();
    type Method = PostMethod;
}
impl<'a> PostQuery for SetTasteProfileQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let (impression_tokens, selection_tokens): (Vec<Value>, Vec<Value>) = self
            .taste_tokens
            .iter()
            .map(|t| (json!(t.impression_value), json!(t.selection_value)))
            .unzip();
        serde_json::Map::from_iter([
            ("browseId".to_string(), json!("FEmusic_home")),
            (
                "formData".to_string(),
                json!({
                    "impressionValues": impression_tokens,
                    "selectedValues": selection_tokens
                }),
            ),
        ])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<A: AuthToken> Query<A> for GetMoodCategoriesQuery {
    type Output = Vec<MoodCategorySection>;
    type Method = PostMethod;
}
impl PostQuery for GetMoodCategoriesQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_moods_and_genres"))])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<A: AuthToken> Query<A> for GetMoodPlaylistsQuery<'_> {
    type Output = Vec<MoodPlaylistCategory>;
    type Method = PostMethod;
}
impl PostQuery for GetMoodPlaylistsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([
            (
                "browseId".to_string(),
                json!("FEmusic_moods_and_genres_category"),
            ),
            ("params".to_string(), json!(self.params)),
        ])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
