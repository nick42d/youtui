use super::{
    ApiSuccess, LikeStatus, ParseFrom, DELETION_ENTITY_ID, HEADER_DETAIL, SECOND_SUBTITLE_RUNS,
    SUBTITLE,
};
use crate::{
    common::{AlbumType, UploadAlbumID, UploadArtistID, UploadEntityID},
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::{
        GRID_ITEMS, INDEX_TEXT, MENU_ITEMS, MENU_LIKE_STATUS, MRLIR, MUSIC_SHELF,
        NAVIGATION_BROWSE_ID, PLAY_BUTTON, SECTION_LIST_ITEM, SINGLE_COLUMN, SINGLE_COLUMN_TAB,
        SUBTITLE2, SUBTITLE3, TAB_RENDERER, TEXT_RUN_TEXT, THUMBNAILS, THUMBNAIL_CROPPED,
        THUMBNAIL_RENDERER, TITLE_TEXT, WATCH_VIDEO_ID,
    },
    parse::parse_item_text,
    process::{process_fixed_column_item, process_flex_column_item},
    query::{
        DeleteUploadEntityQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
        GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
    },
    Error, Result, Thumbnail, VideoID,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedUploadArtist {
    pub name: String,
    pub id: Option<UploadArtistID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedUploadSongAlbum {
    pub name: String,
    pub id: UploadAlbumID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// May need to be enum to track 'Not Available' case.
// TODO: Move to common
pub struct TableListUploadSong {
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
    pub title: String,
    pub artists: Vec<ParsedUploadArtist>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct UploadAlbum {
    pub title: String,
    pub artist: String,
    // Year appears to be optional.
    pub year: Option<String>,
    pub entity_id: UploadEntityID<'static>,
    pub album_id: UploadAlbumID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct UploadArtist {
    pub artist_name: String,
    pub song_count: String,
    pub artist_id: UploadArtistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetLibraryUploadAlbum {
    pub title: String,
    pub artist_name: String,
    pub album_type: AlbumType,
    pub song_count: String,
    pub duration: String,
    pub entity_id: UploadEntityID<'static>,
    pub songs: Vec<GetLibraryUploadAlbumSong>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// May need to be enum to track 'Not Available' case.
// TODO: Move to common
pub struct GetLibraryUploadAlbumSong {
    pub title: String,
    pub track_no: i64,
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
}

impl ParseFrom<GetLibraryUploadSongsQuery> for Vec<TableListUploadSong> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadSongsQuery>,
    ) -> Result<<GetLibraryUploadSongsQuery as crate::query::Query>::Output> {
        let crawler: JsonCrawler = p.into();
        let contents = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
            "/contents"
        ))?;
        contents
            .into_array_into_iter()?
            .map(|mut item| {
                let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                    return Ok(None);
                };
                let title = parse_item_text(&mut data, 0, 0)?;
                if title == "Shuffle all" {
                    return Ok(None);
                };
                Ok(Some(parse_table_list_upload_song(title, data)?))
            })
            .filter_map(Result::transpose)
            .collect()
    }
}
impl ParseFrom<GetLibraryUploadAlbumsQuery> for Vec<UploadAlbum> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumsQuery>,
    ) -> Result<<GetLibraryUploadAlbumsQuery as crate::query::Query>::Output> {
        fn parse_item_list_upload_album(mut json_crawler: JsonCrawler) -> Result<UploadAlbum> {
            let mut data = json_crawler.borrow_pointer("/musicTwoRowItemRenderer")?;
            let album_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let thumbnails = data.take_value_pointer(THUMBNAIL_RENDERER)?;
            let title = data.take_value_pointer(TITLE_TEXT)?;
            let artist = data.take_value_pointer(SUBTITLE2)?;
            let year = data.take_value_pointer(SUBTITLE3).ok();
            let menu = data.borrow_pointer(MENU_ITEMS)?;
            let entity_id = get_delete_history_entity_from_menu(menu)?.take_value()?;
            Ok(UploadAlbum {
                title,
                year,
                thumbnails,
                artist,
                entity_id,
                album_id,
            })
        }
        let crawler: JsonCrawler = p.into();
        let items = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        items
            .into_array_into_iter()?
            .map(parse_item_list_upload_album)
            .collect()
    }
}
impl ParseFrom<GetLibraryUploadArtistsQuery> for Vec<UploadArtist> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistsQuery>,
    ) -> Result<<GetLibraryUploadArtistsQuery as crate::query::Query>::Output> {
        fn parse_item_list_upload_artist(mut json_crawler: JsonCrawler) -> Result<UploadArtist> {
            let mut data = json_crawler.borrow_pointer(MRLIR)?;
            let artist_name = parse_item_text(&mut data.borrow_mut(), 0, 0)?;
            let songs = parse_item_text(&mut data.borrow_mut(), 1, 0)?;
            let thumbnails = data.take_value_pointer(THUMBNAILS)?;
            let artist_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            Ok(UploadArtist {
                thumbnails,
                artist_name,
                song_count: songs,
                artist_id,
            })
        }
        let crawler: JsonCrawler = p.into();
        let items = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
            "/contents"
        ))?;
        items
            .into_array_into_iter()?
            .map(parse_item_list_upload_artist)
            .collect()
    }
}
impl<'a> ParseFrom<GetLibraryUploadAlbumQuery<'a>> for GetLibraryUploadAlbum {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumQuery>,
    ) -> Result<<GetLibraryUploadAlbumQuery as crate::query::Query>::Output> {
        fn parse_playlist_upload_song(
            mut json_crawler: JsonCrawler,
        ) -> Result<GetLibraryUploadAlbumSong> {
            let mut data = json_crawler.borrow_pointer(MRLIR)?;
            let title = parse_item_text(&mut data.borrow_mut(), 0, 0)?;
            let album = parse_upload_song_album(data.borrow_mut(), 2)?;
            let duration = process_fixed_column_item(&mut data.borrow_mut(), 0)?
                .take_value_pointer(TEXT_RUN_TEXT)?;
            let track_no = str::parse(data.take_value_pointer::<String, _>(INDEX_TEXT)?.as_str())
                .map_err(|e| Error::other(format!("Error {e} parsing into u64")))?;
            let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
            let video_id = data.take_value_pointer(concatcp!(
                PLAY_BUTTON,
                "/playNavigationEndpoint",
                WATCH_VIDEO_ID
            ))?;
            let menu = data.navigate_pointer(MENU_ITEMS)?;
            let entity_id = get_delete_history_entity_from_menu(menu)?.take_value()?;
            Ok(GetLibraryUploadAlbumSong {
                title,
                track_no,
                entity_id,
                video_id,
                album,
                duration,
                like_status,
            })
        }
        let mut crawler: JsonCrawler = p.into();
        let mut header = crawler.borrow_pointer(HEADER_DETAIL)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let album_type = header.take_value_pointer(SUBTITLE)?;
        let artist_name = header.take_value_pointer(SUBTITLE2)?;
        let song_count = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/0/text"))?;
        let duration = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/2/text"))?;
        let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
        let menu = header.navigate_pointer(MENU_ITEMS)?;
        let entity_id = get_delete_history_entity_from_menu(menu)?.take_value()?;
        let songs = crawler
            .navigate_pointer(concatcp!(
                SINGLE_COLUMN_TAB,
                SECTION_LIST_ITEM,
                MUSIC_SHELF,
                "/contents"
            ))?
            .into_array_into_iter()?
            .map(parse_playlist_upload_song)
            .collect::<Result<Vec<_>>>()?;
        Ok(GetLibraryUploadAlbum {
            title,
            artist_name,
            album_type,
            song_count,
            duration,
            entity_id,
            songs,
            thumbnails,
        })
    }
}
impl<'a> ParseFrom<GetLibraryUploadArtistQuery<'a>> for Vec<TableListUploadSong> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistQuery>,
    ) -> Result<<GetLibraryUploadArtistQuery as crate::query::Query>::Output> {
        let crawler: JsonCrawler = p.into();
        let contents = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
            "/contents"
        ))?;
        contents
            .into_array_into_iter()?
            .map(|mut item| {
                let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                    return Ok(None);
                };
                let title = parse_item_text(&mut data, 0, 0)?;
                if title == "Shuffle all" {
                    return Ok(None);
                };
                Ok(Some(parse_table_list_upload_song(title, data)?))
            })
            .filter_map(Result::transpose)
            .collect()
    }
}
impl<'a> ParseFrom<DeleteUploadEntityQuery<'a>> for ApiSuccess {
    fn parse_from(
        p: super::ProcessedResult<DeleteUploadEntityQuery<'a>>,
    ) -> crate::Result<<DeleteUploadEntityQuery<'a> as crate::query::Query>::Output> {
        let crawler: JsonCrawler = p.into();
        // Passing an invalid entity ID with will throw a 400 error which
        // is caught by AuthToken.
        // NOTE: Passing the same entity id for deletion multiple times
        crawler
            .navigate_pointer("/actions")?
            .into_array_into_iter()?
            .find_map(|a| a.navigate_pointer("/addToToastAction").ok())
            .map(|_| ApiSuccess)
            .ok_or_else(|| Error::other("Expected /actions to contain a /addToToastAction"))
    }
}
fn parse_upload_song_artists(
    mut data: JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<Vec<ParsedUploadArtist>> {
    process_flex_column_item(&mut data, col_idx)?
        .navigate_pointer("/text/runs")?
        .into_array_iter_mut()?
        .step_by(2)
        .map(|mut item| parse_upload_song_artist(&mut item))
        .collect()
}
fn parse_upload_song_artist(data: &mut JsonCrawlerBorrowed) -> Result<ParsedUploadArtist> {
    Ok(ParsedUploadArtist {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
    })
}
fn parse_upload_song_album(
    mut data: JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<ParsedUploadSongAlbum> {
    Ok(ParsedUploadSongAlbum {
        name: parse_item_text(&mut data, col_idx, 0)?,
        id: process_flex_column_item(&mut data, col_idx)?
            .take_value_pointer(concatcp!("/text/runs/0", NAVIGATION_BROWSE_ID))?,
    })
}
pub(crate) fn parse_table_list_upload_song(
    title: String,
    mut crawler: JsonCrawlerBorrowed,
) -> Result<TableListUploadSong> {
    let duration = process_fixed_column_item(&mut crawler.borrow_mut(), 0)?
        .take_value_pointer(TEXT_RUN_TEXT)?;
    let like_status = crawler.take_value_pointer(MENU_LIKE_STATUS)?;
    let video_id = crawler.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchEndpoint/videoId"
    ))?;
    let thumbnails = crawler.take_value_pointer(THUMBNAILS)?;
    let artists = parse_upload_song_artists(crawler.borrow_mut(), 1)?;
    let album = parse_upload_song_album(crawler.borrow_mut(), 2)?;
    let menu = crawler.borrow_pointer(MENU_ITEMS)?;
    let entity_id = get_delete_history_entity_from_menu(menu)?.take_value()?;
    Ok(TableListUploadSong {
        entity_id,
        video_id,
        album,
        duration,
        like_status,
        title,
        artists,
        thumbnails,
    })
}

