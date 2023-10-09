use crate::common::Thumbnail; //XXX: Move this to parse?
use crate::common::{AlbumType, Explicit};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::*;
use crate::query::*;
use crate::{Error, Result};
use const_format::concatcp;

use super::{parse_playlist_items, ProcessedResult, SongResult};

#[derive(Debug)]
enum AlbumLikeStatus {
    Like,
    Indifferent,
}

#[derive(Debug)]
pub struct AlbumParamsOtherVersion {
    title: String,
    year: String,
    browse_id: String,
    thumbnails: Vec<Thumbnail>,
    is_explicit: Explicit,
}

// Is this similar to another struct?
// XXX: Consider correct privacy
#[derive(Debug)]
pub struct AlbumParams {
    title: String,
    category: AlbumType,
    thumbnails: Vec<Thumbnail>,
    description: Option<String>,
    artists: Option<String>, // Should be super::ParsedSongArtist<'a>, // Basic Artists
    pub year: String,
    track_count: Option<u64>,
    duration: String,
    audio_playlist_id: Option<String>,
    // TODO: better interface
    pub tracks: Vec<SongResult>, //consider moving this struct up to
    //super.
    other_versions: Option<Vec<AlbumParamsOtherVersion>>,
    like_status: Option<AlbumLikeStatus>,
}

pub struct MusicShelfContents<'a> {
    pub json: JsonCrawlerBorrowed<'a>,
}
impl<'a, 'b> MusicShelfContents<'a> {
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

impl<'a> ProcessedResult<GetAlbumQuery<'a>> {
    pub fn parse(self) -> Result<AlbumParams> {
        // Due to limitation of the borrow checker, we can't simply pass a reference
        // to ok_or_else. So instead, we'll keep a clone handy in case of error.
        // The advantage of this approach is that the entire json
        // for the function is stored.
        // XXX: Consider adding code here so this only runs in debug mode.
        // TODO: Implement pointer trace so that we can see exactly where error occurs.
        // TODO: Allow error composition - so that an error in the parsing function
        // also reports enter json_debug file.
        let ProcessedResult {
            mut json_crawler, ..
        } = self;
        // TODO parse_song_runs - returns id, views and a few others.
        // Other verisions = parse_content_list.
        // Fill in Tracks album title and artist (not sure if needed).
        let mut header = json_crawler.borrow_pointer(HEADER_DETAIL)?;
        // If this fails, try TryInto.
        // Type annotation is required because I use title before its used as a struct field.
        let title: String = header.take_value_pointer(TITLE_TEXT)?;
        // I am not sure why the error here is OK but I'll take it!
        let category = AlbumType::try_from(
            header
                .take_value_pointer::<String, &str>(SUBTITLE)?
                .as_str(),
        )?;
        let description = header.take_value_pointer("/description/runs/0/text").ok();
        let thumbnails = super::parse_thumbnails(&mut header.borrow_pointer(THUMBNAIL_CROPPED)?)?;
        // If NAVIGATION_WATCH_PLAYLIST ID, then return that, else try NAVIGATION_PLAYLIST_ID else
        // None.
        // Seems a bit of a hacky way to do this.
        // XXX: This is an issue! Clone inserted to make compile.
        // TODO: Remove allocation.
        // If we clone in this way, we won't have the parent json or path.
        let mut top_level = header.borrow_pointer(concatcp!(MENU, "/topLevelButtons"))?;
        let audio_playlist_id = if let Ok(value) = top_level
            .take_value_pointer(concatcp!("/0/buttonRenderer", NAVIGATION_WATCH_PLAYLIST_ID))
        {
            Some(value)
        } else {
            top_level
                .take_value_pointer(concatcp!("/0/buttonRenderer", NAVIGATION_PLAYLIST_ID))
                .ok()
        };
        // TODO: Error instead of panic
        // TODO: Improve this
        let like_status = top_level
            .take_value_pointer("/1/buttonRenderer/defaultServiceEndpoint/likeEndpoint/status")
            .map(|likestatus| match likestatus {
                1 => AlbumLikeStatus::Like,
                2 => AlbumLikeStatus::Indifferent,
                _ => unreachable!("likestatus should only be 1 or 2"),
            })
            .ok();
        // Original python code:
        //    if len(header['secondSubtitle']['runs']) > 1:
        //        album['trackCount'] = to_int(header['secondSubtitle']['runs'][0]['text'])
        //        album['duration'] = header['secondSubtitle']['runs'][2]['text']
        //    else:
        //        album['duration'] = header['secondSubtitle']['runs'][0]['text']
        //  Below avoid mutable variables but looks messy & appears to be inefficient.
        //  Do I actually need this, when it can be calculated?
        // Should be a better way to do this - potentially if-let.
        // XXX: May be able to remove additional OKs for these.
        let track_count = header
            .borrow_pointer("/secondSubtitle/runs")
            .ok()
            .and_then(|s| s.into_array_iter_mut().ok())
            .and_then(|mut a| {
                if a.len() > 1 {
                    a.nth(0)
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
                    a.nth(0)
                        .and_then(|mut v| v.take_value_pointer("/text").ok())
                }
            })
            .ok_or_else(|| Error::other("Basic error on duration"))?;
        let mut year = String::new();
        // Pretty hacky way to handle this, as the runs are quite free text.
        // TODO: Add a regex crate.
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
        // Tracks themselves don't know who the album artist is. But it can be handy for other
        // parts of the application to know the artist.
        // This may not be the ideal approach due to the allocation requirement but nevertheless we
        // are using it for now.
        // TODO: Consider alternative approach in the app design.
        //for track in tracks.iter_mut() {
        //    track.album = Some(super::ParsedSongAlbum {
        //        id: audio_playlist_id.clone(),
        //        name: Some(title.clone()),
        //    });
        //}

        //        album = parse_album_header(response)
        //        results = nav(response, SINGLE_COLUMN_TAB + SECTION_LIST_ITEM + MUSIC_SHELF)
        //        album['tracks'] = parse_playlist_items(results['contents'])
        //        results = nav(response, SINGLE_COLUMN_TAB + SECTION_LIST + [1] + CAROUSEL, True)
        //        if results is not None:
        //            album['other_versions'] = parse_content_list(results['contents'], parse_album)
        //        album['duration_seconds'] = sum_total_duration(album)
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
            track_count,
            audio_playlist_id,
            other_versions: None,
            year,
            tracks,
            artists: None,
        })
    }
}
