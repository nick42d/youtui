use super::{
    parse_playlist_items, parse_song_artist, parse_song_artists, ParseFrom, ParsedSongArtist,
    PlaylistSong, ProcessedResult, SearchResultArtist,
};
use crate::common::{
    AlbumType, Explicit, FeedbackTokenAddToLibrary, FeedbackTokenRemoveFromLibrary,
};
use crate::common::{PlaylistID, Thumbnail};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::process::{
    get_library_menu_from_menu, process_fixed_column_item, process_flex_column_item,
};
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

pub(crate) struct MusicShelfContents<'a> {
    pub json: JsonCrawlerBorrowed<'a>,
}
impl<'a> MusicShelfContents<'a> {
    pub fn from_crawler(crawler: JsonCrawlerBorrowed<'a>) -> Self {
        Self { json: crawler }
    }
}

fn take_music_shelf_contents(nav: &mut JsonCrawler) -> Result<MusicShelfContents> {
    let json = nav.borrow_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    Ok(MusicShelfContents { json })
}

impl<'a> ParseFrom<GetAlbumQuery<'a>> for AlbumParams {
    fn parse_from(
        p: ProcessedResult<GetAlbumQuery<'a>>,
    ) -> crate::Result<<GetAlbumQuery<'a> as Query>::Output> {
        parse_album_query_2024(p)
    }
}

fn parse_album_track_2024(json: &mut JsonCrawlerBorrowed) -> Result<AlbumSong> {
    let mut data = json.borrow_pointer(MRLIR)?;
    let title = super::parse_item_text(&mut data, 0, 0)?;
    let mut library_menu = get_library_menu_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
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
    let plays = process_flex_column_item(&mut data, 2)
        .and_then(|mut i| i.take_value_pointer(TEXT_RUN_TEXT))?;
    let track_no = str::parse(
        data.take_value_pointer::<String, &str>(concatcp!("/index", RUN_TEXT))?
            .as_str(),
    )
    // TODO: Better error
    .map_err(|e| Error::other(format!("Error {e} parsing into u64")))?;
    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    Ok(AlbumSong {
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
    })
}

// NOTE: Similar code to get_playlist_2024
fn parse_album_query_2024(p: ProcessedResult<GetAlbumQuery>) -> Result<AlbumParams> {
    let json_crawler = JsonCrawler::from(p);
    let mut columns = json_crawler.navigate_pointer(TWO_COLUMN)?;
    let mut header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER))?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    let category = AlbumType::try_from_str(
        header
            .take_value_pointer::<String, &str>(SUBTITLE)?
            .as_str(),
    )?;
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
            r.map(|mut r| r.take_value_pointer::<String, &str>("/text"))
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
        .map(|mut track| parse_album_track_2024(&mut track))
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
        YtMusic,
    };
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[tokio::test]
    async fn test_get_album_query() {
        let source_path = Path::new("./test_json/get_album_20240622.json");
        let expected_path = Path::new("./test_json/get_album_20240622_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        // Blank query has no bearing on function
        let query = GetAlbumQuery::new(AlbumID::from_raw("MPREb_Ylw2kL9wqcw"));
        let output = YtMusic::<BrowserToken>::process_json(source, query).unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
