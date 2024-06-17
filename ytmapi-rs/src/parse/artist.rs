use super::MusicShelfContents;
use super::ParsedSongAlbum;
use super::ProcessedResult;
use crate::common::youtuberesult::ResultCore;
use crate::common::youtuberesult::YoutubeResult;
use crate::common::AlbumID;
use crate::common::BrowseParams;
use crate::common::PlaylistID;
use crate::common::VideoID;
use crate::common::YoutubeID;
use crate::crawler::JsonCrawlerBorrowed;
use crate::nav_consts::*;
use crate::process::process_fixed_column_item;
use crate::query::*;
use crate::ChannelID;
use crate::Result;
use crate::Thumbnail;
use const_format::concatcp;

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

impl<'a> ProcessedResult<GetArtistQuery<'a>> {
    pub fn parse(self) -> Result<ArtistParams> {
        // TODO: Make this optional.
        let ProcessedResult {
            mut json_crawler, ..
        } = self;
        let mut results =
            json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        //        artist = {'description': None, 'views': None}
        let mut description = String::default();
        let mut views = String::default();
        //descriptionShelf = find_object_by_key(results, DESCRIPTION_SHELF[0], is_key=True)
        // XXX Functional way to take description:
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
        //        if 'musicShelfRenderer' in results[0]:  # API sometimes does not return songs
        //            musicShelf = nav(results[0], MUSIC_SHELF)
        //            if 'navigationEndpoint' in nav(musicShelf, TITLE):
        //                artist['songs']['browseId'] = nav(musicShelf, TITLE + NAVIGATION_BROWSE_ID)
        //            artist['songs']['results'] = parse_playlist_items(musicShelf['contents'])
        //            XXX: CPanics here
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
        // XXX: if there are multiple results for each category we only want to look at the
        // first one.
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
            let browse_id: Option<String> = r
                .take_value_pointer(concatcp!(CAROUSEL_TITLE, NAVIGATION_BROWSE_ID))
                .ok();
            // XXX should only be mandatory for albums, singles, playlists
            // as a result leaving as optional for now.
            let params = r
                .take_value_pointer::<String, &str>(concatcp!(
                    CAROUSEL_TITLE,
                    "/navigationEndpoint/browseEndpoint/params"
                ))
                .map(|params| BrowseParams::from_raw(params))
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
                        browse_id: browse_id.map(|id| AlbumID::from_raw(id)),
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
        //                artist[category]['results'] = parse_content_list(data[0]['contents'],
        //                                                                 categories_parser[i])
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
/// The browse_id and params can be used to get the full list of artist's albums.
/// If they aren't set, and results is not empty, assuming that all albums are displayed here already.
#[derive(Debug, Clone)]
pub struct GetArtistAlbums {
    pub results: Vec<AlbumResult>,
    // XXX: Unsure if AlbumID is correct here.
    pub browse_id: Option<AlbumID<'static>>,
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
    core: ResultCore,
}
#[derive(Debug, Clone)]
pub struct VideoResult {
    core: ResultCore,
}
impl YoutubeResult for VideoResult {
    fn get_core(&self) -> &ResultCore {
        &self.core
    }
}
impl YoutubeResult for AlbumResult {
    fn get_core(&self) -> &ResultCore {
        &self.core
    }
}

#[derive(Debug, Clone)]
// Could this alternatively be Result<Song>?
pub struct SongResult {
    core: ResultCore,
    video_id: VideoID<'static>,
    track_no: usize,
    album: Option<ParsedSongAlbum>,
}
impl YoutubeResult for SongResult {
    fn get_core(&self) -> &ResultCore {
        &self.core
    }
}
impl SongResult {
    pub fn get_video_id(&self) -> &VideoID<'static> {
        &self.video_id
    }
    pub fn get_album(&self) -> &Option<ParsedSongAlbum> {
        &self.album
    }
    pub fn get_track_no(&self) -> usize {
        self.track_no
    }
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
    let _year: Option<String> = navigator.take_value_pointer(SUBTITLE2).ok();
    let browse_id: String = navigator.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
    let thumbnails = navigator.take_value_pointer(THUMBNAIL_RENDERER)?;
    let is_explicit = navigator.path_exists(concatcp!(TITLE, SUBTITLE_BADGE_LABEL));
    let core = ResultCore::new(
        None,
        None,
        None,
        None,
        title,
        None,
        thumbnails,
        false,
        is_explicit,
        None,
        Some(ChannelID::from_raw(browse_id)),
        None,
        None,
    );
    Ok(AlbumResult { core })
}

//TODO: Menu entries
//TODO: Consider rename
pub(crate) fn parse_playlist_items(music_shelf: MusicShelfContents) -> Result<Vec<SongResult>> {
    let MusicShelfContents { json } = music_shelf;
    let mut results = Vec::new();
    // this should be set in each loop not here...
    for (i, result_json) in json.into_array_iter_mut().into_iter().flatten().enumerate() {
        let Ok(mut data) = result_json.navigate_pointer(MRLIR) else {
            continue;
        };
        let mut set_video_id = None;
        let mut video_id = String::default();
        let mut feedback_tok_add = None;
        let mut feedback_tok_remove = None;
        let mut like_status = None;

        // If the item has a menu, video_id will be here.
        if data.path_exists("/menu") {
            for mut item in data
                .borrow_pointer(MENU_ITEMS)?
                .into_array_iter_mut()
                .into_iter()
                .flatten()
            {
                if let Ok(mut menu_service) =
                    item.borrow_pointer(concatcp!(MENU_SERVICE, "/playlistEditEndpoint"))
                {
                    set_video_id = menu_service.take_value_pointer("/actions/0/setVideoId")?;
                    video_id = menu_service.take_value_pointer("/actions/0/removedVideoId")?;
                }
                if let Ok(mut toggle_menu) = item.navigate_pointer(TOGGLE_MENU) {
                    let library_add_token = toggle_menu
                        .take_value_pointer(concatcp!("/defaultServiceEndpoint", FEEDBACK_TOKEN))
                        .ok();
                    let library_remove_token = toggle_menu
                        .take_value_pointer(concatcp!("/toggledServiceEndpoint", FEEDBACK_TOKEN))
                        .ok();
                    let service_type =
                        toggle_menu.take_value_pointer::<String, &str>("/defaultIcon/iconType");
                    // Swap if already in library
                    if let Ok("LIBRARY_REMOVE") = service_type.as_deref() {
                        feedback_tok_add = library_remove_token;
                        feedback_tok_remove = library_add_token;
                    } else {
                        feedback_tok_add = library_add_token;
                        feedback_tok_remove = library_remove_token;
                    }
                }
            }
        }
        //   if item is not playable, the video_id was retrieved above
        if let Ok(mut p) = data.borrow_pointer(concatcp!(PLAY_BUTTON, "/playNavigationEndpoint")) {
            video_id = p.take_value_pointer("/watchEndpoint/videoId")?;
            if data.path_exists("/menu") {
                // Optional
                like_status = data.take_value_pointer(MENU_LIKE_STATUS).ok();
            }
        }
        let title = super::parse_item_text(&mut data, 0, 0)?;
        if title == "Song deleted" {
            continue;
        }

        // Artists may not exist, using an empty vector to represent this.
        // It depends on the query type so consider reflecting this in the code.
        // XXX: Consider which parts of this query are mandatory as currently erroring.
        // Using OK as a crutch to avoid error.
        let _artists = super::parse_song_artists(&mut data, 1)?;
        // Album may not exist, using an Option to reflect this.
        // It depends on the query type so consider reflecting this in the code.
        let album = super::parse_song_album(&mut data, 2).ok();
        let duration = if data.path_exists("/fixedColumns") {
            process_fixed_column_item(&mut data, 0).and_then(|mut i| {
                i.take_value_pointer("/text/simpleText")
                    .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
            })?
        } else {
            None
        };
        // Thumbnails is supposedly optional here, so we'll return an empty Vec if failed to find.
        // https://github.com/sigma67/ytmusicapi/blob/master/ytmusicapi/mixins/browsing.py#L231
        let thumbnails = data
            .take_value_pointer::<Vec<Thumbnail>, &str>(THUMBNAILS)
            .into_iter()
            .flatten()
            .collect();

        // XXX: test this
        let is_available = data
            .take_value_pointer::<String, &str>("/musicItemRendererDisplayPolicy")
            .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
            .unwrap_or(true);

        let is_explicit = data.path_exists(BADGE_LABEL);
        let video_type = data
            .take_value_pointer(concatcp!(
                MENU_ITEMS,
                "/0/menuNavigationItemRenderer/navigationEndpoint",
                NAVIGATION_VIDEO_TYPE
            ))
            .ok();

        let result = SongResult {
            core: ResultCore::new(
                set_video_id,
                duration,
                feedback_tok_add,
                feedback_tok_remove,
                title,
                like_status,
                thumbnails,
                is_available,
                is_explicit,
                video_type,
                None,
                None,
                None,
            ),
            album,
            video_id: VideoID::from_raw(video_id),
            // Need to add parsing for this.
            track_no: i + 1,
        };
        // TODO: Menu_entries
        //    if menu_entries:
        //        for menu_entry in menu_entries:
        //            song[menu_entry[-1]] = nav(data, MENU_ITEMS + menu_entry)
        //
        results.push(result);
    }
    Ok(results)
}

impl<'a> ProcessedResult<GetArtistAlbumsQuery<'a>> {
    pub fn parse(self) -> Result<Vec<crate::Album>> {
        let mut albums = Vec::new();
        let mut json_crawler = self.json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        for mut r in json_crawler
            .borrow_mut()
            .into_array_iter_mut()?
            .into_iter()
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
    use std::path::Path;

    use crate::{
        common::{BrowseParams, YoutubeID},
        crawler::JsonCrawler,
        parse::ProcessedResult,
        process::JsonCloner,
        query::GetArtistAlbumsQuery,
        ChannelID,
    };

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
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = GetArtistAlbumsQuery::new(ChannelID::from_raw(""), BrowseParams::from_raw(""));
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
