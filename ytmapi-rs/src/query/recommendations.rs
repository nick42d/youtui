use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{MoodCategoryParams, TasteToken};
use crate::parse::{MoodCategorySection, MoodPlaylistCategory, TasteProfileArtist};
use serde_json::{json, Value};
use std::borrow::Cow;

#[derive(Clone)]
pub struct GetTasteProfileQuery;

#[derive(Clone)]
pub struct SetTasteProfileQuery<I> {
    taste_tokens: I,
}

#[derive(Clone)]
pub struct GetMoodCategoriesQuery;

#[derive(Clone)]
pub struct GetMoodPlaylistsQuery<'a> {
    params: MoodCategoryParams<'a>,
}

impl<'a, I> SetTasteProfileQuery<I>
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

impl<'a, A, I> Query<A> for SetTasteProfileQuery<I>
where
    A: AuthToken,
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    type Output = ();
    type Method = PostMethod;
}
impl<'a, I> PostQuery for SetTasteProfileQuery<I>
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
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
