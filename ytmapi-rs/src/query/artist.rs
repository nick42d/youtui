use super::{PostMethod, PostQuery, Query};
use crate::auth::{AuthToken, LoggedIn};
use crate::common::{ArtistChannelID, BrowseParams, YoutubeID};
use crate::parse::{GetArtist, GetArtistAlbumsAlbum};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct GetArtistQuery<'a> {
    channel_id: ArtistChannelID<'a>,
}
#[derive(Debug, Clone)]
pub struct GetArtistAlbumsQuery<'a> {
    channel_id: ArtistChannelID<'a>,
    params: BrowseParams<'a>,
}
#[derive(Debug, Clone)]
pub struct SubscribeArtistQuery<'a> {
    channel_id: ArtistChannelID<'a>,
}
#[derive(Debug, Clone)]
pub struct UnsubscribeArtistsQuery<'a> {
    channel_ids: Vec<ArtistChannelID<'a>>,
}
impl<'a> GetArtistQuery<'a> {
    pub fn new(channel_id: impl Into<ArtistChannelID<'a>>) -> GetArtistQuery<'a> {
        GetArtistQuery {
            channel_id: channel_id.into(),
        }
    }
}
impl<'a> GetArtistAlbumsQuery<'a> {
    pub fn new(
        channel_id: ArtistChannelID<'a>,
        params: BrowseParams<'a>,
    ) -> GetArtistAlbumsQuery<'a> {
        GetArtistAlbumsQuery { channel_id, params }
    }
}
impl<'a> SubscribeArtistQuery<'a> {
    pub fn new(channel_id: ArtistChannelID<'a>) -> SubscribeArtistQuery<'a> {
        SubscribeArtistQuery { channel_id }
    }
}
impl<'a> UnsubscribeArtistsQuery<'a> {
    pub fn new(
        channel_ids: impl IntoIterator<Item = ArtistChannelID<'a>>,
    ) -> UnsubscribeArtistsQuery<'a> {
        UnsubscribeArtistsQuery {
            channel_ids: channel_ids.into_iter().collect(),
        }
    }
}

impl<'a, T: Into<ArtistChannelID<'a>>> From<T> for GetArtistQuery<'a> {
    fn from(channel_id: T) -> Self {
        GetArtistQuery::new(channel_id.into())
    }
}

impl<A: AuthToken> Query<A> for GetArtistQuery<'_> {
    type Output = GetArtist;
    type Method = PostMethod;
}
impl PostQuery for GetArtistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // XXX: could do in new to avoid process every time called
        // or even better, could do this first time called, and store state so not
        // required after that.
        let value = self.channel_id.get_raw().replacen("MPLA", "", 1);
        let serde_json::Value::Object(map) = json!({
            "browseId" : value,
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}
// TODO: Check if the MPLA strip is correct for both of these.
impl<A: AuthToken> Query<A> for GetArtistAlbumsQuery<'_> {
    type Output = Vec<GetArtistAlbumsAlbum>;
    type Method = PostMethod;
}
impl PostQuery for GetArtistAlbumsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // XXX: should do in new
        // XXX: Think I could remove allocation here
        let value = self.channel_id.get_raw().replacen("MPLA", "", 1);
        FromIterator::from_iter([
            ("browseId".to_string(), json!(value)),
            ("params".to_string(), json!(self.params)),
        ])
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}
impl<A: LoggedIn> Query<A> for SubscribeArtistQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for SubscribeArtistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("channelIds".into(), json!([self.channel_id]))])
    }
    fn path(&self) -> &str {
        "subscription/subscribe"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}
impl<A: LoggedIn> Query<A> for UnsubscribeArtistsQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for UnsubscribeArtistsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("channelIds".into(), json!(self.channel_ids))])
    }
    fn path(&self) -> &str {
        "subscription/unsubscribe"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}
