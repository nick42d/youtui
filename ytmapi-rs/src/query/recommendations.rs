use serde_json::{json, Value};

use super::Query;
use crate::{auth::AuthToken, common::recomendations::TasteToken, parse::TasteProfileArtist};

#[derive(Clone)]
pub struct GetTasteProfileQuery;

#[derive(Clone)]
pub struct SetTasteProfileQuery<'a, I>
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    taste_tokens: I,
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
    type Output = ()
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
