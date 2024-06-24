use super::{
    ParseFrom, ProcessedResult, HEADER_DETAIL, PLAY_BUTTON, SINGLE_COLUMN, SUBTITLE2, SUBTITLE3,
    THUMBNAIL, THUMBNAILS, THUMBNAIL_CROPPED, THUMBNAIL_RENDERER, TITLE_TEXT,
};
use crate::{
    common::PlaylistID,
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::{
        MRLIR, RUN_TEXT, SECOND_SUBTITLE_RUNS, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB, TEXT_RUN_TEXT,
        WATCH_VIDEO_ID,
    },
    process::{process_fixed_column_item, process_flex_column_item},
    query::{
        AddPlaylistItemsQuery, DeletePlaylistQuery, EditPlaylistQuery, GetPlaylistQuery,
        RemovePlaylistItemsQuery,
    },
    Error, Thumbnail, VideoID,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetPlaylist {
    id: PlaylistID<'static>,
    // NOTE: Only present on personal (library) playlists??
    // NOTE: May not be present on old version of API also.
    privacy: Option<PlaylistPrivacy>,
    title: String,
    description: String,
    author: String,
    year: String,
    duration: String,
    track_count_text: String,
    views: String,
    thumbnails: Vec<Thumbnail>,
    suggestions: Vec<()>,
    related: Vec<()>,
    tracks: Vec<GetPlaylistSong>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PlaylistPrivacy {
    Public,
}

// NOTE: Likely a duplicate of another common struct.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetPlaylistSong {
    title: String,
    video_id: VideoID<'static>,
    duration: String,
    artist: String,
    thumbnails: Vec<Thumbnail>,
    // Track may not have an album - e.g a video.
    album: Option<String>,
}

impl TryFrom<&str> for PlaylistPrivacy {
    type Error = crate::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Public" => Ok(PlaylistPrivacy::Public),
            other => Err(Error::other(format!(
                "Error parsing PlaylistPrivacy from value {other}"
            ))),
        }
    }
}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<RemovePlaylistItemsQuery<'a>>,
    ) -> crate::Result<<RemovePlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<AddPlaylistItemsQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<AddPlaylistItemsQuery<'a>>,
    ) -> crate::Result<<AddPlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<EditPlaylistQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<EditPlaylistQuery<'a>>,
    ) -> crate::Result<<EditPlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<DeletePlaylistQuery<'a>>,
    ) -> crate::Result<<DeletePlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}

impl<'a> ParseFrom<GetPlaylistQuery<'a>> for GetPlaylist {
    fn parse_from(
        p: ProcessedResult<GetPlaylistQuery<'a>>,
    ) -> crate::Result<<GetPlaylistQuery<'a> as crate::query::Query>::Output> {
        let mut json_crawler: JsonCrawler = p.into();
        let mut header = json_crawler.borrow_pointer(HEADER_DETAIL)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let privacy = None;
        // TODO
        let suggestions = Vec::new();
        // TODO
        let related = Vec::new();
        // TODO
        let description = String::new();
        let author = header.take_value_pointer(SUBTITLE2)?;
        let year = header.take_value_pointer(SUBTITLE3)?;
        let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
        let views = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/0/text"))?;
        let track_count_text =
            header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/2/text"))?;
        let duration = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/4/text"))?;

        let mut results = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            "/musicPlaylistShelfRenderer"
        ))?;
        let id = results.take_value_pointer("/playlistId")?;
        let tracks = results
            .navigate_pointer("/contents")?
            .as_array_iter_mut()?
            .map(|c| c.navigate_pointer(MRLIR).and_then(|c| get_playlist_song(c)))
            .collect::<crate::Result<Vec<GetPlaylistSong>>>()?;

        Ok(GetPlaylist {
            id,
            privacy,
            title,
            description,
            author,
            year,
            duration,
            track_count_text,
            thumbnails,
            suggestions,
            related,
            views,
            tracks,
        })
    }
}

fn get_playlist_song(mut j: JsonCrawlerBorrowed) -> crate::Result<GetPlaylistSong> {
    let title = process_flex_column_item(&mut j, 0)
        .and_then(|mut i| i.take_value_pointer(TEXT_RUN_TEXT))?;
    let artist = process_flex_column_item(&mut j, 1)
        .and_then(|mut i| i.take_value_pointer(TEXT_RUN_TEXT))?;
    let album = process_flex_column_item(&mut j, 1)
        .and_then(|mut i| i.take_value_pointer(TEXT_RUN_TEXT))
        .ok();
    let video_id = j.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let duration = j.take_value_pointer(concatcp!(
        "/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer",
        TEXT_RUN_TEXT
    ))?;
    let thumbnails = j.take_value_pointer(THUMBNAILS)?;
    return Ok(GetPlaylistSong {
        title,
        video_id,
        duration,
        artist,
        thumbnails,
        album,
    });
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{PlaylistID, YoutubeID},
        query::GetPlaylistQuery,
        YtMusic,
    };
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[tokio::test]
    async fn test_get_playlist_query() {
        let source_path = Path::new("./test_json/get_playlist_20240617.json");
        let expected_path = Path::new("./test_json/get_playlist_20240617_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        // Blank query has no bearing on function
        let query = GetPlaylistQuery::new(PlaylistID::from_raw(""));
        let output = YtMusic::<BrowserToken>::process_json(source, query).unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
