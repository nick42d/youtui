use super::{PostMethod, PostQuery, Query};
use crate::auth::{AuthToken, LoggedIn};
use crate::common::{PlaylistID, SetVideoID, VideoID, YoutubeID};
use crate::parse::{GetPlaylistDetails, PlaylistItem, WatchPlaylistTrack};
pub use additems::*;
pub use create::*;
pub use edit::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::fmt::Display;

pub mod additems;
pub mod create;
pub mod edit;

// Potentially same functionality as similar trait for Create.
pub trait SpecialisedQuery {
    fn additional_header(&self) -> Option<(String, serde_json::Value)>;
}

pub trait GetWatchPlaylistQueryID {
    fn get_video_id(&self) -> Option<Cow<str>>;
    fn get_playlist_id(&self) -> Cow<str>;
}

//TODO: Likely Common
#[derive(Default, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PrivacyStatus {
    Public,
    #[default]
    Private,
    Unlisted,
}
impl Display for PrivacyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = match self {
            PrivacyStatus::Public => "PUBLIC",
            PrivacyStatus::Private => "PRIVATE",
            PrivacyStatus::Unlisted => "UNLISTED",
        };
        write!(f, "{}", str)
    }
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

// Suspect this requires a browseId, not a playlistId - i.e requires VL at the
// start.
pub struct GetPlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

// Suspect this requires a browseId, not a playlistId - i.e requires VL at the
// start.
pub struct GetPlaylistDetailsQuery<'a> {
    id: PlaylistID<'a>,
}

pub struct DeletePlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

pub struct GetWatchPlaylistQuery<T: GetWatchPlaylistQueryID> {
    id: T,
}

pub struct RemovePlaylistItemsQuery<'a> {
    id: PlaylistID<'a>,
    video_items: Vec<SetVideoID<'a>>,
}

impl<'a> GetPlaylistQuery<'a> {
    pub fn new(id: PlaylistID<'a>) -> GetPlaylistQuery<'a> {
        GetPlaylistQuery { id }
    }
}
impl<'a> GetPlaylistDetailsQuery<'a> {
    pub fn new(id: PlaylistID<'a>) -> GetPlaylistDetailsQuery<'a> {
        GetPlaylistDetailsQuery { id }
    }
}
impl<'a> DeletePlaylistQuery<'a> {
    pub fn new(id: PlaylistID<'a>) -> DeletePlaylistQuery<'a> {
        DeletePlaylistQuery { id }
    }
}
impl<'a> From<PlaylistID<'a>> for DeletePlaylistQuery<'a> {
    fn from(value: PlaylistID<'a>) -> Self {
        DeletePlaylistQuery { id: value }
    }
}
impl<'a> RemovePlaylistItemsQuery<'a> {
    pub fn new(
        id: PlaylistID<'a>,
        video_items: impl IntoIterator<Item = SetVideoID<'a>>,
    ) -> RemovePlaylistItemsQuery<'a> {
        RemovePlaylistItemsQuery {
            id,
            video_items: video_items.into_iter().collect(),
        }
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

impl<A: AuthToken> Query<A> for GetPlaylistQuery<'_> {
    type Output = Vec<PlaylistItem>;
    type Method = PostMethod;
}
impl PostQuery for GetPlaylistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if processing required to add 'VL' portion of playlistId
        let serde_json::Value::Object(map) = json!({
            "browseId" : self.id.get_raw(),
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

// Note - this is functionally the same as GetPlaylistQuery, however the output
// is different.
impl<A: AuthToken> Query<A> for GetPlaylistDetailsQuery<'_> {
    type Output = GetPlaylistDetails;
    type Method = PostMethod;
}
impl PostQuery for GetPlaylistDetailsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if processing required to add 'VL' portion of playlistId
        let serde_json::Value::Object(map) = json!({
            "browseId" : self.id.get_raw(),
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

impl<A: LoggedIn> Query<A> for DeletePlaylistQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for DeletePlaylistQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if processing required to remove 'VL' portion of playlistId
        let serde_json::Value::Object(map) = json!({
            "playlistId" : self.id.get_raw(),
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "playlist/delete"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

impl<A: LoggedIn> Query<A> for RemovePlaylistItemsQuery<'_> {
    type Output = ();
    type Method = PostMethod;
}
impl PostQuery for RemovePlaylistItemsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(mut map) = json!({
            "playlistId": self.id,
        }) else {
            unreachable!()
        };
        let actions: Vec<serde_json::Value> = self
            .video_items
            .iter()
            .map(|v| {
                json!(
                {
                    "setVideoId" : v,
                    "action" : "ACTION_REMOVE_VIDEO",
                })
            })
            .collect();
        map.insert("actions".into(), json!(actions));
        map
    }
    fn path(&self) -> &str {
        "browse/edit_playlist"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

impl<T: GetWatchPlaylistQueryID, A: AuthToken> Query<A> for GetWatchPlaylistQuery<T> {
    type Output = Vec<crate::parse::WatchPlaylistTrack>;
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
