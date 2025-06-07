use super::SpecialisedQuery;
use crate::auth::AuthToken;
use crate::common::{PlaylistID, VideoID};
use crate::parse::AddPlaylistItem;
use crate::query::{PostMethod, PostQuery, Query};
use serde_json::json;
use std::borrow::Cow;

#[derive(Default, Debug, Clone, PartialEq)]
pub enum DuplicateHandlingMode {
    #[default]
    ReturnError,
    Unhandled,
}

// XXX: Query type potentially does not need to be mutually exclusive.
pub struct AddPlaylistItemsQuery<'a, T: SpecialisedQuery> {
    id: PlaylistID<'a>,
    query_type: T,
}
/// Helper struct for AddPlaylistItemsQuery
#[derive(Default, Debug, Clone, PartialEq)]
pub struct AddVideosToPlaylist<'a> {
    video_ids: Vec<VideoID<'a>>,
    duplicate_handling_mode: DuplicateHandlingMode,
}
/// Helper struct for AddPlaylistItemsQuery
#[derive(Debug, Clone, PartialEq)]
pub struct AddPlaylistToPlaylist<'a> {
    source_playlist: PlaylistID<'a>,
}
impl SpecialisedQuery for AddVideosToPlaylist<'_> {
    fn additional_header(&self) -> Option<(String, serde_json::Value)> {
        let actions = self
            .video_ids
            .iter()
            .map(|v| match self.duplicate_handling_mode {
                DuplicateHandlingMode::ReturnError => json!({
                    "action" : "ACTION_ADD_VIDEO",
                    "addedVideoId" : v,
                }),
                DuplicateHandlingMode::Unhandled => json!({
                    "action" : "ACTION_ADD_VIDEO",
                    "addedVideoId" : v,
                    "dedupeOption" : "DEDUPE_OPTION_SKIP",
                }),
            });
        Some(("actions".to_string(), actions.collect()))
    }
}
impl SpecialisedQuery for AddPlaylistToPlaylist<'_> {
    fn additional_header(&self) -> Option<(String, serde_json::Value)> {
        Some((
            "actions".to_string(),
            json!([{
                "action" : "ACTION_ADD_PLAYLIST",
                "addedFullListId" : self.source_playlist,
            },
            {
                "action" : "ACTION_ADD_VIDEO",
                "addedVideoId" : null,
            }]),
        ))
    }
}
impl<'a> AddPlaylistItemsQuery<'a, AddPlaylistToPlaylist<'a>> {
    pub fn new_from_playlist(id: PlaylistID<'a>, source_playlist: PlaylistID<'a>) -> Self {
        Self {
            id,
            query_type: AddPlaylistToPlaylist { source_playlist },
        }
    }
}
impl<'a> AddPlaylistItemsQuery<'a, AddVideosToPlaylist<'a>> {
    pub fn new_from_videos(
        id: PlaylistID<'a>,
        video_ids: impl IntoIterator<Item = VideoID<'a>>,
        duplicate_handling_mode: DuplicateHandlingMode,
    ) -> Self {
        Self {
            id,
            query_type: AddVideosToPlaylist {
                video_ids: video_ids.into_iter().collect(),
                duplicate_handling_mode,
            },
        }
    }
}

impl<A: AuthToken, T: SpecialisedQuery> Query<A> for AddPlaylistItemsQuery<'_, T> {
    type Output = Vec<AddPlaylistItem>;
    type Method = PostMethod;
}
impl<T: SpecialisedQuery> PostQuery for AddPlaylistItemsQuery<'_, T> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(mut map) = json!({
            "playlistId" : self.id,
        }) else {
            unreachable!()
        };
        if let Some(additional_header) = self.query_type.additional_header() {
            map.insert(additional_header.0, additional_header.1);
        }
        map
    }
    fn path(&self) -> &str {
        "browse/edit_playlist"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}
