use super::{
    fixed_column_item_pointer, flex_column_item_pointer, ParseFrom, DELETION_ENTITY_ID,
    HEADER_DETAIL, SECOND_SUBTITLE_RUNS, SUBTITLE,
};
use crate::common::{
    AlbumType, LikeStatus, Thumbnail, UploadAlbumID, UploadArtistID, UploadEntityID, VideoID,
};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    CONTINUATION_PARAMS, GRID_ITEMS, INDEX_TEXT, MENU_ITEMS, MENU_LIKE_STATUS, MRLIR, MUSIC_SHELF,
    MUSIC_SHELF_CONTINUATION, NAVIGATION_BROWSE_ID, PLAY_BUTTON, SECTION_LIST_ITEM,
    SINGLE_COLUMN_TAB, SINGLE_COLUMN_TABS, SUBTITLE2, SUBTITLE3, TAB_RENDERER, TEXT_RUN_TEXT,
    THUMBNAILS, THUMBNAIL_ANIMATED_ICON, THUMBNAIL_BADGE_ICON, THUMBNAIL_CROPPED,
    THUMBNAIL_RENDERER, TITLE_TEXT, WATCH_VIDEO_ID,
};
use crate::parse::{parse_fixed_column_item, parse_flex_column_item};
use crate::query::{
    DeleteUploadEntityQuery, GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery,
    GetLibraryUploadArtistQuery, GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
};
use crate::youtube_enums::{YoutubeMusicAnimatedIcon, YoutubeMusicBadgeRendererIcon};
use crate::{Error, Result};
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct ParsedUploadArtist {
    pub name: String,
    pub id: Option<UploadArtistID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct ParsedUploadSongAlbum {
    pub name: String,
    pub id: UploadAlbumID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// May need to be enum to track 'Not Available' case.
pub struct TableListUploadSong {
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: Option<ParsedUploadSongAlbum>,
    pub duration: String,
    pub like_status: LikeStatus,
    pub title: String,
    pub artists: Vec<ParsedUploadArtist>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct UploadArtist {
    pub artist_name: String,
    pub song_count: String,
    pub artist_id: UploadArtistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
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
#[non_exhaustive]
// May need to be enum to track 'Not Available' case.
pub struct GetLibraryUploadAlbumSong {
    pub title: String,
    pub track_no: i64,
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
}

impl ParseFromContinuable<GetLibraryUploadSongsQuery> for Vec<TableListUploadSong> {
    fn parse_from_continuable(
        p: super::ProcessedResult<GetLibraryUploadSongsQuery>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let mut music_shelf = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
        ))?;
        let continuation_params = music_shelf.take_value_pointer(CONTINUATION_PARAMS).ok();
        let songs = parse_table_list_upload_songs(music_shelf)?;
        Ok((songs, continuation_params))
    }
    fn parse_continuation(
        p: super::ProcessedResult<
            crate::query::GetContinuationsQuery<'_, GetLibraryUploadSongsQuery>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let mut music_shelf = crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
        let continuation_params = music_shelf.take_value_pointer(CONTINUATION_PARAMS).ok();
        let songs = parse_table_list_upload_songs(music_shelf)?;
        Ok((songs, continuation_params))
    }
}
impl ParseFromContinuable<GetLibraryUploadArtistsQuery> for Vec<UploadArtist> {
    fn parse_from_continuable(
        p: super::ProcessedResult<GetLibraryUploadArtistsQuery>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        fn parse_item_list_upload_artist(
            mut json_crawler: JsonCrawlerOwned,
        ) -> Result<UploadArtist> {
            let mut data = json_crawler.borrow_pointer(MRLIR)?;
            let artist_name = parse_flex_column_item(&mut data.borrow_mut(), 0, 0)?;
            let songs = parse_flex_column_item(&mut data.borrow_mut(), 1, 0)?;
            let thumbnails = data.take_value_pointer(THUMBNAILS)?;
            let artist_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            Ok(UploadArtist {
                thumbnails,
                artist_name,
                song_count: songs,
                artist_id,
            })
        }
        let crawler: JsonCrawlerOwned = p.into();
        let items = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
            "/contents"
        ))?;
        let res = items
            .try_into_iter()?
            .map(parse_item_list_upload_artist)
            .collect::<Result<Vec<_>>>()?;
        Ok((res, None))
    }
    fn parse_continuation(
        p: super::ProcessedResult<
            crate::query::GetContinuationsQuery<'_, GetLibraryUploadArtistsQuery>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        todo!()
    }
}
impl ParseFromContinuable<GetLibraryUploadArtistQuery<'_>> for Vec<TableListUploadSong> {
    fn parse_from_continuable(
        p: super::ProcessedResult<GetLibraryUploadArtistQuery<'_>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let contents = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
            "/contents"
        ))?;
        let res = contents
            .try_into_iter()?
            .map(|mut item| {
                let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                    return Ok(None);
                };
                // Handle list item is "Shuffle all"
                if data
                    .take_value_pointer::<YoutubeMusicBadgeRendererIcon>(THUMBNAIL_BADGE_ICON)
                    .is_ok()
                {
                    return Ok(None);
                }
                // Handle list item is "x song(s) processing..."
                if data
                    .take_value_pointer::<YoutubeMusicAnimatedIcon>(THUMBNAIL_ANIMATED_ICON)
                    .is_ok()
                {
                    return Ok(None);
                }
                Ok(Some(parse_table_list_upload_song(data)?))
            })
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>>>()?;
        Ok((res, None))
    }
    fn parse_continuation(
        p: super::ProcessedResult<
            crate::query::GetContinuationsQuery<'_, GetLibraryUploadArtistQuery<'_>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        todo!()
    }
}
impl ParseFromContinuable<GetLibraryUploadAlbumsQuery> for Vec<UploadAlbum> {
    fn parse_from_continuable(
        p: super::ProcessedResult<GetLibraryUploadAlbumsQuery>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        fn parse_item_list_upload_album(mut json_crawler: JsonCrawlerOwned) -> Result<UploadAlbum> {
            let mut data = json_crawler.borrow_pointer("/musicTwoRowItemRenderer")?;
            let album_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let thumbnails = data.take_value_pointer(THUMBNAIL_RENDERER)?;
            let title = data.take_value_pointer(TITLE_TEXT)?;
            let artist = data.take_value_pointer(SUBTITLE2)?;
            let year = data.take_value_pointer(SUBTITLE3).ok();
            let entity_id = data
                .borrow_pointer(MENU_ITEMS)?
                .try_iter_mut()?
                .find_path(DELETION_ENTITY_ID)?
                .take_value()?;
            Ok(UploadAlbum {
                title,
                year,
                thumbnails,
                artist,
                entity_id,
                album_id,
            })
        }
        let crawler: JsonCrawlerOwned = p.into();
        let items = get_uploads_tab(crawler)?.navigate_pointer(concatcp!(
            TAB_RENDERER,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        let res = items
            .try_into_iter()?
            .map(parse_item_list_upload_album)
            .collect::<Result<Vec<_>>>()?;
        Ok((res, None))
    }
    fn parse_continuation(
        p: super::ProcessedResult<
            crate::query::GetContinuationsQuery<'_, GetLibraryUploadAlbumsQuery>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        todo!()
    }
}
impl ParseFrom<GetLibraryUploadAlbumQuery<'_>> for GetLibraryUploadAlbum {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumQuery<'_>>,
    ) -> crate::Result<Self> {
        fn parse_playlist_upload_song(
            mut json_crawler: JsonCrawlerOwned,
        ) -> Result<GetLibraryUploadAlbumSong> {
            let mut data = json_crawler.borrow_pointer(MRLIR)?;
            let title = parse_flex_column_item(&mut data.borrow_mut(), 0, 0)?;
            let album = parse_upload_song_album(data.borrow_mut(), 2)?;
            let duration = parse_fixed_column_item(&mut data.borrow_mut(), 0)?;
            let track_no = data.borrow_pointer(INDEX_TEXT)?.take_and_parse_str()?;
            let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
            let video_id = data.take_value_pointer(concatcp!(
                PLAY_BUTTON,
                "/playNavigationEndpoint",
                WATCH_VIDEO_ID
            ))?;
            let entity_id = data
                .borrow_pointer(MENU_ITEMS)?
                .try_iter_mut()?
                .find_path(DELETION_ENTITY_ID)?
                .take_value()?;
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
        let mut crawler: JsonCrawlerOwned = p.into();
        let mut header = crawler.borrow_pointer(HEADER_DETAIL)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let album_type = header.take_value_pointer(SUBTITLE)?;
        let artist_name = header.take_value_pointer(SUBTITLE2)?;
        let song_count = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/0/text"))?;
        let duration = header.take_value_pointer(concatcp!(SECOND_SUBTITLE_RUNS, "/2/text"))?;
        let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
        let entity_id = header
            .navigate_pointer(MENU_ITEMS)?
            .try_into_iter()?
            .find_path(DELETION_ENTITY_ID)?
            .take_value()?;
        let songs = crawler
            .navigate_pointer(concatcp!(
                SINGLE_COLUMN_TAB,
                SECTION_LIST_ITEM,
                MUSIC_SHELF,
                "/contents"
            ))?
            .try_into_iter()?
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
impl<'a> ParseFrom<DeleteUploadEntityQuery<'a>> for () {
    fn parse_from(p: super::ProcessedResult<DeleteUploadEntityQuery<'a>>) -> crate::Result<Self> {
        let crawler: JsonCrawlerOwned = p.into();
        // Passing an invalid entity ID with will throw a 400 error which
        // is caught by AuthToken.
        // NOTE: Passing the same entity id for deletion multiple times
        crawler
            .navigate_pointer("/actions")?
            .try_into_iter()?
            .find_path("/addToToastAction")
            .map(|_| ())
            .map_err(Into::into)
    }
}
pub(crate) fn parse_upload_song_artists(
    data: impl JsonCrawler,
    col_idx: usize,
) -> Result<Vec<ParsedUploadArtist>> {
    data.navigate_pointer(format!("{}/text/runs", flex_column_item_pointer(col_idx)))?
        .try_into_iter()?
        .step_by(2)
        .map(|item| parse_upload_song_artist(item))
        .collect()
}
fn parse_upload_song_artist(mut data: impl JsonCrawler) -> Result<ParsedUploadArtist> {
    Ok(ParsedUploadArtist {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
    })
}
pub(crate) fn parse_upload_song_album(
    mut data: impl JsonCrawler,
    col_idx: usize,
) -> Result<ParsedUploadSongAlbum> {
    Ok(ParsedUploadSongAlbum {
        name: parse_flex_column_item(&mut data, col_idx, 0)?,
        id: data.take_value_pointer(format!(
            "{}/text/runs/0{NAVIGATION_BROWSE_ID}",
            flex_column_item_pointer(col_idx)
        ))?,
    })
}
fn parse_table_list_upload_songs(
    music_shelf: impl JsonCrawler,
) -> crate::Result<Vec<TableListUploadSong>> {
    let contents = music_shelf.navigate_pointer("/contents")?;
    contents
        .try_into_iter()?
        .map(|mut item| {
            let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                return Ok(None);
            };
            // Handle list item is "Shuffle all"
            if data
                .take_value_pointer::<YoutubeMusicBadgeRendererIcon>(THUMBNAIL_BADGE_ICON)
                .is_ok()
            {
                return Ok(None);
            }
            // Handle list item is "x song(s) processing..."
            if data
                .take_value_pointer::<YoutubeMusicAnimatedIcon>(THUMBNAIL_ANIMATED_ICON)
                .is_ok()
            {
                return Ok(None);
            }
            Ok(Some(parse_table_list_upload_song(data.borrow_mut())?))
        })
        .filter_map(Result::transpose)
        .collect()
}
pub(crate) fn parse_table_list_upload_song(
    mut crawler: impl JsonCrawler,
) -> Result<TableListUploadSong> {
    let title: String = parse_flex_column_item(&mut crawler, 0, 0)?;
    let duration =
        crawler.take_value_pointer(format!("{}{TEXT_RUN_TEXT}", fixed_column_item_pointer(0)))?;
    let like_status = crawler.take_value_pointer(MENU_LIKE_STATUS)?;
    let video_id = crawler.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchEndpoint/videoId"
    ))?;
    let thumbnails = crawler.take_value_pointer(THUMBNAILS)?;
    // An uploaded song may not have artists metadata
    let artists = parse_upload_song_artists(crawler.borrow_mut(), 1).unwrap_or_default();
    // An uploaded song may not have aalbum metadata
    let album = parse_upload_song_album(crawler.borrow_mut(), 2).ok();
    let entity_id = crawler
        .navigate_pointer(MENU_ITEMS)?
        .try_into_iter()?
        .find_path(DELETION_ENTITY_ID)?
        .take_value()?;
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

fn get_uploads_tab(json: JsonCrawlerOwned) -> Result<JsonCrawlerOwned> {
    let tabs_path = concatcp!(SINGLE_COLUMN_TABS);
    json.navigate_pointer(tabs_path)?
        .try_into_iter()?
        .try_last()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{UploadAlbumID, UploadArtistID, UploadEntityID, YoutubeID};
    #[tokio::test]
    async fn test_get_library_upload_songs() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_upload_songs_20240712.json",
            "./test_json/get_library_upload_songs_continuation_20240712.json",
            "./test_json/get_library_upload_songs_20240712_output.txt",
            crate::query::GetLibraryUploadSongsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_albums() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_upload_albums_20240712.json",
            "./test_json/get_library_upload_albums_continuation_20240712.json",
            "./test_json/get_library_upload_albums_20240712_output.txt",
            crate::query::GetLibraryUploadAlbumsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artists() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_upload_artists_20240712.json",
            "./test_json/get_library_upload_artists_continuation_20240712.json",
            "./test_json/get_library_upload_artists_20240712_output.txt",
            crate::query::GetLibraryUploadArtistsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artist() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_upload_artist_20240712.json",
            "./test_json/get_library_upload_artist_continuation_20240712.json",
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
        parse_test_value!(
            "./test_json/delete_upload_entity_20240715.json",
            (),
            crate::query::DeleteUploadEntityQuery::new(UploadEntityID::from_raw("")),
            BrowserToken
        );
    }
}
