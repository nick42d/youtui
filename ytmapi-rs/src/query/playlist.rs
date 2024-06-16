use super::Query;
use crate::{
    common::{PlaylistID, YoutubeID},
    VideoID,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;

//TODO: Likely Common
#[derive(Default, Clone)]
pub enum PrivacyStatus {
    Public,
    #[default]
    Private,
    Unlisted,
}

//TODO: Likely Common
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SetVideoID<'a>(Cow<'a, str>);

pub enum AddOrder {
    AddToTop,
    AddToBottom,
}

pub struct GetPlaylistQuery<'a> {
    id: PlaylistID<'a>,
}

pub struct CreatePlaylistQuery<'a> {
    title: Cow<'a, str>,
    description: Cow<'a, str>,
    privacy_status: PrivacyStatus,
    video_ids: Vec<VideoID<'a>>,
    source_playlist: Option<PlaylistID<'a>>,
}

// Is this really a query?
pub struct EditPlaylistQuery<'a> {
    playlist_id: PlaylistID<'a>,
    new_title: Option<Cow<'a, str>>,
    new_description: Option<Cow<'a, str>>,
    new_privacy_status: Option<PrivacyStatus>,
    swap_videos_order: Option<(SetVideoID<'a>, SetVideoID<'a>)>,
    change_add_order: Option<AddOrder>,
    add_playlist: Option<PlaylistID<'a>>,
}

impl<'a> Query for GetPlaylistQuery<'a> {
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
impl<'a> GetPlaylistQuery<'a> {
    pub fn new(id: PlaylistID<'a>) -> GetPlaylistQuery<'a> {
        GetPlaylistQuery { id }
    }
}
