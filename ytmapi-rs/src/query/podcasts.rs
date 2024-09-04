use super::{PostMethod, PostQuery, Query};
use crate::{
    auth::AuthToken,
    common::{EpisodeID, PodcastChannelID, PodcastChannelParams, PodcastID, VideoID},
    parse::{Episode, GetEpisode, GetPodcast, GetPodcastChannel},
};
use serde_json::json;

pub struct GetChannelQuery<'a> {
    channel_id: PodcastChannelID<'a>,
}
pub struct GetChannelEpisodesQuery<'a> {
    channel_id: PodcastChannelID<'a>,
    podcast_channel_params: PodcastChannelParams<'a>,
}
pub struct GetPodcastQuery<'a> {
    podcast_id: PodcastID<'a>,
}
pub struct GetEpisodeQuery<'a> {
    episode_id: EpisodeID<'a>,
}
pub struct GetNewEpisodesQuery;

// NOTE: This is technically the same page as the GetArtist page. It's possible
// this could be generalised.
impl<'a> GetChannelQuery<'a> {
    pub fn new(channel_id: impl Into<PodcastChannelID<'a>>) -> Self {
        Self {
            channel_id: channel_id.into(),
        }
    }
}
impl<'a> GetChannelEpisodesQuery<'a> {
    pub fn new(
        channel_id: impl Into<PodcastChannelID<'a>>,
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
    pub fn new(episode_id: impl Into<EpisodeID<'a>>) -> Self {
        Self {
            episode_id: episode_id.into(),
        }
    }
}

impl<'a, A: AuthToken> Query<A> for GetChannelQuery<'a> {
    type Output = GetPodcastChannel;
    type Method = PostMethod;
}
impl<'a, A: AuthToken> Query<A> for GetChannelEpisodesQuery<'a> {
    type Output = Vec<Episode>;
    type Method = PostMethod;
}
impl<'a, A: AuthToken> Query<A> for GetPodcastQuery<'a> {
    type Output = GetPodcast;
    type Method = PostMethod;
}
impl<'a, A: AuthToken> Query<A> for GetEpisodeQuery<'a> {
    type Output = GetEpisode;
    type Method = PostMethod;
}
impl<A: AuthToken> Query<A> for GetNewEpisodesQuery {
    type Output = Vec<Episode>;
    type Method = PostMethod;
}

impl<'a> PostQuery for GetChannelQuery<'a> {
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
        FromIterator::from_iter([("browseId".into(), json!(self.episode_id))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
// Gets the NewEpisodes auto-playlist. In future there could be other similar
// playlists, we can instead re-implement this as GetEpisodesPlaylist.
impl PostQuery for GetNewEpisodesQuery {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        FromIterator::from_iter([("browseId".into(), json!("VLRDPN"))])
    }
    fn params(&self) -> Option<std::borrow::Cow<str>> {
        None
    }
    fn path(&self) -> &str {
        "browse"
    }
}
