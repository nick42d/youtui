//! Module to contain YouTube enumerated values for internal use only.

use serde::{Deserialize, Serialize};

// watchPlaylistEndpoint params within overlay.
// To distinguish between Community and Featured playlists for playlist search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum PlaylistEndpointParams {
    #[serde(rename = "wAEB")]
    Featured,
    #[serde(rename = "wAEB8gECKAE%3D")]
    Community,
}

/// Currently used to distinguish between Podcasts and Playlists for Community
/// playlists in basic search, but may be able to be generalised further.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum YoutubeMusicPageType {
    #[serde(rename = "MUSIC_PAGE_TYPE_PODCAST_SHOW_DETAIL_PAGE")]
    Podcast,
    #[serde(rename = "MUSIC_PAGE_TYPE_PLAYLIST")]
    Playlist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum YoutubeMusicVideoType {
    // I believe OMV is 'Official Music Video' and UGC is 'User Generated Content'
    #[serde(rename = "MUSIC_VIDEO_TYPE_UGC")]
    Ugc,
    #[serde(rename = "MUSIC_VIDEO_TYPE_OMV")]
    Omv,
    // Could be 'Audio Track Video'? Seems to represent a standard song.
    #[serde(rename = "MUSIC_VIDEO_TYPE_ATV")]
    Atv,
    #[serde(rename = "MUSIC_VIDEO_TYPE_PODCAST_EPISODE")]
    Episode,
    #[serde(rename = "MUSIC_VIDEO_TYPE_PRIVATELY_OWNED_TRACK")]
    Upload,
}
