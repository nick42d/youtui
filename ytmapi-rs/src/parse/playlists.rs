use std::sync::Arc;

use super::{
    parse_playlist_items, ParseFrom, PlaylistItem, ProcessedResult, DESCRIPTION_SHELF_RUNS,
    HEADER_DETAIL, STRAPLINE_TEXT, STRAPLINE_THUMBNAIL, SUBTITLE2, SUBTITLE3, THUMBNAIL_CROPPED,
    TITLE_TEXT, TWO_COLUMN,
};
use crate::{
    common::{ApiOutcome, PlaylistID, SetVideoID},
    nav_consts::{
        RESPONSIVE_HEADER, SECOND_SUBTITLE_RUNS, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB, TAB_CONTENT,
    },
    query::{
        AddPlaylistItemsQuery, CreatePlaylistQuery, CreatePlaylistType, DeletePlaylistQuery,
        EditPlaylistQuery, GetPlaylistQuery, PrivacyStatus, RemovePlaylistItemsQuery,
        SpecialisedQuery,
    },
    Error, Result, Thumbnail, VideoID,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};
use ytmapi_rs_json_crawler::{JsonCrawler, JsonCrawlerGeneral, JsonCrawlerIterator};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetPlaylist {
    pub id: PlaylistID<'static>,
    // NOTE: Only present on personal (library) playlists??
    // NOTE: May not be present on old version of API also.
    pub privacy: Option<PrivacyStatus>,
    pub title: String,
    pub description: Option<String>,
    pub author: String,
    pub year: String,
    pub duration: String,
    pub track_count_text: String,
    // NOTE: Seem to be unable to distinguish when views is optional.
    pub views: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    /// Not yet implemented
    pub suggestions: Vec<()>,
    /// Not yet implemented
    pub related: Vec<()>,
    pub tracks: Vec<PlaylistItem>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
/// Provides a SetVideoID and VideoID for each video added to the playlist.
pub struct AddPlaylistItem {
    pub video_id: VideoID<'static>,
    pub set_video_id: SetVideoID<'static>,
}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for () {
    fn parse_from(_: ProcessedResult<RemovePlaylistItemsQuery<'a>>) -> crate::Result<Self> {
        Ok(())
    }
}
impl<'a, C: CreatePlaylistType> ParseFrom<CreatePlaylistQuery<'a, C>> for PlaylistID<'static> {
    fn parse_from(p: ProcessedResult<CreatePlaylistQuery<'a, C>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawler = p.into();
        json_crawler
            .take_value_pointer("/playlistId")
            .map_err(Into::into)
    }
}
impl<'a, T: SpecialisedQuery> ParseFrom<AddPlaylistItemsQuery<'a, T>> for Vec<AddPlaylistItem> {
    fn parse_from(p: ProcessedResult<AddPlaylistItemsQuery<'a, T>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawler = p.into();
        let status: ApiOutcome = json_crawler.borrow_pointer("/status")?.take_value()?;
        if let ApiOutcome::Failure = status {
            return Err(Error::status_failed());
        }
        json_crawler
            .navigate_pointer("/playlistEditResults")?
            .try_iter_mut()?
            .map(|r| {
                let mut r = r.navigate_pointer("/playlistEditVideoAddedResultData")?;
                Ok(AddPlaylistItem {
                    video_id: r.take_value_pointer("/videoId")?,
                    set_video_id: r.take_value_pointer("/setVideoId")?,
                })
            })
            .collect()
    }
}
impl<'a> ParseFrom<EditPlaylistQuery<'a>> for ApiOutcome {
    fn parse_from(p: ProcessedResult<EditPlaylistQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawler = p.into();
        json_crawler
            .navigate_pointer("/status")?
            .take_value()
            .map_err(Into::into)
    }
}
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for () {
    fn parse_from(_: ProcessedResult<DeletePlaylistQuery<'a>>) -> crate::Result<Self> {
        Ok(())
    }
}

impl<'a> ParseFrom<GetPlaylistQuery<'a>> for GetPlaylist {
    fn parse_from(p: ProcessedResult<GetPlaylistQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawler = p.into();
        if json_crawler.path_exists("/header") {
            get_playlist(json_crawler)
        } else {
            get_playlist_2024(json_crawler)
        }
    }
}

fn get_playlist(mut json_crawler: JsonCrawler) -> Result<GetPlaylist> {
    let mut header = json_crawler.borrow_pointer(HEADER_DETAIL)?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let privacy = None;
    // TODO
    let suggestions = Vec::new();
    // TODO
    let related = Vec::new();
    // TODO
    let description = None;
    let author = header.take_value_pointer(SUBTITLE2)?;
    let year = header.take_value_pointer(SUBTITLE3)?;
    let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
    let mut second_subtitle_runs = header.navigate_pointer(SECOND_SUBTITLE_RUNS)?;
    let duration = second_subtitle_runs
        .as_array_iter_mut()?
        .try_last()?
        .take_value_pointer("/text")?;
    let track_count_text = second_subtitle_runs
        .borrow_mut()
        .into_array_iter_mut()?
        .rev()
        .nth(2)
        .map(|mut run| run.take_value_pointer("/text"))
        .ok_or_else(|| {
            Error::array_size(
                second_subtitle_runs.get_path(),
                // TODO: Remove allocation.
                Arc::new(second_subtitle_runs.get_source().to_owned()),
                3,
            )
        })??;
    let views = second_subtitle_runs
        .as_array_iter_mut()?
        .rev()
        .nth(4)
        .map(|mut item| item.take_value_pointer("/text"))
        .transpose()?;
    let mut results = json_crawler.borrow_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        "/musicPlaylistShelfRenderer"
    ))?;
    let id = results.take_value_pointer("/playlistId")?;
    let music_shelf = results.navigate_pointer("/contents")?;
    let tracks = parse_playlist_items(music_shelf)?;
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

