use super::{PostMethod, PostQuery, Query};
use crate::auth::LoggedIn;
use crate::common::{LikeStatus, PlaylistID, VideoID, YoutubeID};
use serde_json::json;
use std::borrow::Cow;

pub struct RateSongQuery<'a> {
    video_id: VideoID<'a>,
    rating: LikeStatus,
}
impl<'a> RateSongQuery<'a> {
    pub fn new(video_id: VideoID<'a>, rating: LikeStatus) -> Self {
        Self { video_id, rating }
    }
}
pub struct RatePlaylistQuery<'a> {
    playlist_id: PlaylistID<'a>,
    rating: LikeStatus,
}
impl<'a> RatePlaylistQuery<'a> {
    pub fn new(playlist_id: PlaylistID<'a>, rating: LikeStatus) -> Self {
        Self {
            playlist_id,
            rating,
        }
    }
}

impl<A: LoggedIn> Query<A> for RateSongQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for RateSongQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "target".to_string(),
            json!({"videoId" : self.video_id.get_raw()} ),
        )])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        like_endpoint(&self.rating)
    }
}

impl<A: LoggedIn> Query<A> for RatePlaylistQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}

impl PostQuery for RatePlaylistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([(
            "target".to_string(),
            json!({"playlistId" : self.playlist_id.get_raw()} ),
        )])
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        like_endpoint(&self.rating)
    }
}

fn like_endpoint(rating: &LikeStatus) -> &'static str {
    match *rating {
        LikeStatus::Liked => "like/like",
        LikeStatus::Disliked => "like/dislike",
        LikeStatus::Indifferent => "like/removelike",
    }
}
