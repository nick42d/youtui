use super::{
    parse_playlist_items, ParseFrom, PlaylistItem, ProcessedResult, DESCRIPTION_SHELF_RUNS,
    HEADER_DETAIL, STRAPLINE_TEXT, SUBTITLE2, SUBTITLE3, THUMBNAIL_CROPPED, TITLE_TEXT, TWO_COLUMN,
};
use crate::common::{ApiOutcome, LyricsID, PlaylistID, SetVideoID, Thumbnail, VideoID};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    APPEND_CONTINUATION_ITEMS, CONTENT, CONTINUATION_PARAMS, FACEPILE_AVATAR_URL, FACEPILE_TEXT,
    MUSIC_PLAYLIST_SHELF, MUSIC_QUEUE_PLAYLIST_PANEL, NAVIGATION_PLAYLIST_ID,
    RADIO_CONTINUATION_PARAMS, RESPONSIVE_HEADER, SECONDARY_SECTION_LIST_ITEM,
    SECONDARY_SECTION_LIST_RENDERER, SECOND_SUBTITLE_RUNS, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB,
    TAB_CONTENT, THUMBNAILS, WATCH_NEXT_CONTENT,
};
use crate::query::playlist::{
    CreatePlaylistType, GetPlaylistDetailsQuery, GetWatchPlaylistQueryID, PrivacyStatus,
    SpecialisedQuery,
};
use crate::query::{
    AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery, EditPlaylistQuery,
    GetPlaylistQuery, GetWatchPlaylistQuery, RemovePlaylistItemsQuery,
};
use crate::{Error, Result};
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPlaylistDetails {
    pub id: PlaylistID<'static>,
    // NOTE: Only present on personal (library) playlists??
    // NOTE: May not be present on old version of API also.
    pub privacy: Option<PrivacyStatus>,
    pub title: String,
    pub description: Option<String>,
    pub author: String,
    pub author_avatar_url: Option<String>,
    pub year: String,
    pub duration: String,
    pub track_count_text: String,
    // NOTE: Seem to be unable to distinguish when views is optional.
    pub views: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
/// Provides a SetVideoID and VideoID for each video added to the playlist.
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct AddPlaylistItem {
    pub video_id: VideoID<'static>,
    pub set_video_id: SetVideoID<'static>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WatchPlaylistTrack {}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for () {
    fn parse_from(_: ProcessedResult<RemovePlaylistItemsQuery<'a>>) -> crate::Result<Self> {
        Ok(())
    }
}
impl<'a, C: CreatePlaylistType> ParseFrom<CreatePlaylistQuery<'a, C>> for PlaylistID<'static> {
    fn parse_from(p: ProcessedResult<CreatePlaylistQuery<'a, C>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
        json_crawler
            .take_value_pointer("/playlistId")
            .map_err(Into::into)
    }
}
impl<'a, T: SpecialisedQuery> ParseFrom<AddPlaylistItemsQuery<'a, T>> for Vec<AddPlaylistItem> {
    fn parse_from(p: ProcessedResult<AddPlaylistItemsQuery<'a, T>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
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
        let json_crawler: JsonCrawlerOwned = p.into();
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

impl<'a> ParseFrom<GetPlaylistDetailsQuery<'a>> for GetPlaylistDetails {
    fn parse_from(p: ProcessedResult<GetPlaylistDetailsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        if json_crawler.path_exists("/header") {
            get_playlist(json_crawler)
        } else {
            get_playlist_2024(json_crawler)
        }
    }
}

impl<'a> ParseFromContinuable<GetPlaylistQuery<'a>> for Vec<PlaylistItem> {
    fn parse_from_continuable(
        p: ProcessedResult<GetPlaylistQuery<'a>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_playlist_shelf = json_crawler.navigate_pointer(concatcp!(
            TWO_COLUMN,
            SECONDARY_SECTION_LIST_RENDERER,
            CONTENT,
            MUSIC_PLAYLIST_SHELF,
            "/contents"
        ))?;
        parse_playlist_items(music_playlist_shelf)
    }
    fn parse_continuation(
        p: ProcessedResult<crate::query::GetContinuationsQuery<'_, GetPlaylistQuery<'a>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let continuation_items = json_crawler.navigate_pointer(APPEND_CONTINUATION_ITEMS)?;
        parse_playlist_items(continuation_items)
    }
}

