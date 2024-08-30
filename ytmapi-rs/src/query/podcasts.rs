use serde_json::json;
use super::{PostMethod, PostQuery, Query};
use crate::{auth::AuthToken, common::PodcastChannelParams, };

pub struct GetChannelQuery;
pub struct GetChannelEpisodesQuery<'a> {
    channel_id: (),
    podcast_channel_params: PodcastChannelParams<'a>,
}
pub struct GetPodcastQuery;
pub struct GetEpisodeQuery;
pub struct GetEpisodesPlaylistQuery;

impl<'a> GetChannelEpisodesQuery<'a> {
    pub fn new(channel_id: impl Into<()>, podcast_channel_params: impl Into<PodcastChannelParams<'a>>) -> GetChannelEpisodesQuery<'a> {
        GetChannelEpisodesQuery {
            channel_id: channel_id.into(),
            podcast_channel_params: podcast_channel_params.into()
        }
    }
}

impl<A: AuthToken> Query<A> for GetChannelQuery {
    type Output = ();
    type Method = PostMethod;
}
impl<'a, A: AuthToken> Query<A> for GetChannelEpisodesQuery<'a> {
    type Output = ();
    type Method = PostMethod;
}
impl<A: AuthToken> Query<A> for GetPodcastQuery {
    type Output = ();
    type Method = PostMethod;
}
impl<A: AuthToken> Query<A> for GetEpisodeQuery {
    type Output = ();
    type Method = PostMethod;
}
impl<A: AuthToken> Query<A> for GetEpisodesPlaylistQuery {
    type Output = ();
    type Method = PostMethod;
}

impl PostQuery for GetChannelQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!()
        FromIterator::from_iter([("browseId".into(), json!(""))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<'a> PostQuery for GetChannelEpisodesQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("browseId".into(), json!(self.channel_id)), ("params".into(), json!(self.podcast_channel_params))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        todo!()
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl PostQuery for GetPodcastQuery {
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
impl PostQuery for GetEpisodeQuery {
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
impl PostQuery for GetEpisodesPlaylistQuery {
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
