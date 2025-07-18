use super::{GetMethod, GetQuery, PostMethod, PostQuery, Query};
use crate::auth::LoggedIn;
use crate::common::{ApiOutcome, FeedbackTokenRemoveFromHistory, SongTrackingUrl, YoutubeID};
use crate::parse::HistoryPeriod;
use rand::Rng;
use serde_json::json;
use std::borrow::Cow;

pub struct GetHistoryQuery;
pub struct RemoveHistoryItemsQuery<'a> {
    feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>,
}
pub struct AddHistoryItemQuery<'a> {
    song_url: SongTrackingUrl<'a>,
}

impl<'a> RemoveHistoryItemsQuery<'a> {
    pub fn new(
        feedback_tokens: impl IntoIterator<Item = FeedbackTokenRemoveFromHistory<'a>>,
    ) -> Self {
        Self {
            feedback_tokens: feedback_tokens.into_iter().collect(),
        }
    }
}

impl<'a> AddHistoryItemQuery<'a> {
    pub fn new(song_url: SongTrackingUrl<'a>) -> Self {
        Self { song_url }
    }
}

// TODO: Return played and feedback_token component.
impl<A: LoggedIn> Query<A> for GetHistoryQuery {
    type Output = Vec<HistoryPeriod>;
    type Method = PostMethod;
}
impl PostQuery for GetHistoryQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_history"))])
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}

// NOTE: Does not work on brand accounts
impl<A: LoggedIn> Query<A> for RemoveHistoryItemsQuery<'_> {
    type Output = Vec<ApiOutcome>;
    type Method = PostMethod;
}
impl PostQuery for RemoveHistoryItemsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("feedbackTokens".to_string(), json!(self.feedback_tokens))])
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "feedback"
    }
}

impl<A: LoggedIn> Query<A> for AddHistoryItemQuery<'_> {
    type Output = ();
    type Method = GetMethod;
}

impl GetQuery for AddHistoryItemQuery<'_> {
    fn url(&self) -> &str {
        self.song_url.get_raw()
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        // Original implementation by sigma67
        // https://github.com/sigma67/ytmusicapi/blob/a15d90c4f356a530c6b2596277a9d70c0b117a0c/ytmusicapi/mixins/library.py#L310
        let possible_chars: Vec<char> =
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_"
                .chars()
                .collect();
        let random_cpn: String = rand::rng()
            .sample_iter(
                rand::distr::slice::Choose::new(&possible_chars)
                    .expect("Provided a hard-coded non-empty slice"),
            )
            .take(16)
            .collect();
        vec![
            ("ver", "2".into()),
            ("c", "WEB_REMIX".into()),
            ("cpn", random_cpn.into()),
        ]
    }
}
