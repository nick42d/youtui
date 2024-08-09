use super::{
    parse_flex_column_item, parse_song_album, parse_song_artists, parse_upload_song_album,
    parse_upload_song_artists, EpisodeDate, EpisodeDuration, LibraryManager, LibraryStatus,
    LikeStatus, ParseFrom, ParsedSongAlbum, ParsedSongArtist, ParsedUploadArtist,
    ParsedUploadSongAlbum, ProcessedResult, SearchResultVideo, TableListUploadSong, Thumbnail,
};
use crate::{
    common::{
        AlbumID, AlbumType, BrowseParams, Explicit, FeedbackTokenAddToLibrary,
        FeedbackTokenRemoveFromLibrary, PlaylistID, UploadEntityID, VideoID,
    },
    crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator},
    nav_consts::*,
    process::{process_fixed_column_item, process_flex_column_item},
    query::*,
    youtube_enums::YoutubeMusicVideoType,
    ChannelID, Result,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

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

fn parse_artist_song(json: &mut JsonCrawlerBorrowed) -> Result<ArtistSong> {
    let mut data = json.borrow_pointer(MRLIR)?;
    let title = parse_flex_column_item(&mut data, 0, 0)?;
    let plays = parse_flex_column_item(&mut data, 2, 0)?;
    let artists = parse_song_artists(&mut data, 1)?;
    let album = parse_song_album(&mut data, 3)?;
    let video_id = data.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let mut library_menu = data
        .borrow_pointer(MENU_ITEMS)?
        .into_array_iter_mut()?
        .find_path("/toggleMenuServiceItemRenderer")?;
    let library_status = library_menu.take_value_pointer("/defaultIcon/iconType")?;
    let (feedback_tok_add_to_library, feedback_tok_rem_from_library) = match library_status {
        LibraryStatus::InLibrary => (
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
        ),
        LibraryStatus::NotInLibrary => (
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
        ),
    };
    Ok(ArtistSong {
        video_id,
        plays,
        album,
        artists,
        library_status,
        feedback_tok_add_to_library,
        feedback_tok_rem_from_library,
        title,
        like_status,
        explicit,
    })
}
fn parse_artist_songs(json: &mut JsonCrawlerBorrowed) -> Result<GetArtistSongs> {
    // Unsure if this should be optional or not.
    let browse_id = json.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
    let results = json
        .borrow_pointer("/contents")?
        .into_array_iter_mut()?
        .map(|mut item| parse_artist_song(&mut item))
        .collect::<Result<Vec<ArtistSong>>>()?;
    Ok(GetArtistSongs { results, browse_id })
}

