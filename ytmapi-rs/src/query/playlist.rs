use super::{PostMethod, PostQuery, Query};
use crate::auth::{AuthToken, LoggedIn};
use crate::common::{PlaylistID, SetVideoID, YoutubeID};
use crate::parse::GetPlaylist;
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

// Suspect this requires a browseId, not a playlistId - i.e requires VL at the
// start.
pub struct GetPlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

pub struct DeletePlaylistQuery<'a> {
    id: PlaylistID<'a>,
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
impl<'a> DeletePlaylistQuery<'a> {
    pub fn new(id: PlaylistID<'a>) -> DeletePlaylistQuery<'a> {
        DeletePlaylistQuery { id }
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

impl<A: AuthToken> Query<A> for GetPlaylistQuery<'_> {
    type Output = GetPlaylist;
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
impl<'a> From<PlaylistID<'a>> for DeletePlaylistQuery<'a> {
    fn from(value: PlaylistID<'a>) -> Self {
        DeletePlaylistQuery { id: value }
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
