use crate::auth::AuthToken;

use super::{PostMethod, PostQuery, Query};

pub struct GetChannelQuery;
pub struct GetChannelEpisodesQuery;
pub struct GetPodcastQuery;
pub struct GetEpisodeQuery;
pub struct GetEpisodesPlaylistQuery;

impl<A: AuthToken> Query<A> for GetChannelQuery {
    type Output = ();
    type Method = PostMethod;
}
impl<A: AuthToken> Query<A> for GetChannelEpisodesQuery {
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
    }

    fn params(&self) -> Option<std::borrow::Cow<str>> {
        todo!()
    }

    fn path(&self) -> &str {
        todo!()
    }
}
impl PostQuery for GetChannelEpisodesQuery {
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
