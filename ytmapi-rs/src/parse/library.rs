use super::{
    parse_flex_column_item, parse_library_management_items_from_menu, ParseFrom, ProcessedResult,
    SearchResultAlbum, TableListSong, BADGE_LABEL, MENU_LIKE_STATUS, SUBTITLE, SUBTITLE2,
    SUBTITLE3, SUBTITLE_BADGE_LABEL, THUMBNAILS,
};
use crate::common::library::{LibraryArtist, Playlist};
use crate::common::{ApiOutcome, Explicit, PlaylistID};
use crate::nav_consts::{
    GRID, GRID_ITEMS, ITEM_SECTION, MENU_ITEMS, MRLIR, MTRIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
    NAVIGATION_PLAYLIST_ID, PLAY_BUTTON, SECTION_LIST, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB,
    THUMBNAIL_RENDERER, TITLE, TITLE_TEXT, WATCH_VIDEO_ID,
};
use crate::process::fixed_column_item_pointer;
use crate::query::{
    EditSongLibraryStatusQuery, GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery,
    GetLibraryArtistsQuery, GetLibraryPlaylistsQuery, GetLibrarySongsQuery,
};
use crate::{ChannelID, Result, Thumbnail};
use const_format::concatcp;
use ytmapi_rs_json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerOwned};

#[derive(Debug)]
// Very similar to LibraryArtist struct
pub struct GetLibraryArtistSubscription {
    pub name: String,
    pub subscribers: String,
    pub channel_id: ChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

impl ParseFrom<GetLibraryArtistSubscriptionsQuery> for Vec<GetLibraryArtistSubscription> {
    fn parse_from(p: ProcessedResult<GetLibraryArtistSubscriptionsQuery>) -> crate::Result<Self> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_artist_subscriptions(json_crawler)
    }
}

impl ParseFrom<GetLibraryAlbumsQuery> for Vec<SearchResultAlbum> {
    fn parse_from(p: ProcessedResult<GetLibraryAlbumsQuery>) -> crate::Result<Self> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_albums(json_crawler)
    }
}

impl ParseFrom<GetLibrarySongsQuery> for Vec<TableListSong> {
    fn parse_from(p: ProcessedResult<GetLibrarySongsQuery>) -> crate::Result<Self> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_songs(json_crawler)
    }
}

impl ParseFrom<GetLibraryArtistsQuery> for Vec<LibraryArtist> {
    fn parse_from(p: ProcessedResult<GetLibraryArtistsQuery>) -> crate::Result<Self> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_artists(json_crawler)
    }
}

impl<'a> ParseFrom<EditSongLibraryStatusQuery<'a>> for Vec<ApiOutcome> {
    fn parse_from(p: super::ProcessedResult<EditSongLibraryStatusQuery>) -> crate::Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        json_crawler
            .navigate_pointer("/feedbackResponses")?
            .try_into_iter()?
            .map(|mut response| {
                response
                    .take_value_pointer::<bool>("/isProcessed")
                    .map(|p| {
                        if p {
                            return ApiOutcome::Success;
                        }
                        ApiOutcome::Failure
                    })
            })
            .rev()
            .collect::<ytmapi_rs_json_crawler::CrawlerResult<_>>()
            .map_err(Into::into)
    }
}

impl ParseFrom<GetLibraryPlaylistsQuery> for Vec<Playlist> {
    fn parse_from(p: ProcessedResult<GetLibraryPlaylistsQuery>) -> crate::Result<Self> {
        // TODO: Continuations
        // TODO: Implement count and author fields
        let json_crawler = p.into();
        parse_library_playlist_query(json_crawler)
    }
}

