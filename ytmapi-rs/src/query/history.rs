use super::Query;
use crate::{
    auth::AuthToken,
    common::{ApiOutcome, FeedbackTokenRemoveFromHistory},
    parse::TableListItem,
};
use serde_json::json;

pub struct GetHistoryQuery;
pub struct RemoveHistoryItemsQuery<'a> {
    feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>,
}

impl<'a> RemoveHistoryItemsQuery<'a> {
    pub fn new(feedback_tokens: Vec<FeedbackTokenRemoveFromHistory<'a>>) -> Self {
        Self { feedback_tokens }
    }
}

// NOTE: Requires auth
// TODO: Return played and feedback_token component.
impl<A: AuthToken> Query<A> for GetHistoryQuery {
    type Output = Vec<TableListItem>
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("browseId".to_string(), json!("FEmusic_history"))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}

// NOTE: Does not work on brand accounts
impl<'a, A: AuthToken> Query<A> for RemoveHistoryItemsQuery<'a> {
    type Output = Vec<ApiOutcome>
    where
        Self: Sized;

    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([("feedbackTokens".to_string(), json!(self.feedback_tokens))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "feedback"
    }
}
