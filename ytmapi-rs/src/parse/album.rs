use super::{
    parse_flex_column_item, parse_library_management_items_from_menu, parse_song_artist, ParseFrom,
    ParsedSongArtist, ProcessedResult,
};
use crate::common::{
    AlbumType, Explicit, LibraryManager, LibraryStatus, LikeStatus, PlaylistID, Thumbnail, VideoID,
};
use crate::nav_consts::*;
use crate::process::fixed_column_item_pointer;
use crate::query::*;
use crate::Result;
use const_format::concatcp;
use json_crawler::{
    CrawlerResult, JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned,
};
use serde::{Deserialize, Serialize};

/// In some contexts, dislike will also be classified as indifferent.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum InLikedSongs {
    Liked,
    Indifferent,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct AlbumSong {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub plays: String,
    /// Library management fields are optional; if a album has already been
    /// added to your library, you cannot add the individual songs.
    // https://github.com/nick42d/youtui/issues/138
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub like_status: LikeStatus,
    pub explicit: Explicit,
}

// Is this similar to another struct?
// XXX: Consider correct privacy
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetAlbum {
    pub title: String,
    pub category: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
    pub artist_thumbnails: Vec<Thumbnail>,
    pub description: Option<String>,
    pub artists: Vec<ParsedSongArtist>,
    pub year: String,
    pub track_count_text: Option<String>,
    pub duration: String,
    pub audio_playlist_id: Option<PlaylistID<'static>>,
    // TODO: better interface
    pub tracks: Vec<AlbumSong>,
    pub library_status: LibraryStatus,
}

impl<'a> ParseFrom<GetAlbumQuery<'a>> for GetAlbum {
    fn parse_from(p: ProcessedResult<GetAlbumQuery<'a>>) -> crate::Result<Self> {
        parse_album_query(p)
    }
}

fn parse_album_track(json: &mut JsonCrawlerBorrowed) -> Result<Option<AlbumSong>> {
    let mut data = json.borrow_pointer(MRLIR)?;
    // A playlist item could be greyed out, and in this case we'll ignore the song
    // from the list of tracks.
    if let Ok("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .as_deref()
    {
        return Ok(None);
    }
    let title = super::parse_flex_column_item(&mut data, 0, 0)?;
    let library_management =
        parse_library_management_items_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let duration = data
        .borrow_pointer(fixed_column_item_pointer(0))
        .and_then(|mut i| {
            i.take_value_pointer("/text/simpleText")
                .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
        })?;
    let plays = parse_flex_column_item(&mut data, 2, 0)?;
    let track_no = data
        .borrow_pointer(concatcp!("/index", RUN_TEXT))?
        .take_and_parse_str()?;
    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    Ok(Some(AlbumSong {
        video_id,
        track_no,
        duration,
        plays,
        library_management,
        title,
        like_status,
        explicit,
    }))
}

// NOTE: Similar code to get_playlist_2024
fn parse_album_query(p: ProcessedResult<GetAlbumQuery>) -> Result<GetAlbum> {
    let json_crawler = JsonCrawlerOwned::from(p);
    let mut columns = json_crawler.navigate_pointer(TWO_COLUMN)?;
    let mut header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER))?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let category = header.take_value_pointer(SUBTITLE)?;
    let year = header.take_value_pointer(SUBTITLE2)?;
    let artists = header
        .borrow_pointer("/straplineTextOne/runs")?
        .try_into_iter()?
        .step_by(2)
        .map(|mut item| parse_song_artist(&mut item))
        .collect::<Result<Vec<ParsedSongArtist>>>()?;
    let description = header
        .borrow_pointer(DESCRIPTION_SHELF_RUNS)
        .and_then(|d| d.try_into_iter())
        .ok()
        .map(|r| {
            r.map(|mut r| r.take_value_pointer::<String>("/text"))
                .collect::<CrawlerResult<String>>()
        })
        .transpose()?;
    // artist thumbnails may not be present, refer to https://github.com/nick42d/youtui/issues/144
    let artist_thumbnails = header
        .take_value_pointer(STRAPLINE_THUMBNAIL)
        .unwrap_or_default();
    let thumbnails = header.take_value_pointer(THUMBNAILS)?;
    let duration = header.take_value_pointer("/secondSubtitle/runs/2/text")?;
    let track_count_text = header.take_value_pointer("/secondSubtitle/runs/0/text")?;
    let mut buttons = header.borrow_pointer("/buttons")?;
    // NOTE: Google is conducting an A/B rollout of renaming playlistId to
    // watchPlaylistId, so we will try both. https://github.com/nick42d/youtui/issues/205
    let audio_playlist_id = buttons
        .try_iter_mut()?
        .find_path("/musicPlayButtonRenderer")?
        .take_value_pointers(&[
            "/playNavigationEndpoint/watchEndpoint/playlistId",
            "/playNavigationEndpoint/watchPlaylistEndpoint/playlistId",
        ])?;
    let library_status = buttons
        .try_iter_mut()?
        .find_path("/toggleButtonRenderer")?
        .take_value_pointer("/defaultIcon/iconType")?;
    let tracks = columns
        .borrow_pointer(
            "/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents",
        )?
        .try_into_iter()?
        .filter_map(|mut track| parse_album_track(&mut track).transpose())
        .collect::<Result<Vec<AlbumSong>>>()?;
    Ok(GetAlbum {
        library_status,
        title,
        description,
        artist_thumbnails,
        duration,
        category,
        track_count_text,
        audio_playlist_id,
        year,
        tracks,
        artists,
        thumbnails,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::noauth::NoAuthToken;
    use crate::auth::BrowserToken;
    use crate::common::{AlbumID, YoutubeID};
    use crate::parse::album::GetAlbumQuery;

    #[tokio::test]
    async fn test_get_album_query() {
        parse_test!(
            "./test_json/get_album_20240724.json",
            "./test_json/get_album_20240724_output.txt",
            GetAlbumQuery::new(AlbumID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_album_query_no_artist_thumbnail() {
        parse_test!(
            "./test_json/get_album_various_artists_no_thumbnail_20240818.json",
            "./test_json/get_album_various_artists_no_thumbnail_20240818_output.txt",
            GetAlbumQuery::new(AlbumID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_album_query_not_signed_in() {
        parse_test!(
            "./test_json/get_album_not_signed_in_20250611.json",
            "./test_json/get_album_not_signed_in_20250611_output.txt",
            GetAlbumQuery::new(AlbumID::from_raw("")),
            NoAuthToken
        );
    }
}