impl<'a> ParseFrom<GetArtistQuery<'a>> for ArtistParams {
    // While this function gets improved, we'll allow this lint for the creation of
    // GetArtistTopReleases.
    #[allow(clippy::field_reassign_with_default)]
    fn parse_from(p: ProcessedResult<GetArtistQuery<'a>>) -> crate::Result<Self> {
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
        let mut top_releases = GetArtistTopReleases::default();
        top_releases.songs = results
            .borrow_pointer(concatcp!("/0", MUSIC_SHELF))
            .ok()
            .map(|mut j| parse_artist_songs(&mut j))
            .transpose()?;
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
                .take_value_pointer(concatcp!(
                    CAROUSEL_TITLE,
                    "/navigationEndpoint/browseEndpoint/params"
                ))
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
    pub results: Vec<ArtistSong>,
    pub browse_id: PlaylistID<'static>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct ArtistSong {
    pub video_id: VideoID<'static>,
    pub plays: String,
    pub album: ParsedSongAlbum,
    pub artists: Vec<ParsedSongArtist>,
    pub library_status: LibraryStatus,
    pub feedback_tok_add_to_library: FeedbackTokenAddToLibrary<'static>,
    pub feedback_tok_rem_from_library: FeedbackTokenRemoveFromLibrary<'static>,
    pub title: String,
    pub like_status: LikeStatus,
    pub explicit: Explicit,
}
#[derive(Debug, Clone)]
pub struct GetArtistVideos {
    pub results: Vec<SearchResultVideo>,
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
    pub library_status: LibraryStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Could this alternatively be Result<Song>?
// May need to be enum to track 'Not Available' case.
pub struct PlaylistSong {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub album: ParsedSongAlbum,
    pub duration: String,
    /// Some songs may not have library management features. There could be
    /// various resons for this.
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub artists: Vec<super::ParsedSongArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistVideo {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub title: String,
    // Could be 'ParsedVideoChannel'
    pub channel_name: String,
    pub channel_id: ChannelID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistEpisode {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub date: EpisodeDate,
    pub duration: EpisodeDuration,
    pub title: String,
    pub podcast_name: String,
    pub podcast_id: PlaylistID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub is_available: bool,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistUploadSong {
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub album: ParsedUploadSongAlbum,
    pub title: String,
    pub artists: Vec<ParsedUploadArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Could this alternatively be Result<Song>?
// May need to be enum to track 'Not Available' case.
// NOTE: Difference between this and PlaylistSong is no trackId.
pub struct TableListSong {
    pub video_id: VideoID<'static>,
    pub album: ParsedSongAlbum,
    pub duration: String,
    /// Some songs may not have library management features. There could be
    /// various resons for this.
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub artists: Vec<super::ParsedSongArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PlaylistItem {
    Song(PlaylistSong),
    Video(PlaylistVideo),
    Episode(PlaylistEpisode),
    UploadSong(PlaylistUploadSong),
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
    let explicit = if navigator.path_exists(concatcp!(SUBTITLE_BADGE_LABEL)) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let mut library_menu = navigator
        .borrow_pointer(MENU_ITEMS)?
        .into_array_iter_mut()?
        .find_path("/toggleMenuServiceItemRenderer")?;
    let library_status = library_menu.take_value_pointer("/defaultIcon/iconType")?;
    Ok(AlbumResult {
        title,
        album_type,
        year,
        album_id,
        library_status,
        thumbnails,
        explicit,
    })
}

pub(crate) fn parse_library_management_items_from_menu(
    menu: JsonCrawlerBorrowed,
) -> Result<Option<LibraryManager>> {
    let Ok(mut library_menu) = menu
        .into_array_iter_mut()?
        .find_path("/toggleMenuServiceItemRenderer")
    else {
        return Ok(None);
    };
    let status = library_menu.take_value_pointer("/defaultIcon/iconType")?;
    let (add_to_library_token, remove_from_library_token) = match status {
        LibraryStatus::InLibrary => (
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
        ),
        LibraryStatus::NotInLibrary => (
            library_menu.take_value_pointer(DEFAULT_ENDPOINT)?,
            library_menu.take_value_pointer(TOGGLED_ENDPOINT)?,
        ),
    };
    Ok(Some(LibraryManager {
        status,
        add_to_library_token,
        remove_from_library_token,
    }))
}

pub(crate) fn parse_playlist_song(
    title: String,
    track_no: usize,
    mut data: JsonCrawlerBorrowed,
) -> Result<PlaylistSong> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let library_management = data
        .borrow_pointer(MENU_ITEMS)
        .and_then(parse_library_management_items_from_menu)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let artists = super::parse_song_artists(&mut data, 1)?;
    let album = super::parse_song_album(&mut data, 2)?;
    let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
        i.take_value_pointer("/text/simpleText")
            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
    })?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);

    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(PlaylistSong {
        video_id,
        track_no,
        duration,
        library_management,
        title,
        artists,
        like_status,
        thumbnails,
        explicit,
        album,
        playlist_id,
        is_available,
    })
}
pub(crate) fn parse_playlist_upload_song(
    title: String,
    track_no: usize,
    mut data: JsonCrawlerBorrowed,
) -> Result<PlaylistUploadSong> {
    let duration =
        process_fixed_column_item(&mut data.borrow_mut(), 0)?.take_value_pointer(TEXT_RUN_TEXT)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchEndpoint/videoId"
    ))?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let artists = parse_upload_song_artists(data.borrow_mut(), 1)?;
    let album = parse_upload_song_album(data.borrow_mut(), 2)?;
    let mut menu = data.navigate_pointer(MENU_ITEMS)?;
    let entity_id = menu
        .as_array_iter_mut()?
        .find_path(DELETION_ENTITY_ID)?
        .take_value()?;
    Ok(PlaylistUploadSong {
        entity_id,
        video_id,
        album,
        duration,
        like_status,
        title,
        artists,
        thumbnails,
        track_no,
    })
}
pub(crate) fn parse_playlist_episode(
    title: String,
    track_no: usize,
    mut data: JsonCrawlerBorrowed,
) -> Result<PlaylistEpisode> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let is_live = data.path_exists(LIVE_BADGE_LABEL);
    let (duration, date) = match is_live {
        true => (EpisodeDuration::Live, EpisodeDate::Live),
        false => {
            let date = parse_flex_column_item(&mut data, 2, 0)?;
            let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
                i.take_value_pointer("/text/simpleText")
                    .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
            })?;
            (
                EpisodeDuration::Recorded { duration },
                EpisodeDate::Recorded { date },
            )
        }
    };
    let podcast_name = parse_flex_column_item(&mut data, 1, 0)?;
    let podcast_id = process_flex_column_item(&mut data, 1)?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    Ok(PlaylistEpisode {
        video_id,
        duration,
        title,
        like_status,
        thumbnails,
        date,
        podcast_name,
        podcast_id,
        is_available,
        track_no,
    })
}
pub(crate) fn parse_playlist_video(
    title: String,
    track_no: usize,
    mut data: JsonCrawlerBorrowed,
) -> Result<PlaylistVideo> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let channel_name = parse_flex_column_item(&mut data, 1, 0)?;
    let channel_id = process_flex_column_item(&mut data, 1)?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
        i.take_value_pointer("/text/simpleText")
            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
    })?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);

    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(PlaylistVideo {
        video_id,
        track_no,
        duration,
        title,
        like_status,
        thumbnails,
        playlist_id,
        is_available,
        channel_name,
        channel_id,
    })
}

