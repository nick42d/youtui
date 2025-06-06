use super::PrivacyStatus;
use crate::auth::AuthToken;
use crate::common::{PlaylistID, VideoID};
use crate::query::{PostMethod, PostQuery, Query};
use serde_json::json;
use std::borrow::Cow;

pub trait CreatePlaylistType {
    fn additional_header(&self) -> Option<(String, serde_json::Value)>;
}

/// A playlist can be created using a list of video ids, or as a copy of an
/// existing playlist (but not both at the same time).
#[derive(Debug, Clone, PartialEq)]
pub struct CreatePlaylistQuery<'a, C: CreatePlaylistType> {
    title: Cow<'a, str>,
    description: Option<Cow<'a, str>>,
    privacy_status: PrivacyStatus,
    query_type: C,
}

/// Helper struct for CreatePlaylistQuery
#[derive(Default, Debug, Clone, PartialEq)]
pub struct BasicCreatePlaylist {}
/// Helper struct for CreatePlaylistQuery
#[derive(Default, Debug, Clone, PartialEq)]
pub struct CreatePlaylistFromVideos<'a> {
    video_ids: Vec<VideoID<'a>>,
}
/// Helper struct for CreatePlaylistQuery
#[derive(Debug, Clone, PartialEq)]
pub struct CreatePlaylistFromPlaylist<'a> {
    source_playlist: PlaylistID<'a>,
}

impl CreatePlaylistType for BasicCreatePlaylist {
    fn additional_header(&self) -> Option<(String, serde_json::Value)> {
        None
    }
}
impl CreatePlaylistType for CreatePlaylistFromVideos<'_> {
    fn additional_header(&self) -> Option<(String, serde_json::Value)> {
        Some(("videoIds".into(), json!(self.video_ids)))
    }
}
impl CreatePlaylistType for CreatePlaylistFromPlaylist<'_> {
    fn additional_header(&self) -> Option<(String, serde_json::Value)> {
        Some(("sourcePlaylistId".into(), json!(self.source_playlist)))
    }
}

impl<'a> CreatePlaylistQuery<'a, BasicCreatePlaylist> {
    pub fn new(
        title: &'a str,
        description: Option<&'a str>,
        privacy_status: PrivacyStatus,
    ) -> CreatePlaylistQuery<'a, BasicCreatePlaylist> {
        CreatePlaylistQuery {
            title: title.into(),
            description: description.map(|d| d.into()),
            privacy_status,
            query_type: BasicCreatePlaylist {},
        }
    }
}

impl<'a> CreatePlaylistQuery<'a, BasicCreatePlaylist> {
    pub fn with_source<T: Into<PlaylistID<'a>>>(
        self,
        source_playlist: T,
    ) -> CreatePlaylistQuery<'a, CreatePlaylistFromPlaylist<'a>> {
        let CreatePlaylistQuery {
            title,
            description,
            privacy_status,
            ..
        } = self;
        CreatePlaylistQuery {
            title,
            description,
            privacy_status,
            query_type: CreatePlaylistFromPlaylist {
                source_playlist: source_playlist.into(),
            },
        }
    }
}

impl<'a> CreatePlaylistQuery<'a, BasicCreatePlaylist> {
    pub fn with_video_ids(
        self,
        video_ids: Vec<VideoID<'a>>,
    ) -> CreatePlaylistQuery<'a, CreatePlaylistFromVideos<'a>> {
        let CreatePlaylistQuery {
            title,
            description,
            privacy_status,
            ..
        } = self;
        CreatePlaylistQuery {
            title,
            description,
            privacy_status,
            query_type: CreatePlaylistFromVideos { video_ids },
        }
    }
}

impl<A: AuthToken, C: CreatePlaylistType> Query<A> for CreatePlaylistQuery<'_, C> {
    type Output = PlaylistID<'static>;
    type Method = PostMethod;
}
impl<C: CreatePlaylistType> PostQuery for CreatePlaylistQuery<'_, C> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        // TODO: Confirm if processing required to remove 'VL' portion of playlistId
        let serde_json::Value::Object(mut map) = json!({
            "title" : self.title,
            "privacyStatus" : self.privacy_status.to_string(),
        }) else {
            unreachable!()
        };
        if let Some(description) = &self.description {
            // TODO: Process description to ensure it doesn't contain html. Google doesn't
            // allow html.
            // https://github.com/sigma67/ytmusicapi/blob/main/ytmusicapi/mixins/playlists.py#L311
            map.insert("description".to_string(), description.as_ref().into());
        }
        if let Some(additional_header) = self.query_type.additional_header() {
            map.insert(additional_header.0, additional_header.1);
        }
        map
    }
    fn path(&self) -> &str {
        "playlist/create"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}
