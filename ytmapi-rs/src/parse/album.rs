use std::sync::Arc;

use super::{
    parse_flex_column_item, parse_song_artist, ParseFrom, ParsedSongArtist, ProcessedResult,
};
use crate::common::{
    AlbumType, Explicit, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary,
};
use crate::common::{PlaylistID, Thumbnail};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator};
use crate::process::process_fixed_column_item;
use crate::query::*;
use crate::{nav_consts::*, VideoID};
use crate::{Error, Result};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct LibraryManager {
    pub status: LibraryStatus,
    pub add_to_library_token: FeedbackTokenAddToLibrary<'static>,
    pub remove_from_library_token: FeedbackTokenRemoveFromLibrary<'static>,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum LibraryStatus {
    #[serde(alias = "LIBRARY_SAVED")]
    InLibrary,
    #[serde(alias = "LIBRARY_ADD")]
    NotInLibrary,
}

/// In some contexts, dislike will also be classified as indifferent.
#[derive(Debug)]
pub enum InLikedSongs {
    Liked,
    Indifferent,
}

/// Indifferent means that the song has not been liked or disliked.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum LikeStatus {
    #[serde(alias = "LIKE")]
    Liked,
    #[serde(alias = "DISLIKE")]
    Disliked,
    #[serde(alias = "INDIFFERENT")]
    Indifferent,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct AlbumSong {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub plays: String,
    pub library_status: LibraryStatus,
    pub feedback_tok_add: FeedbackTokenAddToLibrary<'static>,
    pub feedback_tok_rem: FeedbackTokenRemoveFromLibrary<'static>,
    pub title: String,
    pub like_status: LikeStatus,
    pub explicit: Explicit,
}

// Is this similar to another struct?
// XXX: Consider correct privacy
#[derive(Debug)]
pub struct AlbumParams {
    pub title: String,
    pub category: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
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

impl<'a> ParseFrom<GetAlbumQuery<'a>> for AlbumParams {
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
    let mut library_menu = data
        .borrow_pointer(MENU_ITEMS)?
        .into_array_iter_mut()?
        .find_path("/toggleMenuServiceItemRenderer")?;
    let library_status = library_menu.take_value_pointer("/defaultIcon/iconType")?;
    let (feedback_tok_add, feedback_tok_rem) = match library_status {
        LibraryStatus::InLibrary => (
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
        ),
        LibraryStatus::NotInLibrary => (
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
        ),
    };
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
        i.take_value_pointer("/text/simpleText")
            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
    })?;
    let plays = parse_flex_column_item(&mut data, 2, 0)?;
    let track_no = str::parse::<usize>(
        data.take_value_pointer::<String>(concatcp!("/index", RUN_TEXT))?
            .as_str(),
    )
    .map_err(|e| {
        Error::parsing(
            data.get_path(),
            // TODO: Remove allocation.
            Arc::new(data.get_source().to_owned()),
            crate::error::ParseTarget::Other("usize".to_string()),
            Some(e.to_string()),
        )
    })?;
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
        library_status,
        feedback_tok_add,
        feedback_tok_rem,
        title,
        like_status,
        explicit,
    }))
}

// NOTE: Similar code to get_playlist_2024
fn parse_album_query(p: ProcessedResult<GetAlbumQuery>) -> Result<AlbumParams> {
    let json_crawler = JsonCrawler::from(p);
    let mut columns = json_crawler.navigate_pointer(TWO_COLUMN)?;
    let mut header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER))?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let category = header.take_value_pointer(SUBTITLE)?;
    let year = header.take_value_pointer(SUBTITLE2)?;
    let artists = header
        .borrow_pointer("/straplineTextOne/runs")?
        .into_array_iter_mut()?
        .step_by(2)
        .map(|mut item| parse_song_artist(&mut item))
        .collect::<Result<Vec<ParsedSongArtist>>>()?;
    let description = header
        .borrow_pointer(DESCRIPTION_SHELF_RUNS)
        .and_then(|d| d.into_array_iter_mut())
        .ok()
        .map(|r| {
            r.map(|mut r| r.take_value_pointer::<String>("/text"))
                .collect::<Result<String>>()
        })
        .transpose()?;
    let thumbnails: Vec<Thumbnail> = header.take_value_pointer(STRAPLINE_THUMBNAIL)?;
    let duration = header.take_value_pointer("/secondSubtitle/runs/2/text")?;
    let track_count_text = header.take_value_pointer("/secondSubtitle/runs/0/text")?;
    let audio_playlist_id = header.take_value_pointer(
        "/buttons/1/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/playlistId",
    )?;
    let library_status =
        header.take_value_pointer("/buttons/0/toggleButtonRenderer/defaultIcon/iconType")?;
    let tracks = columns
        .borrow_pointer(
            "/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents",
        )?
        .into_array_iter_mut()?
        .filter_map(|mut track| parse_album_track(&mut track).transpose())
        .collect::<Result<Vec<AlbumSong>>>()?;
    Ok(AlbumParams {
        library_status,
        title,
        description,
        thumbnails,
        duration,
        category,
        track_count_text,
        audio_playlist_id,
        year,
        tracks,
        artists,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{AlbumID, YoutubeID},
        parse::album::GetAlbumQuery,
    };

    #[tokio::test]
    async fn test_get_album_query() {
        parse_test!(
            "./test_json/get_album_20240724.json",
            "./test_json/get_album_20240724_output.txt",
            GetAlbumQuery::new(AlbumID::from_raw("")),
            BrowserToken
        );
    }
}
