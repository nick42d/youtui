use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{PlaylistID, VideoID, YoutubeID};
use serde_json::json;
use std::borrow::Cow;

pub trait GetWatchPlaylistQueryID {
    fn get_video_id(&self) -> Option<Cow<str>>;
    fn get_playlist_id(&self) -> Cow<str>;
}

pub struct GetWatchPlaylistQuery<T: GetWatchPlaylistQueryID> {
    id: T,
}
pub struct VideoAndPlaylistID<'a> {
    video_id: VideoID<'a>,
    playlist_id: PlaylistID<'a>,
}

impl GetWatchPlaylistQueryID for VideoAndPlaylistID<'_> {
    fn get_video_id(&self) -> Option<Cow<str>> {
        Some(self.video_id.get_raw().into())
    }

    fn get_playlist_id(&self) -> Cow<str> {
        self.playlist_id.get_raw().into()
    }
}
impl GetWatchPlaylistQueryID for VideoID<'_> {
    fn get_video_id(&self) -> Option<Cow<str>> {
        Some(self.get_raw().into())
    }

    fn get_playlist_id(&self) -> Cow<str> {
        format!("RDAMVM{}", self.get_raw()).into()
    }
}
impl GetWatchPlaylistQueryID for PlaylistID<'_> {
    fn get_video_id(&self) -> Option<Cow<str>> {
        None
    }
    fn get_playlist_id(&self) -> Cow<str> {
        self.get_raw().into()
    }
}

impl<T: GetWatchPlaylistQueryID, A: AuthToken> Query<A> for GetWatchPlaylistQuery<T> {
    type Output = crate::parse::WatchPlaylist;
    type Method = PostMethod;
}
impl<T: GetWatchPlaylistQueryID> PostQuery for GetWatchPlaylistQuery<T> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(mut map) = json!({
            "enablePersistentPlaylistPanel": true,
            "isAudioOnly": true,
            "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
            "playlistId" : self.id.get_playlist_id(),
        }) else {
            unreachable!()
        };
        if let Some(video_id) = self.id.get_video_id() {
            map.insert("videoId".to_string(), json!(video_id));
        };
        map
    }
    fn path(&self) -> &str {
        "next"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}
impl<'a> GetWatchPlaylistQuery<VideoID<'a>> {
    pub fn new_from_video_id<T: Into<VideoID<'a>>>(id: T) -> GetWatchPlaylistQuery<VideoID<'a>> {
        GetWatchPlaylistQuery { id: id.into() }
    }
    pub fn with_playlist_id(
        self,
        playlist_id: PlaylistID<'a>,
    ) -> GetWatchPlaylistQuery<VideoAndPlaylistID<'a>> {
        GetWatchPlaylistQuery {
            id: VideoAndPlaylistID {
                video_id: self.id,
                playlist_id,
            },
        }
    }
}
impl<'a> GetWatchPlaylistQuery<PlaylistID<'a>> {
    pub fn new_from_playlist_id(id: PlaylistID<'a>) -> GetWatchPlaylistQuery<PlaylistID<'a>> {
        GetWatchPlaylistQuery { id }
    }
    pub fn with_video_id(
        self,
        video_id: VideoID<'a>,
    ) -> GetWatchPlaylistQuery<VideoAndPlaylistID<'a>> {
        GetWatchPlaylistQuery {
            id: VideoAndPlaylistID {
                video_id,
                playlist_id: self.id,
            },
        }
    }
}
