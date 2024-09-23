use super::{PostMethod, PostQuery, Query};
use crate::{
    auth::AuthToken,
    common::{ArtistChannelID, BrowseParams, YoutubeID},
    parse::{ArtistParams, GetArtistAlbumsAlbum},
};
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

impl<'a, T: Into<ArtistChannelID<'a>>> From<T> for GetArtistQuery<'a> {
    fn from(channel_id: T) -> Self {
        GetArtistQuery::new(channel_id.into())
    }
}

impl<'a, A: AuthToken> Query<A> for GetArtistQuery<'a> {
    type Output = ArtistParams;
    type Method = PostMethod;
}
impl<'a> PostQuery for GetArtistQuery<'a> {
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
impl<'a, A: AuthToken> Query<A> for GetArtistAlbumsQuery<'a> {
    type Output = Vec<GetArtistAlbumsAlbum>;
    type Method = PostMethod;
}
impl<'a> PostQuery for GetArtistAlbumsQuery<'a> {
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
