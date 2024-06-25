use super::Query;
use crate::{
    common::{PlaylistID, SetVideoID, YoutubeID},
    parse::{ApiSuccess, GetPlaylist},
    Error, Result, VideoID,
};
pub use create::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;

pub mod create;

//TODO: Likely Common
#[derive(Default, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PrivacyStatus {
    Public,
    #[default]
    Private,
    Unlisted,
}
impl TryFrom<&str> for PrivacyStatus {
    type Error = crate::Error;
    fn try_from(value: &str) -> Result<Self> {
        match value {
            "Public" => Ok(PrivacyStatus::Public),
            "Private" => Ok(PrivacyStatus::Private),
            "Unlisted" => Ok(PrivacyStatus::Unlisted),
            other => Err(Error::other(format!(
                "Error parsing PlaylistPrivacy from value {other}"
            ))),
        }
    }
}
impl ToString for PrivacyStatus {
    fn to_string(&self) -> String {
        match self {
            PrivacyStatus::Public => "PUBLIC",
            PrivacyStatus::Private => "PRIVATE",
            PrivacyStatus::Unlisted => "UNLISTED",
        }
        .to_string()
    }
}

pub enum AddOrder {
    AddToTop,
    AddToBottom,
}

pub enum DuplicateHandlingMode {
    ReturnError,
    Unhandled,
}

pub struct GetPlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

// Is this really a query? It's more of an action/command.
// TODO: Confirm if all options can be passed - or mutually exclusive.
// XXX: Private until completed
pub(crate) struct EditPlaylistQuery<'a> {
    id: PlaylistID<'a>,
    new_title: Option<Cow<'a, str>>,
    new_description: Option<Cow<'a, str>>,
    new_privacy_status: Option<PrivacyStatus>,
    swap_videos_order: Option<(SetVideoID<'a>, SetVideoID<'a>)>,
    change_add_order: Option<AddOrder>,
    add_playlist: Option<PlaylistID<'a>>,
}

pub struct DeletePlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

// XXX: Private until completed
pub(crate) struct AddPlaylistItemsQuery<'a> {
    id: PlaylistID<'a>,
    video_ids: Vec<VideoID<'a>>,
    source_playlist: Option<PlaylistID<'a>>,
    // NOTE: Duplicate handling mode ReturnError is mutually exclusive with
    // source_playlist.is_some()
    duplicate_handling_mode: DuplicateHandlingMode,
}

// XXX: Private until completed
pub(crate) struct RemovePlaylistItemsQuery<'a> {
    id: PlaylistID<'a>,
    // TODO: Should be a Track returned by get_playlist - as it requires both a VideoID and
    // SetVideoID
    video_items: Vec<(VideoID<'a>, SetVideoID<'a>)>,
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
        video_items: Vec<(VideoID<'a>, SetVideoID<'a>)>,
    ) -> RemovePlaylistItemsQuery<'a> {
        RemovePlaylistItemsQuery { id, video_items }
    }
}

impl<'a> Query for GetPlaylistQuery<'a> {
    type Output = GetPlaylist;
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
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}

impl<'a> Query for DeletePlaylistQuery<'a> {
    // TODO
    type Output = ApiSuccess;
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
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}
impl<'a> Into<DeletePlaylistQuery<'a>> for PlaylistID<'a> {
    fn into(self) -> DeletePlaylistQuery<'a> {
        DeletePlaylistQuery { id: self }
    }
}
impl<'a> Query for EditPlaylistQuery<'a> {
    // TODO
    type Output = ();
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!();
        // let actions = Vec::new();
        // TODO: Confirm if VL needs to be stripped / added from playlistId
        // let serde_json::Value::Object(map) = json!({
        //     "playlistId" : self.id.get_raw(),
        //     "actions" : actions,
        // }) else {
        //     unreachable!()
        // };
        // map
    }
    fn path(&self) -> &str {
        "browse/edit_playlist"
    }
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}

impl<'a> Query for RemovePlaylistItemsQuery<'a> {
    // TODO
    type Output = ();
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!();
        // let serde_json::Value::Object(map) = json!({
        //     "enablePersistentPlaylistPanel": true,
        //     "isAudioOnly": true,
        //     "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
        //     "videoId" : self.id.get_raw(),
        //     "playlistId" : format!("RDAMVM{}",self.id.get_raw()),
        // }) else {
        //     unreachable!()
        // };
        // map
    }
    fn path(&self) -> &str {
        todo!();
        // "next"
    }
    fn params(&self) -> Option<Cow<str>> {
        todo!();
        // None
    }
}

impl<'a> Query for AddPlaylistItemsQuery<'a> {
    //TODO
    type Output = ();
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        todo!();
        // let serde_json::Value::Object(map) = json!({
        //     "enablePersistentPlaylistPanel": true,
        //     "isAudioOnly": true,
        //     "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
        //     "videoId" : self.id.get_raw(),
        //     "playlistId" : format!("RDAMVM{}",self.id.get_raw()),
        // }) else {
        //     unreachable!()
        // };
        // map
    }
    fn path(&self) -> &str {
        todo!();
        // "next"
    }
    fn params(&self) -> Option<Cow<str>> {
        todo!();
        // None
    }
}
