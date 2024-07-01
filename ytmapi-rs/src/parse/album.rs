use crate::common::{AlbumType, Explicit};
use crate::common::{PlaylistID, Thumbnail};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::*;
use crate::query::*;
use crate::{Error, Result};
use const_format::concatcp;

use super::{parse_playlist_items, ParseFrom, ProcessedResult, SongResult};

#[derive(Debug)]
pub enum AlbumLikeStatus {
    Like,
    Indifferent,
}

#[derive(Debug)]
pub struct AlbumParamsOtherVersion {
    pub title: String,
    pub year: String,
    pub browse_id: String,
    pub thumbnails: Vec<Thumbnail>,
    pub is_explicit: Explicit,
}

// Is this similar to another struct?
// XXX: Consider correct privacy
#[derive(Debug)]
pub struct AlbumParams {
    pub title: String,
    pub category: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
    pub description: Option<String>,
    pub artists: Option<String>, // Should be super::ParsedSongArtist<'a>, // Basic Artists
    pub year: String,
    pub track_count_text: Option<String>,
    pub duration: String,
    pub audio_playlist_id: Option<PlaylistID<'static>>,
    // TODO: better interface
    pub tracks: Vec<SongResult>,
    //consider moving this struct up to super.
    pub other_versions: Option<Vec<AlbumParamsOtherVersion>>,
    pub like_status: Option<AlbumLikeStatus>,
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
        // Google API changing. Old version indicated by existance of HEADER_DETAIL key,
        // otherwise new version.
        if p.get_json().pointer(HEADER_DETAIL).is_some() {
            parse_album_query(p)
        } else {
            parse_album_query_2024(p)
        }
    }
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
    let artists = Some(header.take_value_pointer(STRAPLINE_TEXT)?);
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
    let music_shelf = MusicShelfContents {
        json: columns.borrow_pointer(
            "/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents",
        )?,
    };
    let tracks = parse_playlist_items(music_shelf)?;
    Ok(AlbumParams {
        // TODO
        like_status: None,
        title,
        description,
        thumbnails,
        duration,
        category,
        track_count_text,
        audio_playlist_id,
        // TODO
        other_versions: None,
        year,
        tracks,
        artists,
    })
}

fn parse_album_query(p: ProcessedResult<GetAlbumQuery>) -> Result<AlbumParams> {
    let mut json_crawler = JsonCrawler::from(p);
    let mut header = json_crawler.borrow_pointer(HEADER_DETAIL)?;
    let title = header.take_value_pointer(TITLE_TEXT)?;
    // I am not sure why the error here is OK but I'll take it!
    let category = AlbumType::try_from_str(header.take_value_pointer::<String, &str>(SUBTITLE)?)?;
    let description = header.take_value_pointer("/description/runs/0/text").ok();
    let thumbnails = header.take_value_pointer(THUMBNAIL_CROPPED)?;
    // If NAVIGATION_WATCH_PLAYLIST ID, then return that, else try
    // NAVIGATION_PLAYLIST_ID else None.
    // Seems a bit of a hacky way to do this.
    let mut top_level = header.borrow_pointer(concatcp!(MENU, "/topLevelButtons"))?;
    let audio_playlist_id = top_level
        .take_value_pointer(concatcp!("/0/buttonRenderer", NAVIGATION_WATCH_PLAYLIST_ID))
        .or_else(|_| {
            top_level.take_value_pointer(concatcp!("/0/buttonRenderer", NAVIGATION_PLAYLIST_ID))
        })
        .ok();
    // TODO: parsing function
    let like_status = top_level
        .take_value_pointer("/1/buttonRenderer/defaultServiceEndpoint/likeEndpoint/status")
        .ok()
        .map(|likestatus| match likestatus {
            1 => Ok(AlbumLikeStatus::Like),
            2 => Ok(AlbumLikeStatus::Indifferent),
            other => Err(crate::Error::other(format!(
                "Received likestatus {}, but expected only \"1\" or \"2\"",
                other
            ))),
        })
        .transpose()?;
    // Based on code from ytmusicapi (python)
    let track_count = header
        .borrow_pointer("/secondSubtitle/runs")
        .ok()
        .and_then(|s| s.into_array_iter_mut().ok())
        .and_then(|mut a| {
            if a.len() > 1 {
                a.next()
                    .and_then(|mut v| v.take_value_pointer("/text").ok())
            } else {
                None
            }
        });
    let duration = header
        .borrow_pointer("/secondSubtitle/runs")
        .ok()
        .and_then(|s| s.into_array_iter_mut().ok())
        .and_then(|mut a| {
            if a.len() > 1 {
                a.nth(2)
                    .and_then(|mut v| v.take_value_pointer("/text").ok())
            } else {
                a.next()
                    .and_then(|mut v| v.take_value_pointer("/text").ok())
            }
        })
        .ok_or_else(|| Error::other("Basic error on duration"))?;
    let mut year = String::new();
    // Pretty hacky way to handle this, as the runs are quite free text.
    // TODO: Add a regex crate.
    // NOTE: See the search parser for a better way to implement.
    for mut a in header
        .navigate_pointer("/subtitle/runs")
        .and_then(|s| s.into_array_iter_mut())
        .into_iter()
        .flatten()
        .skip(2)
        .step_by(2)
    {
        let value: Result<String> = a.take_value_pointer("/text");
        if let Ok(4) = value.as_ref().map(|v| v.len()) {
            year = value.unwrap();
        }
    }
    let _results_other_versions = json_crawler.borrow_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST,
        "/0",
        MUSIC_SHELF
    )); //this can be none.
    let music_shelf = take_music_shelf_contents(&mut json_crawler)?;
    let tracks = parse_playlist_items(music_shelf)?;
    //let mut tracks = super::artist::parse_playlist_items(results_tracks.take())?;
    // Tracks themselves don't know who the album artist is. But it can be handy for
    // other parts of the application to know the artist.
    // This may not be the ideal approach due to the allocation requirement but
    // nevertheless we are using it for now.
    // TODO: Consider alternative approach in the app design.
    //for track in tracks.iter_mut() {
    //    track.album = Some(super::ParsedSongAlbum {
    //        id: audio_playlist_id.clone(),
    //        name: Some(title.clone()),
    //    });
    //}

    //        album = parse_album_header(response)
    //        results = nav(response, SINGLE_COLUMN_TAB + SECTION_LIST_ITEM +
    // MUSIC_SHELF)        album['tracks'] =
    // parse_playlist_items(results['contents'])        results = nav(response,
    // SINGLE_COLUMN_TAB + SECTION_LIST + [1] + CAROUSEL, True)
    //        if results is not None:
    //            album['other_versions'] = parse_content_list(results['contents'],
    // parse_album)        album['duration_seconds'] = sum_total_duration(album)
    //        for i, track in enumerate(album['tracks']):
    //            album['tracks'][i]['album'] = album['title']
    //            album['tracks'][i]['artists'] = album['artists']
    //
    //        return album
    Ok(AlbumParams {
        like_status,
        title,
        description,
        thumbnails,
        duration,
        category,
        track_count_text: track_count,
        audio_playlist_id,
        other_versions: None,
        year,
        tracks,
        artists: None,
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
