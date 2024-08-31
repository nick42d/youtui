use super::{PostMethod, PostQuery, Query};
use crate::{
    auth::AuthToken,
    common::{PodcastChannelParams, PodcastID, VideoID},
};
use serde_json::json;

pub struct GetChannelQuery {
    channel_id: (),
}
pub struct GetChannelEpisodesQuery<'a> {
    channel_id: (),
    podcast_channel_params: PodcastChannelParams<'a>,
}
pub struct GetPodcastQuery<'a> {
    podcast_id: PodcastID<'a>,
}
pub struct GetEpisodeQuery<'a> {
    video_id: VideoID<'a>,
}
pub struct GetEpisodesPlaylistQuery;

impl GetChannelQuery {
    pub fn new(channel_id: impl Into<()>) -> Self {
        Self {
            channel_id: channel_id.into(),
        }
    }
}
impl<'a> GetChannelEpisodesQuery<'a> {
    pub fn new(
        channel_id: impl Into<()>,
        podcast_channel_params: impl Into<PodcastChannelParams<'a>>,
    ) -> GetChannelEpisodesQuery<'a> {
        GetChannelEpisodesQuery {
            channel_id: channel_id.into(),
            podcast_channel_params: podcast_channel_params.into(),
        }
    }
}
impl<'a> GetPodcastQuery<'a> {
    pub fn new(podcast_id: impl Into<PodcastID<'a>>) -> Self {
        Self {
            podcast_id: podcast_id.into(),
        }
    }
}
impl<'a> GetEpisodeQuery<'a> {
    pub fn new(video_id: impl Into<VideoID<'a>>) -> Self {
        Self {
            video_id: video_id.into(),
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
impl<'a, A: AuthToken> Query<A> for GetPodcastQuery<'a> {
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
        FromIterator::from_iter([("browseId".into(), json!(self.channel_id))])
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
        FromIterator::from_iter([
            ("browseId".into(), json!(self.channel_id)),
            ("params".into(), json!(self.podcast_channel_params)),
        ])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// TODO: Continuations
impl<'a> PostQuery for GetPodcastQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if any parsing required
        FromIterator::from_iter([("browseId".into(), json!(self.podcast_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
impl<'a> PostQuery for GetEpisodeQuery<'a> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if any parsing required
        FromIterator::from_iter([("browseId".into(), json!(self.video_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Gets all episodes in a playlist of episodes.
// The only playlist like this seems to be the New Episodes auto-playlist, so
// it's possible that this is not worth implementing.
impl<'a> PostQuery for GetEpisodesPlaylistQuery<'a> {
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
