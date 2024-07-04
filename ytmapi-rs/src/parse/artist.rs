use super::LibraryStatus;
use super::MusicShelfContents;
use super::ParseFrom;
use super::ParsedSongAlbum;
use super::ProcessedResult;
use super::SongLikeStatus;
use crate::common::{
    youtuberesult::{ResultCore, YoutubeResult},
    AlbumID, AlbumType, BrowseParams, Explicit, FeedbackTokenAddToLibrary,
    FeedbackTokenRemoveFromLibrary, PlaylistID, SetVideoID, VideoID, YoutubeID,
};
use crate::crawler::JsonCrawler;
use crate::crawler::JsonCrawlerBorrowed;
use crate::nav_consts::*;
use crate::process::process_fixed_column_item;
use crate::query::*;
use crate::ChannelID;
use crate::Error;
use crate::Result;
use crate::Thumbnail;
use const_format::concatcp;
use serde::de::value::UsizeDeserializer;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ArtistParams {
    pub description: String,
    pub views: String,
    pub name: String,
    pub channel_id: String,
    pub shuffle_id: Option<String>,
    pub radio_id: Option<String>,
    pub subscribers: Option<String>,
    pub subscribed: Option<String>,
    pub thumbnails: Option<String>,
    pub top_releases: GetArtistTopReleases,
}

