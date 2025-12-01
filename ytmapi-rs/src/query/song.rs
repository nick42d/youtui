use super::{PostMethod, PostQuery, Query};
use crate::Result;
use crate::auth::AuthToken;
use crate::common::{LyricsID, SongTrackingUrl, VideoID, YoutubeID};
use crate::parse::Lyrics;
use serde_json::json;
use std::borrow::Cow;
use std::time::SystemTime;

pub struct GetLyricsIDQuery<'a> {
    video_id: VideoID<'a>,
}

pub struct GetLyricsQuery<'a> {
    id: LyricsID<'a>,
}

pub struct GetSongTrackingUrlQuery<'a> {
    video_id: VideoID<'a>,
    signature_timestamp: u64,
}

impl<'a> GetLyricsIDQuery<'a> {
    pub fn new(video_id: VideoID<'a>) -> GetLyricsIDQuery<'a> {
        GetLyricsIDQuery { video_id }
    }
}

impl<'a> GetLyricsQuery<'a> {
    pub fn new(id: LyricsID<'a>) -> GetLyricsQuery<'a> {
        GetLyricsQuery { id }
    }
}

impl GetSongTrackingUrlQuery<'_> {
    /// # NOTE
    /// A GetSongTrackingUrlQuery stores a timestamp, it's not recommended
    /// to store these for a long period of time. The constructor can fail
    /// due to a System Time error.
    pub fn new(video_id: VideoID<'_>) -> Result<GetSongTrackingUrlQuery<'_>> {
        let signature_timestamp = get_signature_timestamp()?;
        Ok(GetSongTrackingUrlQuery {
            video_id,
            signature_timestamp,
        })
    }
}

impl<A: AuthToken> Query<A> for GetLyricsIDQuery<'_> {
    type Output = LyricsID<'static>;
    type Method = PostMethod;
}
impl PostQuery for GetLyricsIDQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
            "enablePersistentPlaylistPanel": true,
            "isAudioOnly": true,
            "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
            "playlistId" : format!("RDAMVM{}", self.video_id.get_raw()),
            "videoId" : self.video_id.get_raw(),
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "next"
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
}

impl<A: AuthToken> Query<A> for GetLyricsQuery<'_> {
    type Output = Lyrics;
    type Method = PostMethod;
}
impl PostQuery for GetLyricsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let serde_json::Value::Object(map) = json!({
            "browseId": self.id.get_raw(),
        }) else {
            unreachable!()
        };
        map
    }
    fn path(&self) -> &str {
        "browse"
    }
    fn params(&self) -> std::vec::Vec<(&str, std::borrow::Cow<'_, str>)> {
        vec![]
    }
}

impl<A: AuthToken> Query<A> for GetSongTrackingUrlQuery<'_> {
    type Output = SongTrackingUrl<'static>;
    type Method = PostMethod;
}
impl PostQuery for GetSongTrackingUrlQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([
            (
                "playbackContext".to_string(),
                json!(
                    {
                        "contentPlaybackContext": {
                            "signatureTimestamp": self.signature_timestamp
                        }
                    }
                ),
            ),
            ("video_id".to_string(), json!(self.video_id)),
        ])
    }
    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        vec![]
    }
    fn path(&self) -> &str {
        "player"
    }
}

// Original: https://github.com/sigma67/ytmusicapi/blob/a15d90c4f356a530c6b2596277a9d70c0b117a0c/ytmusicapi/mixins/_utils.py#L42
/// Approximation for google's signatureTimestamp which would normally be
/// extracted from base.js.
fn get_signature_timestamp() -> Result<u64> {
    const SECONDS_IN_DAY: u64 = 60 * 60 * 24;
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        // SAFETY: SECONDS_IN_DAY is nonzero.
        .saturating_div(SECONDS_IN_DAY))
}
