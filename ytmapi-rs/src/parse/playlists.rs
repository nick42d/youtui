use super::{
    parse_playlist_items, MusicShelfContents, ParseFrom, ProcessedResult, SongResult,
    DESCRIPTION_SHELF_RUNS, HEADER_DETAIL, STRAPLINE_TEXT, STRAPLINE_THUMBNAIL, SUBTITLE2,
    SUBTITLE3, THUMBNAIL_CROPPED, TITLE_TEXT, TWO_COLUMN,
};
use crate::{
    common::PlaylistID,
    crawler::JsonCrawler,
    nav_consts::{
        RESPONSIVE_HEADER, SECOND_SUBTITLE_RUNS, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB, TAB_CONTENT,
    },
    query::{
        AddPlaylistItemsQuery, CreatePlaylistQuery, CreatePlaylistType, DeletePlaylistQuery,
        EditPlaylistQuery, GetPlaylistQuery, PrivacyStatus, RemovePlaylistItemsQuery,
    },
    Result, Thumbnail,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetPlaylist {
    id: PlaylistID<'static>,
    // NOTE: Only present on personal (library) playlists??
    // NOTE: May not be present on old version of API also.
    privacy: Option<PrivacyStatus>,
    title: String,
    // NOTE: May not be present at all on playlists - to confirm.
    description: Option<String>,
    author: String,
    year: String,
    duration: String,
    track_count_text: String,
    views: String,
    thumbnails: Vec<Thumbnail>,
    suggestions: Vec<()>,
    related: Vec<()>,
    tracks: Vec<SongResult>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
/// Indicates a successful result from an API action such as a 'delete playlist'
pub struct ApiSuccess {}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for ApiSuccess {
    fn parse_from(
        p: ProcessedResult<RemovePlaylistItemsQuery<'a>>,
    ) -> crate::Result<<RemovePlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        Ok(ApiSuccess {})
    }
}
impl<'a, C: CreatePlaylistType> ParseFrom<CreatePlaylistQuery<'a, C>> for PlaylistID<'static> {
    fn parse_from(
        p: ProcessedResult<CreatePlaylistQuery<'a, C>>,
    ) -> crate::Result<<CreatePlaylistQuery<'a, C> as crate::query::Query>::Output> {
        let mut json_crawler: JsonCrawler = p.into();
        json_crawler.take_value_pointer("/playlistId")
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
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for ApiSuccess {
    fn parse_from(
        p: ProcessedResult<DeletePlaylistQuery<'a>>,
    ) -> crate::Result<<DeletePlaylistQuery<'a> as crate::query::Query>::Output> {
        Ok(ApiSuccess {})
    }
}

impl<'a> ParseFrom<GetPlaylistQuery<'a>> for GetPlaylist {
    fn parse_from(
        p: ProcessedResult<GetPlaylistQuery<'a>>,
    ) -> crate::Result<<GetPlaylistQuery<'a> as crate::query::Query>::Output> {
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
    let views = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/0/text"))?;
    let track_count_text = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/2/text"))?;
    let duration = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/4/text"))?;

    let mut results = json_crawler.borrow_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        "/musicPlaylistShelfRenderer"
    ))?;
    let id = results.take_value_pointer("/playlistId")?;
    let music_shelf = MusicShelfContents {
        json: results.navigate_pointer("/contents")?,
    };
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
    let mut header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER))?;
    // TODO
    let suggestions = Vec::new();
    // TODO
    let related = Vec::new();
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let author = header.take_value_pointer(STRAPLINE_TEXT)?;
    let thumbnails: Vec<Thumbnail> = header.take_value_pointer(STRAPLINE_THUMBNAIL)?;
    let description = header
        .borrow_pointer(DESCRIPTION_SHELF_RUNS)
        .and_then(|d| d.into_array_iter_mut())
        .ok()
        .map(|r| {
            r.map(|mut r| r.take_value_pointer::<String, &str>("/text"))
                .collect::<Result<String>>()
        })
        .transpose()?;
    let mut subtitle = header.borrow_pointer("/subtitle/runs")?;
    let subtitle_len = subtitle.as_array_iter_mut()?.len();
    let privacy = if subtitle_len == 5 {
        Some(PrivacyStatus::try_from(
            subtitle
                .take_value_pointer::<String, &str>("/text")?
                .as_str(),
        )?)
    } else {
        None
    };
    let year = subtitle.take_value_pointer(format!("/{}/text", subtitle_len.saturating_sub(1)))?;
    let views = header.take_value_pointer("/secondSubtitle/runs/0/text")?;
    let track_count_text = header.take_value_pointer("/secondSubtitle/runs/2/text")?;
    let duration = header.take_value_pointer("/secondSubtitle/runs/4/text")?;
    let id = header.take_value_pointer(
        "/buttons/1/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/playlistId",
    )?;
    let music_shelf = MusicShelfContents {
        json: columns.borrow_pointer(
            "/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents",
        )?,
    };
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

    #[tokio::test]
    async fn test_get_playlist_query_2024() {
        let source_path = Path::new("./test_json/get_playlist_20240624.json");
        let expected_path = Path::new("./test_json/get_playlist_20240624_output.txt");
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