impl<'a> ParseFrom<GetArtistQuery<'a>> for ArtistParams {
    fn parse_from(
        p: ProcessedResult<GetArtistQuery<'a>>,
    ) -> crate::Result<<GetArtistQuery<'a> as Query>::Output> {
        // TODO: Make this optional.
        let mut json_crawler: JsonCrawler = p.into();
        let mut results =
            json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        //        artist = {'description': None, 'views': None}
        let mut description = String::default();
        let mut views = String::default();
        //descriptionShelf = find_object_by_key(results, DESCRIPTION_SHELF[0],
        // is_key=True) XXX Functional way to take description:
        // let x: String = results
        //     .as_array_iter_mut()
        //     .map(|mut r| {
        //         r.find_map(|a| a.navigate_pointer(DESCRIPTION_SHELF).ok())
        //             .and_then(|mut d| d.take_value_pointer(DESCRIPTION).ok())
        //     })
        //     .unwrap_or(Some(String::new()))
        //     .unwrap_or(String::new());
        if let Ok(results_array) = results.as_array_iter_mut() {
            for r in results_array {
                if let Ok(mut description_shelf) = r.navigate_pointer(DESCRIPTION_SHELF) {
                    description = description_shelf.take_value_pointer(DESCRIPTION)?;
                    if let Ok(mut subheader) = description_shelf.borrow_pointer("/subheader") {
                        views = subheader.take_value_pointer("/runs/0/text")?;
                    }
                    break;
                }
            }
        }
        //        if 'musicShelfRenderer' in results[0]:  # API sometimes does not
        // return songs            musicShelf = nav(results[0], MUSIC_SHELF)
        //            if 'navigationEndpoint' in nav(musicShelf, TITLE):
        //                artist['songs']['browseId'] = nav(musicShelf, TITLE +
        // NAVIGATION_BROWSE_ID)            artist['songs']['results'] =
        // parse_playlist_items(musicShelf['contents'])            XXX: CPanics
        // here
        let mut top_releases = GetArtistTopReleases::default();
        if results.path_exists("/0/musicShelfRenderer") {
            if let Ok(mut music_shelf) = results.borrow_pointer(concatcp!("/0", MUSIC_SHELF)) {
                // Unsure if this should be optional or not.
                let browse_id = music_shelf
                    .take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))
                    .map(|b_id: String| PlaylistID::from_raw(b_id))?;
                let music_shelf_contents =
                    MusicShelfContents::from_crawler(music_shelf.navigate_pointer("/contents")?);
                let results = parse_playlist_items(music_shelf_contents)?;
                let songs = GetArtistSongs { results, browse_id };
                top_releases.songs = Some(songs);
            }
        }
        // TODO: Check if Carousel Title is in list of categories.
        // TODO: Actually pass these variables in the return
        // XXX: Looks to be two loops over results here.
        // XXX: if there are multiple results for each category we only want to look at
        // the first one.
        for mut r in results
            .as_array_iter_mut()
            .into_iter()
            .flatten()
            .filter_map(|r| r.navigate_pointer("/musicCarouselShelfRenderer").ok())
        {
            // XXX: Should this only be on the first result per category?
            let category = ArtistTopReleaseCategory::from_string(
                r.take_value_pointer(concatcp!(CAROUSEL_TITLE, "/text"))?,
            );
            // Likely optional, need to confirm.
            // XXX: Errors here
            let browse_id: Option<ChannelID> = r
                .take_value_pointer(concatcp!(CAROUSEL_TITLE, NAVIGATION_BROWSE_ID))
                .ok();
            // XXX should only be mandatory for albums, singles, playlists
            // as a result leaving as optional for now.
            let params = r
                .take_value_pointer::<String, &str>(concatcp!(
                    CAROUSEL_TITLE,
                    "/navigationEndpoint/browseEndpoint/params"
                ))
                .map(BrowseParams::from_raw)
                .ok();
            // TODO: finish other categories
            match category {
                ArtistTopReleaseCategory::Related => (),
                ArtistTopReleaseCategory::Videos => (),
                ArtistTopReleaseCategory::Singles => (),
                ArtistTopReleaseCategory::Albums => {
                    let mut results = Vec::new();
                    for i in r.navigate_pointer("/contents")?.as_array_iter_mut()? {
                        results.push(parse_album_from_mtrir(i.navigate_pointer(MTRIR)?)?);
                    }
                    let albums = GetArtistAlbums {
                        browse_id,
                        params,
                        results,
                    };
                    top_releases.albums = Some(albums);
                }
                ArtistTopReleaseCategory::Playlists => todo!(),
                ArtistTopReleaseCategory::None => (),
            }
        }
        // Assume header exists, assumption may be incorrect.
        // I think Json is owned by someone else here?
        // I think I can do another self.get_navigable()
        let mut header = json_crawler.navigate_pointer("/header/musicImmersiveHeaderRenderer")?;
        let name = header.take_value_pointer(TITLE_TEXT)?;
        let shuffle_id = header
            .take_value_pointer(concatcp!(
                "/playButton/buttonRenderer",
                NAVIGATION_WATCH_PLAYLIST_ID
            ))
            .ok();
        let radio_id = header
            .take_value_pointer(concatcp!(
                "/startRadioButton/buttonRenderer",
                NAVIGATION_WATCH_PLAYLIST_ID
            ))
            .ok();
        // TODO: Validate if this could instead be returned as a Thumbnails struct.
        let thumbnails = header.take_value_pointer(THUMBNAILS).ok();
        // Assume subscription button exists, assumption may not be correct.
        let mut subscription_button =
            header.navigate_pointer("/subscriptionButton/subscribeButtonRenderer")?;
        let channel_id = subscription_button.take_value_pointer("/channelId")?;
        let subscribers = subscription_button
            .take_value_pointer("/subscriberCountText/runs/0/text")
            .ok();
        // XXX: Unsure if this is optional. It errors currently, removed the ?.
        let subscribed = subscription_button.take_value_pointer("/subscribed").ok();
        //                artist[category]['results'] =
        // parse_content_list(data[0]['contents'],
        // categories_parser[i])
        Ok(ArtistParams {
            views,
            description,
            name,
            top_releases,
            thumbnails,
            subscribed,
            radio_id,
            channel_id,
            shuffle_id,
            subscribers,
        })
    }
}
#[derive(Debug, Clone, Default)]
pub struct GetArtistTopReleases {
    pub songs: Option<GetArtistSongs>,
    pub albums: Option<GetArtistAlbums>,
    pub singles: Option<GetArtistAlbums>,
    pub videos: Option<GetArtistVideos>,
    pub related: Option<GetArtistRelated>,
}
#[derive(Debug, Clone)]
pub struct GetArtistRelated {
    pub results: Vec<RelatedResult>,
}
#[derive(Debug, Clone)]
pub struct GetArtistSongs {
    pub results: Vec<SongResult>,
    pub browse_id: PlaylistID<'static>,
}
#[derive(Debug, Clone)]
pub struct GetArtistVideos {
    pub results: Vec<VideoResult>,
    pub browse_id: PlaylistID<'static>,
}
/// The Albums section of the Browse Artist page.
/// The browse_id and params can be used to get the full list of artist's
/// albums. If they aren't set, and results is not empty, you can assume that
/// all albums are displayed here already.
#[derive(Debug, Clone)]
pub struct GetArtistAlbums {
    pub results: Vec<AlbumResult>,
    // XXX: Unsure if AlbumID is correct here.
    pub browse_id: Option<ChannelID<'static>>,
    pub params: Option<BrowseParams<'static>>,
}
#[derive(Debug, Clone)]
pub struct RelatedResult {
    pub browse_id: ChannelID<'static>,
    pub title: String,
    pub subscribers: String,
}
#[derive(Debug, Clone)]
pub struct AlbumResult {
    pub title: String,
    pub album_type: AlbumType,
    pub year: String,
    pub album_id: AlbumID<'static>,
    pub feedback_tok_add: FeedbackTokenAddToLibrary<'static>,
    pub feedback_tok_rem: FeedbackTokenRemoveFromLibrary<'static>,
    pub library_status: LibraryStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
}
// pub struct AlbumResult {
//     core: ResultCore,
// }
#[derive(Debug, Clone)]
pub struct VideoResult {
    core: ResultCore,
}
impl YoutubeResult for VideoResult {
    fn get_core(&self) -> &ResultCore {
        &self.core
    }
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Could this alternatively be Result<Song>?
// May need to be enum to track 'Not Available' case.
pub struct SongResult {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub album: ParsedSongAlbum,
    pub duration: String,
    pub library_status: LibraryStatus,
    pub feedback_tok_add: FeedbackTokenAddToLibrary<'static>,
    pub feedback_tok_rem: FeedbackTokenRemoveFromLibrary<'static>,
    pub title: String,
    pub artists: Vec<super::ParsedSongArtist>,
    pub like_status: SongLikeStatus,
    pub thumbnails: Vec<super::Thumbnail>,
    pub explicit: Explicit,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

// Should be at higher level in mod structure.
#[derive(Debug)]
enum ArtistTopReleaseCategory {
    Albums,
    Singles,
    Videos,
    Playlists,
    Related,
    None,
}
// May not need this.
// XXX: remove?
impl ArtistTopReleaseCategory {
    // Note, this only works with user lang set to english.
    // TODO: Implement i18n.
    pub fn from_string(str: String) -> ArtistTopReleaseCategory {
        match str.to_lowercase().as_str() {
            "albums" => ArtistTopReleaseCategory::Albums,
            "singles" => ArtistTopReleaseCategory::Singles,
            "videos" => ArtistTopReleaseCategory::Videos,
            "playlists" => ArtistTopReleaseCategory::Playlists,
            "fans might also like" => ArtistTopReleaseCategory::Related,
            _ => ArtistTopReleaseCategory::None,
        }
    }
}
pub(crate) fn parse_album_from_mtrir(mut navigator: JsonCrawlerBorrowed) -> Result<AlbumResult> {
    let title = navigator.take_value_pointer(TITLE_TEXT)?;
    let album_type = navigator.take_value_pointer(SUBTITLE)?;
    let year = navigator.take_value_pointer(SUBTITLE2)?;
    let album_id = navigator.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
    let thumbnails = navigator.take_value_pointer(THUMBNAIL_RENDERER)?;
    let explicit = if navigator.path_exists(concatcp!(TITLE, SUBTITLE_BADGE_LABEL)) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let mut library_menu = navigator.navigate_pointer(concatcp!(MENU_ITEMS, "/4"))?;
    let library_status =
        library_menu.take_value_pointer("/toggledMenuServiceItemRenderer/defaultIcon/iconType")?;
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
    Ok(AlbumResult {
        title,
        album_type,
        year,
        album_id,
        feedback_tok_add,
        feedback_tok_rem,
        library_status,
        thumbnails,
        explicit,
    })
}

pub(crate) fn parse_playlist_item(
    track_no: usize,
    json: &mut JsonCrawlerBorrowed,
) -> Result<Option<SongResult>> {
    let Ok(mut data) = json.borrow_pointer(MRLIR) else {
        return Ok(None);
    };
    let title = super::parse_item_text(&mut data, 0, 0)?;
    if title == "Song deleted" {
        return Ok(None);
    }
    let mut library_menu = data
        .borrow_pointer(MENU_ITEMS)?
        .into_array_iter_mut()?
        .find_map(|item| {
            item.navigate_pointer("/toggledMenuServiceItemRenderer")
                .ok()
        })
        // Future function try_map() will potentially eliminate this ok->ok_or_else combo.
        .ok_or_else(|| {
            Error::other("expected playlist item to contain a /toggledMenuServiceItemRenderer")
        })?;
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
    let artists = super::parse_song_artists(&mut data, 1)?;
    let album = super::parse_song_album(&mut data, 2)?;
    let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
        i.take_value_pointer("/text/simpleText")
            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
    })?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String, &str>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);

    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer/navigationEndpoint",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(Some(SongResult {
        video_id,
        // Need to add parsing for this.
        track_no,
        duration,
        feedback_tok_add,
        feedback_tok_rem,
        title,
        artists,
        like_status,
        thumbnails,
        explicit,
        library_status,
        album,
        playlist_id,
        is_available,
    }))
}
//TODO: Menu entries
//TODO: Consider rename
pub(crate) fn parse_playlist_items(music_shelf: MusicShelfContents) -> Result<Vec<SongResult>> {
    let MusicShelfContents { json } = music_shelf;
    json.into_array_iter_mut()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(idx, mut item)| parse_playlist_item(idx, &mut item).transpose())
        .collect()
}
impl<'a> ParseFrom<GetArtistAlbumsQuery<'a>> for Vec<crate::Album> {
    fn parse_from(
        p: ProcessedResult<GetArtistAlbumsQuery<'a>>,
    ) -> crate::Result<<GetArtistAlbumsQuery<'a> as Query>::Output> {
        let json_crawler: JsonCrawler = p.into();
        let mut albums = Vec::new();
        let mut json_crawler = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        for mut r in json_crawler
            .borrow_mut()
            .into_array_iter_mut()?
            .flat_map(|i| i.navigate_pointer(MTRIR))
        {
            let browse_id = r.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
            let playlist_id = r.take_value_pointer(MENU_PLAYLIST_ID).ok();
            let title = r.take_value_pointer(TITLE_TEXT)?;
            let thumbnails = r.take_value_pointer(THUMBNAIL_RENDERER)?;
            // TODO: category
            let category = r.take_value_pointer(SUBTITLE).ok();
            albums.push(crate::Album {
                browse_id,
                year: None,
                title,
                category,
                thumbnails,
                playlist_id,
            });
        }
        Ok(albums)
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{BrowseParams, YoutubeID},
        query::GetArtistAlbumsQuery,
        ChannelID, YtMusic,
    };
    use std::path::Path;

    #[tokio::test]
    async fn test_get_albums_query() {
        // Radiohead's albums.
        let source_path = Path::new("./test_json/browse_artist_albums.json");
        let expected_path = Path::new("./test_json/browse_artist_albums_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        // Blank query has no bearing on function
        let query = GetArtistAlbumsQuery::new(ChannelID::from_raw(""), BrowseParams::from_raw(""));
        let output = YtMusic::<BrowserToken>::process_json(source, query).unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