impl<T: GetWatchPlaylistQueryID> ParseFromContinuable<GetWatchPlaylistQuery<T>>
    for Vec<WatchPlaylistTrack>
{
    fn parse_from_continuable(
        p: ProcessedResult<GetWatchPlaylistQuery<T>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let playlist_panel = json_crawler
            .navigate_pointer(concatcp!(WATCH_NEXT_CONTENT, MUSIC_QUEUE_PLAYLIST_PANEL))?;
        let continuation_params = playlist_panel
            .take_value_pointer(RADIO_CONTINUATION_PARAMS)
            .ok();
        Ok((vec![], continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<crate::query::GetContinuationsQuery<'_, GetWatchPlaylistQuery<T>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let playlist_panel = json_crawler
            .navigate_pointer(concatcp!(WATCH_NEXT_CONTENT, MUSIC_QUEUE_PLAYLIST_PANEL))?;
        let continuation_params = playlist_panel
            .take_value_pointer(RADIO_CONTINUATION_PARAMS)
            .ok();
        Ok((vec![], continuation_params))
    }
}

fn get_playlist(mut json_crawler: JsonCrawlerOwned) -> Result<GetPlaylistDetails> {
    let mut header = json_crawler.borrow_pointer(HEADER_DETAIL)?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let privacy = None;
    let description = None;
    let author = header.take_value_pointer(SUBTITLE2)?;
    let year = header.take_value_pointer(SUBTITLE3)?;
    let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
    let mut second_subtitle_runs = header.navigate_pointer(SECOND_SUBTITLE_RUNS)?;
    let duration = second_subtitle_runs
        .try_iter_mut()?
        .try_last()?
        .take_value_pointer("/text")?;
    let track_count_text = second_subtitle_runs.try_expect(
        "second subtitle runs should count at least 3 runs",
        |second_subtitle_runs| {
            second_subtitle_runs
                .try_iter_mut()?
                .rev()
                .nth(2)
                .map(|mut run| run.take_value_pointer("/text"))
                .transpose()
        },
    )?;
    let views = second_subtitle_runs
        .try_iter_mut()?
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
    Ok(GetPlaylistDetails {
        id,
        privacy,
        title,
        description,
        author,
        year,
        duration,
        track_count_text,
        thumbnails,
        views,
        author_avatar_url: None,
    })
}

// NOTE: Similar code to get_album_2024
fn get_playlist_2024(json_crawler: JsonCrawlerOwned) -> Result<GetPlaylistDetails> {
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
    let title = header.take_value_pointer(TITLE_TEXT)?;
    // STRAPLINE_TEXT to be deprecated in future.
    let author = header.take_value_pointers(&[STRAPLINE_TEXT, FACEPILE_TEXT])?;
    let thumbnails: Vec<Thumbnail> = header.take_value_pointer(THUMBNAILS)?;
    let author_avatar_url: Option<String> = header.take_value_pointer(FACEPILE_AVATAR_URL).ok();
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
        .try_iter_mut()?
        .try_last()?
        .take_value_pointer("/text")?;
    let track_count_text = second_subtitle_runs.try_expect(
        "second subtitle runs should count at least 3 runs",
        |second_subtitle_runs| {
            second_subtitle_runs
                .try_iter_mut()?
                .rev()
                .nth(2)
                .map(|mut run| run.take_value_pointer("/text"))
                .transpose()
        },
    )?;
    let views = second_subtitle_runs
        .try_iter_mut()?
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
    Ok(GetPlaylistDetails {
        id,
        privacy,
        title,
        description,
        author,
        year,
        duration,
        track_count_text,
        thumbnails,
        views,
        author_avatar_url,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{ApiOutcome, PlaylistID, YoutubeID};
    use crate::query::{AddPlaylistItemsQuery, EditPlaylistQuery, GetPlaylistQuery};
    use crate::{process_json, Error};
    use pretty_assertions::assert_eq;
    use std::path::Path;

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
    #[deprecated]
    async fn test_get_playlist_query_old() {
        parse_test!(
            "./test_json/get_playlist_20240617.json",
            "./test_json/get_playlist_20240617_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    #[deprecated]
    async fn test_get_playlist_query_2024() {
        parse_test!(
            "./test_json/get_playlist_20240624.json",
            "./test_json/get_playlist_20240624_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    #[deprecated]
    // In 2025, playlist channel details were moved from strapline to facepile.
    async fn test_get_playlist_query_2025() {
        parse_test!(
            "./test_json/get_playlist_20250604.json",
            "./test_json/get_playlist_20250604_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[deprecated]
    #[tokio::test]
    async fn test_get_playlist_query_2024_no_channel_thumbnail() {
        parse_test!(
            "./test_json/get_playlist_no_channel_thumbnail_20240818.json",
            "./test_json/get_playlist_no_channel_thumbnail_20240818_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_playlist_query() {
        parse_with_matching_continuation_test!(
            "./test_json/get_playlist_X.json",
            "./test_json/get_playlist_continuation_X.json",
            "./test_json/get_playlist_X_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_watch_playlist_query() {
        parse_with_matching_continuation_test!(
            "./test_json/get_watch_playlist_20250630.json",
            "./test_json/get_watch_playlist_continuation_20250630.json",
            "./test_json/get_watch_playlist_20250630_output.txt",
            GetPlaylistQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
}
