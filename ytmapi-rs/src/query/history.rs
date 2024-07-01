use super::Query;
use crate::common::FeedbackToken;
use serde_json::json;

pub struct GetHistoryQuery {}
pub struct AddHistoryItemQuery {}
pub struct RemoveHistoryItemsQuery<'a> {
    feedback_tokens: Vec<FeedbackToken<'a>>,
}

// NOTE: Requires auth
impl Query for GetHistoryQuery {
    type Output = ()
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

// NOTE: Sends a GET request, not POST.
impl Query for AddHistoryItemQuery {
    type Output = ()
    where
        Self: Sized;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!()
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        todo!()
    }
    fn path(&self) -> &str {
        todo!()
    }
}

impl<'a> Query for RemoveHistoryItemsQuery<'a> {
    type Output = ()
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
