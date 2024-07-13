use super::{LibraryManager, LikeStatus, ParseFrom};
use crate::{
    common::{EntityID, Explicit, PlaylistID, UploadAlbumID, UploadArtistID, YoutubeID},
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::{
        DELETION_ENTITY_ID, MENU_ITEMS, MENU_LIKE_STATUS, MRLIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
        PLAYLIST_ITEM_VIDEO_ID, PLAY_BUTTON, SECTION_LIST_ITEM, SINGLE_COLUMN, TAB_RENDERER,
        TEXT_RUN_TEXT, THUMBNAILS, TWO_COLUMN,
    },
    parse::parse_item_text,
    process::{process_fixed_column_item, process_flex_column_item},
    query::{
        GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery, GetLibraryUploadArtistQuery,
        GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
    },
    utils::constants::USER_AGENT,
    Error, Result, VideoID,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedLibraryUploadSongArtist {
    pub name: String,
    pub id: Option<UploadArtistID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedLibraryUploadSongAlbum {
    pub name: String,
    pub id: UploadAlbumID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// May need to be enum to track 'Not Available' case.
pub struct LibraryUploadSong {
    pub entity_id: EntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedLibraryUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
    pub title: String,
    pub artists: Vec<ParsedLibraryUploadSongArtist>,
    pub thumbnails: Vec<super::Thumbnail>,
}

impl ParseFrom<GetLibraryUploadSongsQuery> for Vec<LibraryUploadSong> {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadSongsQuery>,
    ) -> Result<<GetLibraryUploadSongsQuery as crate::query::Query>::Output> {
        fn parse_library_upload_song_artists(
            mut data: JsonCrawlerBorrowed,
            col_idx: usize,
        ) -> Result<Vec<ParsedLibraryUploadSongArtist>> {
            process_flex_column_item(data.borrow_mut(), col_idx)?
                .navigate_pointer("/text/runs")?
                .into_array_iter_mut()?
                .step_by(2)
                .map(|mut item| parse_library_upload_song_artist(&mut item))
                .collect()
        }
        fn parse_library_upload_song_artist(
            data: &mut JsonCrawlerBorrowed,
        ) -> Result<ParsedLibraryUploadSongArtist> {
            Ok(ParsedLibraryUploadSongArtist {
                name: data.take_value_pointer("/text")?,
                id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
            })
        }
        fn parse_library_upload_song_album(
            mut data: JsonCrawlerBorrowed,
            col_idx: usize,
        ) -> Result<ParsedLibraryUploadSongAlbum> {
            Ok(ParsedLibraryUploadSongAlbum {
                name: parse_item_text(&mut data, col_idx, 0)?,
                id: process_flex_column_item(&mut data, col_idx)?
                    .take_value_pointer(concatcp!("/text/runs/0", NAVIGATION_BROWSE_ID))?,
            })
        }
        fn parse_library_upload_song(crawler: JsonCrawler) -> Result<Option<LibraryUploadSong>> {
            let mut inner = crawler.navigate_pointer(MRLIR)?;
            let title = parse_item_text(&mut inner.borrow_mut(), 0, 0)?;
            if &title == "Shuffle all" {
                return Ok(None);
            };
            let duration = process_fixed_column_item(&mut inner.borrow_mut(), 0)?
                .take_value_pointer(TEXT_RUN_TEXT)?;
            let like_status = inner.take_value_pointer(MENU_LIKE_STATUS)?;
            let video_id = inner.take_value_pointer(concatcp!(
                PLAY_BUTTON,
                "/playNavigationEndpoint/watchEndpoint/videoId"
            ))?;
            let thumbnails = inner.take_value_pointer(THUMBNAILS)?;
            let artists = parse_library_upload_song_artists(inner.borrow_mut(), 1)?;
            let album = parse_library_upload_song_album(inner.borrow_mut(), 2)?;
            let mut menu = inner.borrow_pointer(MENU_ITEMS)?;
            let menu_path = menu.get_path();
            let entity_id = menu
                .as_array_iter_mut()?
                .last()
                .ok_or_else(|| {
                    Error::other(format!("Expected <{menu_path}> to contain array elements"))
                })?
                .take_value_pointer(DELETION_ENTITY_ID)?;
            Ok(Some(LibraryUploadSong {
                entity_id,
                video_id,
                album,
                duration,
                like_status,
                title,
                artists,
                thumbnails,
            }))
        }
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
            .filter_map(|item| parse_library_upload_song(item).transpose())
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