// NOTE: Similar code to get_album_2024
fn get_playlist_2024(json_crawler: JsonCrawler) -> Result<GetPlaylist> {
    let mut columns = json_crawler.navigate_pointer(TWO_COLUMN)?;
    let header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER));
    // TODO: Utilise a crawler library function here.
    let mut header = match header {
        Ok(header) => header,
        Err(_) => columns.borrow_pointer(concatcp!(
            TAB_CONTENT,
            SECTION_LIST_ITEM,
            "/musicEditablePlaylistDetailHeaderRenderer/header",
            RESPONSIVE_HEADER
        ))?,
    };
    // TODO
    let suggestions = Vec::new();
    // TODO
    let related = Vec::new();
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let author = header.take_value_pointer(STRAPLINE_TEXT)?;
    let thumbnails: Vec<Thumbnail> = header.take_value_pointer(STRAPLINE_THUMBNAIL)?;
    let description = header
        .borrow_pointer(DESCRIPTION_SHELF_RUNS)
        .and_then(|d| d.try_into_iter())
        .ok()
        .map(|r| {
            r.map(|mut r| r.take_value_pointer::<String>("/text"))
                .collect::<std::result::Result<String, _>>()
        })
        .transpose()?;
    let mut subtitle = header.borrow_pointer("/subtitle/runs")?;
    let subtitle_len = subtitle.try_iter_mut()?.len();
    let privacy = if subtitle_len == 5 {
        Some(subtitle.take_value_pointer("/2/text")?)
    } else {
        None
    };
    let year = subtitle.take_value_pointer(format!("/{}/text", subtitle_len.saturating_sub(1)))?;
    let mut second_subtitle_runs = header.borrow_pointer(SECOND_SUBTITLE_RUNS)?;
    let duration = second_subtitle_runs
        .as_array_iter_mut()?
        .try_last()?
        .take_value_pointer("/text")?;
    let track_count_text = second_subtitle_runs
        .borrow_mut()
        .into_array_iter_mut()?
        .rev()
        .nth(2)
        .map(|mut run| run.take_value_pointer("/text"))
        .ok_or_else(|| {
            Error::array_size(
                second_subtitle_runs.get_path(),
                // TODO: Remove allocation.
                Arc::new(second_subtitle_runs.get_source().to_owned()),
                3,
            )
        })??;
    let views = second_subtitle_runs
        .as_array_iter_mut()?
        .rev()
        .nth(4)
        .map(|mut item| item.take_value_pointer("/text"))
        .transpose()?;
    let id = header
        .navigate_pointer("/buttons")?
        .try_into_iter()?
        .find_path("/musicPlayButtonRenderer")?
        .take_value_pointer("/playNavigationEndpoint/watchEndpoint/playlistId")?;
    let music_shelf = columns.borrow_pointer(
        "/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents",
    )?;
    let tracks = parse_playlist_items(music_shelf)?;
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

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{ApiOutcome, PlaylistID, YoutubeID},
        process_json,
        query::{AddPlaylistItemsQuery, EditPlaylistQuery, GetPlaylistQuery},
        Error,
    };
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[tokio::test]
    async fn test_get_playlist_query() {
        parse_test!(
            "./test_json/get_playlist_20240617.json",
            "./test_json/get_playlist_20240617_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_add_playlist_items_query_failure() {
        let source_path = Path::new("./test_json/add_playlist_items_failure_20240626.json");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        // Blank query has no bearing on function
        let query = AddPlaylistItemsQuery::new_from_playlist(
            PlaylistID::from_raw(""),
            PlaylistID::from_raw(""),
        );
        let output = process_json::<_, BrowserToken>(source, query);
        let err: crate::Result<()> = Err(Error::status_failed());
        assert_eq!(format!("{:?}", err), format!("{:?}", output));
    }
    #[tokio::test]
    async fn test_add_playlist_items_query() {
        parse_test!(
            "./test_json/add_playlist_items_20240626.json",
            "./test_json/add_playlist_items_20240626_output.txt",
            AddPlaylistItemsQuery::new_from_playlist(
                PlaylistID::from_raw(""),
                PlaylistID::from_raw(""),
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_edit_playlist_title_query() {
        parse_test_value!(
            "./test_json/edit_playlist_title_20240626.json",
            ApiOutcome::Success,
            EditPlaylistQuery::new_title(PlaylistID::from_raw(""), ""),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_playlist_query_2024() {
        parse_test!(
            "./test_json/get_playlist_20240624.json",
            "./test_json/get_playlist_20240624_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
}
