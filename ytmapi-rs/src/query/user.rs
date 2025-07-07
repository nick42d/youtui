use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{ArtistChannelID, UserChannelID, UserPlaylistsParams, UserVideosParams};
use crate::parse::{GetUser, UserPlaylist, UserVideo};
use serde_json::json;

pub struct GetUserQuery<'a> {
    user_channel_id: UserChannelID<'a>,
}
pub struct GetUserPlaylistsQuery<'a> {
    user_channel_id: UserChannelID<'a>,
    params: UserPlaylistsParams<'a>,
}
pub struct GetUserVideosQuery<'a> {
    user_channel_id: UserChannelID<'a>,
    params: UserVideosParams<'a>,
}

impl<'a> GetUserQuery<'a> {
    pub fn new(user_channel_id: UserChannelID<'a>) -> Self {
        Self { user_channel_id }
    }
}
impl<'a> GetUserPlaylistsQuery<'a> {
    pub fn new(user_channel_id: UserChannelID<'a>, params: UserPlaylistsParams<'a>) -> Self {
        Self {
            user_channel_id,
            params,
        }
    }
}
impl<'a> GetUserVideosQuery<'a> {
    pub fn new(user_channel_id: UserChannelID<'a>, params: UserVideosParams<'a>) -> Self {
        Self {
            user_channel_id,
            params,
        }
    }
}

impl<A: AuthToken> Query<A> for GetUserQuery<'_> {
    type Output = GetUser;
    type Method = PostMethod;
}
impl PostQuery for GetUserQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("browseId".to_string(), json!(self.user_channel_id))])
    }
    fn params(&self) -> Vec<(&str, std::borrow::Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<A: AuthToken> Query<A> for GetUserPlaylistsQuery<'_> {
    type Output = Vec<UserPlaylist>;
    type Method = PostMethod;
}
impl PostQuery for GetUserPlaylistsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([
            ("browseId".to_string(), json!(self.user_channel_id)),
            ("params".to_string(), json!(self.params)),
        ])
    }
    fn params(&self) -> Vec<(&str, std::borrow::Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<A: AuthToken> Query<A> for GetUserVideosQuery<'_> {
    type Output = Vec<UserVideo>;
    type Method = PostMethod;
}
impl PostQuery for GetUserVideosQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([
            ("browseId".to_string(), json!(self.user_channel_id)),
            ("params".to_string(), json!(self.params)),
        ])
    }
    fn params(&self) -> Vec<(&str, std::borrow::Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "browse"
    }
}
