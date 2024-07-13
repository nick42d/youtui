use super::{LikeStatus, ParseFrom};
use crate::{
    common::{EntityID, UploadAlbumID, UploadArtistID},
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::{
        MENU_ITEMS, MENU_LIKE_STATUS, MRLIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
        PLAY_BUTTON, SECTION_LIST_ITEM, SINGLE_COLUMN, TAB_RENDERER, TEXT_RUN_TEXT, THUMBNAILS,
    },
    parse::parse_item_text,
    process::{
        get_delete_history_menu_from_menu, process_fixed_column_item, process_flex_column_item,
    },
    query::{
        GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery, GetLibraryUploadArtistQuery,
        GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
    },
    Error, Result, VideoID,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedUploadSongArtist {
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
    pub entity_id: EntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
    pub title: String,
    pub artists: Vec<ParsedUploadSongArtist>,
    pub thumbnails: Vec<super::Thumbnail>,
}

impl ParseFrom<GetLibraryUploadSongsQuery> for Vec<TableListUploadSong> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadSongsQuery>,
    ) -> Result<<GetLibraryUploadSongsQuery as crate::query::Query>::Output> {
        let crawler: JsonCrawler = p.into();
        let tabs_path = concatcp!(SINGLE_COLUMN, "/tabs");
        let contents = crawler
            .navigate_pointer(tabs_path)?
            .into_array_into_iter()?
            // Assume Uploads as always the last element.
            .last()
            .ok_or_else(|| {
                Error::other(format!(
                    "Expected array at <{tabs_path}> to contain elements",
                ))
            })?
            .navigate_pointer(concatcp!(
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
impl ParseFrom<GetLibraryUploadAlbumsQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumsQuery>,
    ) -> Result<<GetLibraryUploadAlbumsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl ParseFrom<GetLibraryUploadArtistsQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistsQuery>,
    ) -> Result<<GetLibraryUploadArtistsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<GetLibraryUploadAlbumQuery<'a>> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumQuery>,
    ) -> Result<<GetLibraryUploadAlbumQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<GetLibraryUploadArtistQuery<'a>> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistQuery>,
    ) -> Result<<GetLibraryUploadArtistQuery as crate::query::Query>::Output> {
        todo!()
    }
}
fn parse_upload_song_artists(
    mut data: JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<Vec<ParsedUploadSongArtist>> {
    process_flex_column_item(&mut data, col_idx)?
        .navigate_pointer("/text/runs")?
        .into_array_iter_mut()?
        .step_by(2)
        .map(|mut item| parse_upload_song_artist(&mut item))
        .collect()
}
fn parse_upload_song_artist(data: &mut JsonCrawlerBorrowed) -> Result<ParsedUploadSongArtist> {
    Ok(ParsedUploadSongArtist {
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
    let entity_id = get_delete_history_menu_from_menu(menu)?.take_value()?;
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

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{UploadAlbumID, UploadArtistID, YoutubeID},
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
            "./test_json/get_library_upload_artists_20240712.json",
            "./test_json/get_library_upload_artists_20240712_output.txt",
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
}