fn parse_library_albums(
    json_crawler: JsonCrawlerOwned,
) -> std::prelude::v1::Result<Vec<SearchResultAlbum>, crate::Error> {
    let items = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        GRID_ITEMS
    ))?;
    items.try_into_iter()?.map(parse_item_list_album).collect()
}
fn parse_library_songs(
    json_crawler: JsonCrawlerOwned,
) -> std::prelude::v1::Result<Vec<TableListSong>, crate::Error> {
    let contents = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    contents
        .try_into_iter()?
        .map(|mut item| {
            let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                return Ok(None);
            };
            let title = super::parse_flex_column_item(&mut data, 0, 0)?;
            if title == "Shuffle all" {
                return Ok(None);
            }
            Ok(Some(parse_table_list_song(title, data)?))
        })
        .filter_map(Result::transpose)
        .collect()
}
fn parse_library_artist_subscriptions(
    json_crawler: JsonCrawlerOwned,
) -> std::prelude::v1::Result<Vec<GetLibraryArtistSubscription>, crate::Error> {
    let contents = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    contents
        .try_into_iter()?
        .map(parse_content_list_artist_subscription)
        .collect()
}

fn parse_library_artists(json_crawler: JsonCrawlerOwned) -> Result<Vec<LibraryArtist>> {
    if let Some(contents) = process_library_contents_music_shelf(json_crawler) {
        parse_content_list_artists(contents)
    } else {
        Ok(Vec::new())
    }
}

fn parse_library_playlist_query(json_crawler: JsonCrawlerOwned) -> Result<Vec<Playlist>> {
    if let Some(contents) = process_library_contents_grid(json_crawler) {
        parse_content_list_playlist(contents)
    } else {
        Ok(Vec::new())
    }
}

// Consider returning ProcessedLibraryContents
// TODO: Move to process
fn process_library_contents_grid(mut json_crawler: JsonCrawlerOwned) -> Option<JsonCrawlerOwned> {
    let section = json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST));
    // Assume empty library in this case.
    if let Ok(section) = section {
        if section.path_exists("/itemSectionRenderer") {
            json_crawler
                .navigate_pointer(concatcp!(ITEM_SECTION, GRID))
                .ok()
        } else {
            json_crawler
                .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID))
                .ok()
        }
    } else {
        None
    }
}
// Consider returning ProcessedLibraryContents
// TODO: Move to process
fn process_library_contents_music_shelf(
    mut json_crawler: JsonCrawlerOwned,
) -> Option<JsonCrawlerOwned> {
    let section = json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST));
    // Assume empty library in this case.
    if let Ok(section) = section {
        if section.path_exists("itemSectionRenderer") {
            json_crawler
                .navigate_pointer(concatcp!(ITEM_SECTION, MUSIC_SHELF))
                .ok()
        } else {
            json_crawler
                .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, MUSIC_SHELF))
                .ok()
        }
    } else {
        None
    }
}

