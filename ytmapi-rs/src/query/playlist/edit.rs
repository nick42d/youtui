use super::PrivacyStatus;
use crate::{
    auth::AuthToken,
    common::{ApiOutcome, PlaylistID, SetVideoID, YoutubeID},
    query::Query,
};
use serde_json::json;
use std::borrow::Cow;

// TODO: Confirm if all options can be passed - or mutually exclusive.
pub struct EditPlaylistQuery<'a> {
    id: PlaylistID<'a>,
    new_title: Option<Cow<'a, str>>,
    new_description: Option<Cow<'a, str>>,
    new_privacy_status: Option<PrivacyStatus>,
    swap_videos_order: Option<(SetVideoID<'a>, SetVideoID<'a>)>,
    change_add_order: Option<AddOrder>,
    add_playlist: Option<PlaylistID<'a>>,
}

#[derive(Default)]
pub enum AddOrder {
    AddToTop,
    #[default]
    AddToBottom,
}

impl<'a> EditPlaylistQuery<'a> {
    pub fn new_title<T: Into<PlaylistID<'a>>, S: Into<Cow<'a, str>>>(id: T, new_title: S) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: Some(new_title.into()),
            new_description: None,
            new_privacy_status: None,
            swap_videos_order: None,
            change_add_order: None,
            add_playlist: None,
        }
    }
    pub fn new_description<T: Into<PlaylistID<'a>>, S: Into<Cow<'a, str>>>(
        id: T,
        new_description: S,
    ) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: None,
            new_description: Some(new_description.into()),
            new_privacy_status: None,
            swap_videos_order: None,
            change_add_order: None,
            add_playlist: None,
        }
    }
    pub fn new_privacy_status<T: Into<PlaylistID<'a>>>(
        id: T,
        new_privacy_status: PrivacyStatus,
    ) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: None,
            new_privacy_status: Some(new_privacy_status),
            new_description: None,
            swap_videos_order: None,
            change_add_order: None,
            add_playlist: None,
        }
    }
    pub fn swap_videos_order<T: Into<PlaylistID<'a>>>(
        id: T,
        video_1: SetVideoID<'a>,
        video_2: SetVideoID<'a>,
    ) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: None,
            swap_videos_order: Some((video_1, video_2)),
            new_privacy_status: None,
            new_description: None,
            change_add_order: None,
            add_playlist: None,
        }
    }
    pub fn change_add_order<T: Into<PlaylistID<'a>>>(id: T, change_add_order: AddOrder) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: None,
            change_add_order: Some(change_add_order),
            new_privacy_status: None,
            swap_videos_order: None,
            new_description: None,
            add_playlist: None,
        }
    }
    pub fn add_playlist<T: Into<PlaylistID<'a>>>(id: T, add_playlist: PlaylistID<'a>) -> Self {
        let id = id.into();
        Self {
            id,
            new_title: None,
            add_playlist: Some(add_playlist),
            new_privacy_status: None,
            swap_videos_order: None,
            change_add_order: None,
            new_description: None,
        }
    }
    pub fn with_new_description<S: Into<Cow<'a, str>>>(mut self, new_description: S) -> Self {
        self.new_description = Some(new_description.into());
        self
    }
    pub fn with_new_privacy_status(mut self, new_privacy_status: PrivacyStatus) -> Self {
        self.new_privacy_status = Some(new_privacy_status);
        self
    }
    pub fn with_change_add_order(mut self, change_add_order: AddOrder) -> Self {
        self.change_add_order = Some(change_add_order);
        self
    }
    pub fn with_add_playlist(mut self, add_playlist: PlaylistID<'a>) -> Self {
        self.add_playlist = Some(add_playlist);
        self
    }
    pub fn with_swap_videos_order(
        mut self,
        first_video: SetVideoID<'a>,
        second_video: SetVideoID<'a>,
    ) -> Self {
        self.swap_videos_order = Some((first_video, second_video));
        self
    }
}

impl<'a, A: AuthToken> Query<A> for EditPlaylistQuery<'a> {
    type Output = ApiOutcome;
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut actions = Vec::new();
        if let Some(new_title) = &self.new_title {
            actions.push(json!({
                "action" : "ACTION_SET_PLAYLIST_NAME",
                "playlistName" : new_title
            }))
        };
        if let Some(new_description) = &self.new_description {
            actions.push(json!({
                "action" : "ACTION_SET_PLAYLIST_DESCRIPTION",
                "playlistDescription" : new_description
            }))
        };
        if let Some(new_privacy_status) = &self.new_privacy_status {
            actions.push(json!({
                "action" : "ACTION_SET_PLAYLIST_PRIVACY",
                "playlistPrivacy" : new_privacy_status
            }))
        };
        if let Some((video_1, video_2)) = &self.swap_videos_order {
            actions.push(json!({
                "action" : "ACTION_MOVE_VIDEO_BEFORE",
                "setVideoId" : video_1,
                "movedSetVideoIdSuccessor" : video_2
            }))
        };
        if let Some(add_playlist) = &self.add_playlist {
            actions.push(json!({
                "action" : "ACTION_ADD_PLAYLIST",
                "addedFullListId" : add_playlist
            }))
        };
        if let Some(change_add_order) = &self.change_add_order {
            let add_to_top = match change_add_order {
                AddOrder::AddToTop => true,
                AddOrder::AddToBottom => false,
            };
            actions.push(json!({
                "action" : "ACTION_SET_ADD_TO_TOP",
                "addToTop" : add_to_top
            }))
        };
        if let Some(new_privacy_status) = &self.new_privacy_status {
            actions.push(json!({
                "action" : "ACTION_SET_PLAYLIST_PRIVACY",
                "playlistPrivacy" : new_privacy_status
            }))
        };
        // TODO: Confirm if VL needs to be stripped / added from playlistId
        // Confirmed!
        let serde_json::Value::Object(map) = json!({
            "playlistId" : self.id.get_raw(),
            "actions" : actions,
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "browse/edit_playlist"
    }
    fn params(&self) -> Option<Cow<str>> {
        None
    }
}
