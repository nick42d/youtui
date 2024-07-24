use serde_json::{json, Value};

use super::Query;
use crate::{
    auth::AuthToken,
    common::{recomendations::TasteToken, MoodCategoryParams},
    parse::{ApiSuccess, MoodCategorySection, MoodPlaylistCategory, TasteProfileArtist},
};

#[derive(Clone)]
pub struct GetTasteProfileQuery;

#[derive(Clone)]
pub struct SetTasteProfileQuery<'a, I>
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    taste_tokens: I,
}

#[derive(Clone)]
pub struct GetMoodCategoriesQuery;

#[derive(Clone)]
pub struct GetMoodPlaylistsQuery<'a> {
    params: MoodCategoryParams<'a>,
}

impl<'a, I> SetTasteProfileQuery<'a, I>
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    pub fn new<II: IntoIterator<IntoIter = I>>(taste_tokens: II) -> Self {
        let taste_tokens = taste_tokens.into_iter();
        Self { taste_tokens }
    }
}

impl<'a> GetMoodPlaylistsQuery<'a> {
    pub fn new(params: MoodCategoryParams<'a>) -> Self {
        Self { params }
    }
}

impl<A: AuthToken> Query<A> for GetTasteProfileQuery {
    type Output = Vec<TasteProfileArtist>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_tastebuilder"))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<'a, A, I> Query<A> for SetTasteProfileQuery<'a, I>
where
    A: AuthToken,
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    type Output = ApiSuccess
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let (impression_tokens, selection_tokens): (Vec<Value>, Vec<Value>) = self
            .taste_tokens
            .clone()
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
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<A: AuthToken> Query<A> for GetMoodCategoriesQuery {
    type Output = Vec<MoodCategorySection>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_moods_and_genres"))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}

impl<'a, A: AuthToken> Query<A> for GetMoodPlaylistsQuery<'a> {
    type Output = Vec<MoodPlaylistCategory>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([
            (
                "browseId".to_string(),
                json!("FEmusic_moods_and_genres_category"),
            ),
            ("params".to_string(), json!(self.params)),
        ])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