pub(crate) fn parse_playlist_item(
    track_no: usize,
    json: &mut JsonCrawlerBorrowed,
) -> Result<Option<PlaylistItem>> {
    let Ok(mut data) = json.borrow_pointer(MRLIR) else {
        return Ok(None);
    };
    let title = super::parse_flex_column_item(&mut data, 0, 0)?;
    if title == "Song deleted" {
        return Ok(None);
    }
    let video_type_path = concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        NAVIGATION_VIDEO_TYPE
    );
    let video_type: YoutubeMusicVideoType = data.take_value_pointer(video_type_path)?;
    // TODO: Deserialize to enum
    let item = match video_type {
        YoutubeMusicVideoType::Ugc | YoutubeMusicVideoType::Omv => Some(PlaylistItem::Video(
            parse_playlist_video(title, track_no, data)?,
        )),
        YoutubeMusicVideoType::Atv => Some(PlaylistItem::Song(parse_playlist_song(
            title, track_no, data,
        )?)),
        YoutubeMusicVideoType::Upload => Some(PlaylistItem::UploadSong(
            parse_playlist_upload_song(title, track_no, data)?,
        )),
        YoutubeMusicVideoType::Episode => Some(PlaylistItem::Episode(parse_playlist_episode(
            title, track_no, data,
        )?)),
    };
    Ok(item)
}
//TODO: Menu entries
//TODO: Consider rename
pub(crate) fn parse_playlist_items(json: JsonCrawlerBorrowed) -> Result<Vec<PlaylistItem>> {
    json.into_array_iter_mut()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(idx, mut item)| parse_playlist_item(idx + 1, &mut item).transpose())
        .collect()
}
impl<'a> ParseFrom<GetArtistAlbumsQuery<'a>> for Vec<crate::Album> {
    fn parse_from(p: ProcessedResult<GetArtistAlbumsQuery<'a>>) -> crate::Result<Self> {
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
        ChannelID,
    };

    #[tokio::test]
    async fn test_get_artist_albums_query() {
        parse_test!(
            // Radiohead's albums.
            "./test_json/browse_artist_albums.json",
            "./test_json/browse_artist_albums_output.txt",
            GetArtistAlbumsQuery::new(ChannelID::from_raw(""), BrowseParams::from_raw("")),
            BrowserToken
        );
    }

    #[tokio::test]
    async fn test_get_artist() {
        parse_test!(
            "./test_json/get_artist_20240705.json",
            "./test_json/get_artist_20240705_output.txt",
            crate::query::GetArtistQuery::new(ChannelID::from_raw("")),
            BrowserToken
        );
    }
}
