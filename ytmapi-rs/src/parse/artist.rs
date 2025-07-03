use super::search::SearchResultVideo;
use super::{
    parse_flex_column_item, parse_song_album,
    parse_song_artists, ParseFrom, ParsedSongAlbum, ParsedSongArtist, ProcessedResult, Thumbnail,
};
use crate::common::{
    AlbumID, AlbumType, ArtistChannelID, BrowseParams, Explicit,
    LibraryManager, LibraryStatus, LikeStatus, PlaylistID, VideoID,
};
use crate::nav_consts::*;
use crate::query::*;
use crate::Result;
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GetArtistAlbumsAlbum {
    pub title: String,
    // TODO: Use type system
    pub playlist_id: Option<String>,
    // TODO: Use type system
    pub browse_id: AlbumID<'static>,
    pub category: Option<String>, // TODO change to enum
    pub thumbnails: Vec<Thumbnail>,
    pub year: Option<String>,
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
    let library_management =
        parse_library_management_items_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
    Ok(ArtistSong {
        video_id,
        plays,
        album,
        artists,
        library_management,
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
        .try_into_iter()?
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
        let mut json_crawler: JsonCrawlerOwned = p.into();
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
        if let Ok(results_array) = results.try_iter_mut() {
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
            .try_iter_mut()
            .into_iter()
            .flatten()
            .filter_map(|r| r.navigate_pointer("/musicCarouselShelfRenderer").ok())
        {
            // XXX: Should this only be on the first result per category?
            let category = r.take_value_pointer(concatcp!(CAROUSEL_TITLE, "/text"))?;
            // Likely optional, need to confirm.
            // XXX: Errors here
            let browse_id: Option<ArtistChannelID> = r
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
                    for i in r.navigate_pointer("/contents")?.try_iter_mut()? {
                        results.push(parse_album_from_mtrir(i.navigate_pointer(MTRIR)?)?);
                    }
                    let albums = GetArtistAlbums {
                        browse_id,
                        params,
                        results,
                    };
                    top_releases.albums = Some(albums);
                }
                ArtistTopReleaseCategory::Playlists => (),
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct GetArtistTopReleases {
    pub songs: Option<GetArtistSongs>,
    pub albums: Option<GetArtistAlbums>,
    pub singles: Option<GetArtistAlbums>,
    pub videos: Option<GetArtistVideos>,
    pub related: Option<GetArtistRelated>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct GetArtistRelated {
    pub results: Vec<RelatedResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct GetArtistSongs {
    pub results: Vec<ArtistSong>,
    pub browse_id: PlaylistID<'static>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct ArtistSong {
    pub video_id: VideoID<'static>,
    pub plays: String,
    pub album: ParsedSongAlbum,
    pub artists: Vec<ParsedSongArtist>,
    /// Library management fields are optional; if a album has already been
    /// added to your library, you cannot add the individual songs.
    // https://github.com/nick42d/youtui/issues/138
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub like_status: LikeStatus,
    pub explicit: Explicit,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct GetArtistVideos {
    pub results: Vec<SearchResultVideo>,
    pub browse_id: PlaylistID<'static>,
}
/// The Albums section of the Browse Artist page.
/// The browse_id and params can be used to get the full list of artist's
/// albums. If they aren't set, and results is not empty, you can assume that
/// all albums are displayed here already.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct GetArtistAlbums {
    pub results: Vec<AlbumResult>,
    // XXX: Unsure if AlbumID is correct here.
    pub browse_id: Option<ArtistChannelID<'static>>,
    pub params: Option<BrowseParams<'static>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct RelatedResult {
    pub browse_id: ArtistChannelID<'static>,
    pub title: String,
    pub subscribers: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct AlbumResult {
    pub title: String,
    #[deprecated = "Future deprecation see https://github.com/nick42d/youtui/issues/211"]
    pub album_type: Option<AlbumType>,
    pub year: String,
    pub album_id: AlbumID<'static>,
    pub library_status: LibraryStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
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

// Should be at higher level in mod structure.
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
enum ArtistTopReleaseCategory {
    #[serde(alias = "albums")]
    Albums,
    #[serde(alias = "singles")]
    Singles,
    #[serde(alias = "videos")]
    Videos,
    #[serde(alias = "playlists")]
    Playlists,
    #[serde(alias = "fans might also like")]
    Related,
    #[serde(other)]
    None,
}

/// Google A/B change pending
pub(crate) fn parse_album_from_mtrir(mut navigator: JsonCrawlerBorrowed) -> Result<AlbumResult> {
    let title = navigator.take_value_pointer(TITLE_TEXT)?;

    let (year, album_type) = match navigator.borrow_pointer(SUBTITLE2) {
        Ok(mut subtitle2) => {
            // See https://github.com/nick42d/youtui/issues/211
            ab_warn!();
            (
                subtitle2.take_value()?,
                navigator.take_value_pointer(SUBTITLE)?,
            )
        }
        Err(_) => (navigator.take_value_pointer(SUBTITLE)?, None),
    };

    let album_id = navigator.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
    let thumbnails = navigator.take_value_pointer(THUMBNAIL_RENDERER)?;
    let explicit = if navigator.path_exists(concatcp!(SUBTITLE_BADGE_LABEL)) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let mut library_menu = navigator
        .borrow_pointer(MENU_ITEMS)?
        .try_into_iter()?
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
    menu: impl JsonCrawler,
) -> Result<Option<LibraryManager>> {
    let Some((status, add_to_library_token, remove_from_library_token)) = menu
        .try_into_iter()?
        .filter_map(|menu_item| {
            menu_item
                .navigate_pointer("/toggleMenuServiceItemRenderer")
                .ok()
        })
        .filter_map(|mut toggle_menu| {
            let Ok(status) = toggle_menu.take_value_pointer("/defaultIcon/iconType") else {
                // In this case the toggle_menu is not the right type, e.g might be Pin to
                // Listen Again.
                //
                // e.g: https://github.com/nick42d/youtui/issues/193
                return None;
            };
            if let Ok("Sign in") = toggle_menu
                .take_value_pointer::<String>(DEFAULT_ENDPOINT_MODAL_TEXT)
                .as_deref()
            {
                // In this case you are not signed in as so there are no add/remove from library
                // tokens.
                // NOTE: Since this is known at compile time, could specialise the
                // ParseFrom and return a hard error when signed in.
                return None;
            }
            let (add_to_library_token, remove_from_library_token) = match status {
                LibraryStatus::InLibrary => (
                    toggle_menu.take_value_pointer(TOGGLED_ENDPOINT),
                    toggle_menu.take_value_pointer(DEFAULT_ENDPOINT),
                ),
                LibraryStatus::NotInLibrary => (
                    toggle_menu.take_value_pointer(DEFAULT_ENDPOINT),
                    toggle_menu.take_value_pointer(TOGGLED_ENDPOINT),
                ),
            };
            Some((status, add_to_library_token, remove_from_library_token))
        })
        .next()
    else {
        // In this case there is no toggle_menu, so returning None is not an error.
        return Ok(None);
    };
    Ok(Some(LibraryManager {
        status,
        add_to_library_token: add_to_library_token?,
        remove_from_library_token: remove_from_library_token?,
    }))
}

impl<'a> ParseFrom<GetArtistAlbumsQuery<'a>> for Vec<GetArtistAlbumsAlbum> {
    fn parse_from(p: ProcessedResult<GetArtistAlbumsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut albums = Vec::new();
        let mut json_crawler = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            GRID_ITEMS
        ))?;
        for mut r in json_crawler
            .borrow_mut()
            .try_into_iter()?
            .flat_map(|i| i.navigate_pointer(MTRIR))
        {
            let browse_id = r.take_value_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?;
            let playlist_id = r.take_value_pointer(MENU_PLAYLIST_ID).ok();
            let title = r.take_value_pointer(TITLE_TEXT)?;
            let thumbnails = r.take_value_pointer(THUMBNAIL_RENDERER)?;
            // TODO: category
            let category = r.take_value_pointer(SUBTITLE).ok();
            albums.push(GetArtistAlbumsAlbum {
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
    use crate::auth::BrowserToken;
    use crate::common::{ArtistChannelID, BrowseParams, YoutubeID};
    use crate::query::GetArtistAlbumsQuery;

    #[tokio::test]
    async fn test_get_artist_albums_query() {
        parse_test!(
            // Radiohead's albums.
            "./test_json/browse_artist_albums.json",
            "./test_json/browse_artist_albums_output.txt",
            GetArtistAlbumsQuery::new(ArtistChannelID::from_raw(""), BrowseParams::from_raw("")),
            BrowserToken
        );
    }

    // Old as of https://github.com/nick42d/youtui/issues/211
    #[tokio::test]
    async fn test_get_artist_old_1() {
        parse_test!(
            "./test_json/get_artist_20240705.json",
            "./test_json/get_artist_20240705_output.txt",
            crate::query::GetArtistQuery::new(ArtistChannelID::from_raw("")),
            BrowserToken
        );
    }

    #[tokio::test]
    async fn test_get_artist() {
        parse_test!(
            "./test_json/get_artist_20250310.json",
            "./test_json/get_artist_20250310_output.txt",
            crate::query::GetArtistQuery::new(ArtistChannelID::from_raw("")),
            BrowserToken
        );
    }
}
