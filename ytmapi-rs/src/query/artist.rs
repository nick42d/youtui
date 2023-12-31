use serde_json::json;

use super::Query;
use crate::common::{BrowseParams, ChannelID, YoutubeID};
use std::borrow::Cow;

pub struct GetArtistQuery<'a> {
    channel_id: ChannelID<'a>,
}
// TODO make params no longer public.
#[derive(Debug)]
pub struct GetArtistAlbumsQuery<'a> {
    channel_id: ChannelID<'a>,
    pub params: BrowseParams<'a>,
}
impl<'a> GetArtistQuery<'a> {
    pub fn new(channel_id: ChannelID<'a>) -> GetArtistQuery<'a> {
        GetArtistQuery { channel_id }
    }
}
impl<'a> GetArtistAlbumsQuery<'a> {
    pub fn new(channel_id: ChannelID<'a>, params: BrowseParams<'a>) -> GetArtistAlbumsQuery<'a> {
        GetArtistAlbumsQuery { channel_id, params }
    }
}

impl<'a> Query for GetArtistQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // XXX: could do in new to avoid process every time called
        // or even better, could do this first time called, and store state so not required
        // after that.
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
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}
// TODO: Check if the MPLA strip is correct for both of these.
impl<'a> Query for GetArtistAlbumsQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // XXX: should do in new
        // XXX: Think I could remove allocation here
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
    fn params(&self) -> Option<Cow<str>> {
        Some(self.params.get_raw().into())
    }
}