fn get_delete_history_entity_from_menu(menu: JsonCrawlerBorrowed) -> Result<JsonCrawlerBorrowed> {
    let cur_path = menu.get_path();
    menu.into_array_iter_mut()?
        .find_map(|item| item.navigate_pointer(DELETION_ENTITY_ID).ok())
        // Future function try_map() will potentially eliminate this ok->ok_or_else combo.
        .ok_or_else(|| {
            Error::other(format!("Expected playlist item to contain at least one <{DELETION_ENTITY_ID}> underneath path {cur_path}"))
        })
}

fn get_uploads_tab(json: JsonCrawler) -> Result<JsonCrawler> {
    let tabs_path = concatcp!(SINGLE_COLUMN, "/tabs");
    json.navigate_pointer(tabs_path)?
        .into_array_into_iter()?
        // Assume Uploads as always the last element.
        .last()
        .ok_or_else(|| {
            Error::other(format!(
                "Expected array at <{tabs_path}> to contain elements",
            ))
        })
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{UploadAlbumID, UploadArtistID, UploadEntityID, YoutubeID},
    };
    #[tokio::test]
    async fn test_get_library_upload_songs() {
        parse_test!(
            "./test_json/get_library_upload_songs_20240712.json",
            "./test_json/get_library_upload_songs_20240712_output.txt",
            crate::query::GetLibraryUploadSongsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_albums() {
        parse_test!(
            "./test_json/get_library_upload_albums_20240712.json",
            "./test_json/get_library_upload_albums_20240712_output.txt",
            crate::query::GetLibraryUploadAlbumsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artists() {
        parse_test!(
            "./test_json/get_library_upload_artists_20240712.json",
            "./test_json/get_library_upload_artists_20240712_output.txt",
            crate::query::GetLibraryUploadArtistsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artist() {
        parse_test!(
            "./test_json/get_library_upload_artist_20240712.json",
            "./test_json/get_library_upload_artist_20240712_output.txt",
            crate::query::GetLibraryUploadArtistQuery::new(UploadArtistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_album() {
        parse_test!(
            "./test_json/get_library_upload_album_20240712.json",
            "./test_json/get_library_upload_album_20240712_output.txt",
            crate::query::GetLibraryUploadAlbumQuery::new(UploadAlbumID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_delete_upload_entity() {
        parse_test!(
            "./test_json/delete_upload_entity_20240715.json",
            "./test_json/api_success_output.txt",
            crate::query::DeleteUploadEntityQuery::new(UploadEntityID::from_raw("")),
            BrowserToken
        );
    }
}