fn parse_item_list_album(mut json_crawler: JsonCrawlerOwned) -> Result<SearchResultAlbum> {
    let mut data = json_crawler.borrow_pointer("/musicTwoRowItemRenderer")?;
    let browse_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails = data.take_value_pointer(THUMBNAIL_RENDERER)?;
    let title = data.take_value_pointer(TITLE_TEXT)?;
    let artist = data.take_value_pointer(SUBTITLE2)?;
    let year = data.take_value_pointer(SUBTITLE3)?;
    let album_type = data.take_value_pointer(SUBTITLE)?;
    let explicit = if data.path_exists(SUBTITLE_BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    Ok(SearchResultAlbum {
        title,
        artist,
        year,
        explicit,
        album_id: browse_id,
        album_type,
        thumbnails,
    })
}

fn parse_content_list_artist_subscription(
    mut json_crawler: JsonCrawlerOwned,
) -> Result<GetLibraryArtistSubscription> {
    let mut data = json_crawler.borrow_pointer(MRLIR)?;
    let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let name = parse_flex_column_item(&mut data, 0, 0)?;
    let subscribers = parse_flex_column_item(&mut data, 1, 0)?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    Ok(GetLibraryArtistSubscription {
        name,
        subscribers,
        channel_id,
        thumbnails,
    })
}

fn parse_content_list_artists(json_crawler: JsonCrawlerOwned) -> Result<Vec<LibraryArtist>> {
    let mut results = Vec::new();
    for result in json_crawler.navigate_pointer("/contents")?.try_iter_mut()? {
        let mut data = result.navigate_pointer(MRLIR)?;
        let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
        let artist = parse_flex_column_item(&mut data, 0, 0)?;
        let byline = parse_flex_column_item(&mut data, 1, 0)?;
        results.push(LibraryArtist {
            channel_id,
            artist,
            byline,
        })
    }
    Ok(results)
}

fn parse_table_list_song(title: String, mut data: JsonCrawlerBorrowed) -> Result<TableListSong> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let library_management =
        parse_library_management_items_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let artists = super::parse_song_artists(&mut data, 1)?;
    let album = super::parse_song_album(&mut data, 2)?;
    let duration = data
        .borrow_pointer(fixed_column_item_pointer(0))?
        .take_value_pointers(vec!["/text/simpleText", "/text/runs/0/text"])?;
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
    Ok(TableListSong {
        video_id,
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

fn parse_content_list_playlist(json_crawler: JsonCrawlerOwned) -> Result<Vec<Playlist>> {
    // TODO: Implement count and author fields
    let mut results = Vec::new();
    for result in json_crawler
        .navigate_pointer("/items")?
        .try_iter_mut()?
        // First result is just a link to create a new playlist.
        .skip(1)
        .map(|c| c.navigate_pointer(MTRIR))
    {
        let mut result = result?;
        let title = result.take_value_pointer(TITLE_TEXT)?;
        let playlist_id: PlaylistID = result
            .borrow_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?
            // ytmusicapi uses range index [2:] here but doesn't seem to be required.
            // Revisit later if we crash.
            .take_value()?;
        let thumbnails: Vec<Thumbnail> = result.take_value_pointer(THUMBNAIL_RENDERER)?;
        let mut description = None;
        let count = None;
        let author = None;
        if let Ok(mut subtitle) = result.borrow_pointer("/subtitle") {
            let runs = subtitle.borrow_pointer("/runs")?.try_into_iter()?;
            // Extract description from runs.
            // Collect the iterator of Result<String> into a single Result<String>
            description = Some(
                runs.map(|mut c| c.take_value_pointer::<String>("/text"))
                    .collect::<std::result::Result<String, _>>()?,
            );
        }
        let playlist = Playlist {
            description,
            author,
            playlist_id,
            title,
            thumbnails,
            count,
        };
        results.push(playlist)
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;

    // Consider if the parse function itself should be removed from impl.
    #[tokio::test]
    async fn test_library_playlists_dummy_json() {
        parse_test!(
            "./test_json/get_library_playlists.json",
            "./test_json/get_library_playlists_output.txt",
            crate::query::GetLibraryPlaylistsQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_library_artists_dummy_json() {
        parse_test!(
            "./test_json/get_library_artists.json",
            "./test_json/get_library_artists_output.txt",
            crate::query::GetLibraryArtistsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_albums() {
        parse_test!(
            "./test_json/get_library_albums_20240701.json",
            "./test_json/get_library_albums_20240701_output.txt",
            crate::query::GetLibraryAlbumsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_songs() {
        parse_test!(
            "./test_json/get_library_songs_20240701.json",
            "./test_json/get_library_songs_20240701_output.txt",
            crate::query::GetLibrarySongsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_artist_subscriptions() {
        parse_test!(
            "./test_json/get_library_artist_subscriptions_20240701.json",
            "./test_json/get_library_artist_subscriptions_20240701_output.txt",
            crate::query::GetLibraryArtistSubscriptionsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_edit_song_library_status() {
        // Note - same files as remove_histry_items
        parse_test!(
            "./test_json/remove_history_items_20240704.json",
            "./test_json/remove_history_items_20240704_output.txt",
            crate::query::EditSongLibraryStatusQuery::new_from_add_to_library_feedback_tokens(
                Vec::new()
            )
            .with_remove_from_library_feedback_tokens(vec![]),
            BrowserToken
        );
    }
}
