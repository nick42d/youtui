use super::{
    BADGE_LABEL, CONTINUATION_PARAMS, GRID_CONTINUATION, MENU_LIKE_STATUS,
    MUSIC_SHELF_CONTINUATION, ParseFrom, ParsedPodcastChannel, ProcessedResult, SUBTITLE,
    SUBTITLE_BADGE_LABEL, SUBTITLE2, SUBTITLE3, SearchResultAlbum, THUMBNAILS, TableListSong,
    fixed_column_item_pointer, parse_flex_column_item, parse_library_management_items_from_menu,
    parse_podcast_channel,
};
use crate::Result;
use crate::common::{
    ApiOutcome, ArtistChannelID, ContinuationParams, Explicit, PlaylistID, PodcastChannelID,
    PodcastID, Thumbnail,
};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    GRID, ITEM_SECTION, MENU_ITEMS, MRLIR, MTRIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
    NAVIGATION_PLAYLIST_ID, PLAY_BUTTON, RUN_TEXT, SECTION_LIST, SECTION_LIST_ITEM,
    SINGLE_COLUMN_TAB, SUBTITLE_BADGE_ICON, THUMBNAIL_RENDERER, TITLE, TITLE_TEXT, WATCH_VIDEO_ID,
};
use crate::query::library::{GetLibraryChannelsQuery, GetLibraryPodcastsQuery};
use crate::query::{
    EditSongLibraryStatusQuery, GetContinuationsQuery, GetLibraryAlbumsQuery,
    GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery, GetLibraryPlaylistsQuery,
    GetLibrarySongsQuery,
};
use crate::youtube_enums::YoutubeMusicBadgeRendererIcon;
use const_format::concatcp;
use json_crawler::{CrawlerResult, JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// Very similar to LibraryArtist struct
pub struct GetLibraryArtistSubscription {
    pub name: String,
    pub subscribers: String,
    pub channel_id: ArtistChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// Very similar to LibraryArtist struct
pub struct LibraryArtistSubscription {
    pub name: String,
    pub subscribers: String,
    pub channel_id: ArtistChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct LibraryPlaylist {
    pub playlist_id: PlaylistID<'static>,
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub tracks: String,
    pub author: String,
    // Authoer may be YouTube Music in some cases - no ChannelID
    pub author_id: Option<ArtistChannelID<'static>>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct LibraryArtist {
    pub channel_id: ArtistChannelID<'static>,
    pub artist: String,
    pub byline: String, // e.g 16 songs or 17.8k subscribers
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct LibraryPodcast {
    pub title: String,
    pub channels: Vec<ParsedPodcastChannel>,
    pub podcast_id: PodcastID<'static>,
    pub thumbnails: Vec<Thumbnail>,
    pub podcast_source: PodcastSource,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct LibraryChannel {
    pub title: String,
    pub subscribers: String,
    pub channel_id: PodcastChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PodcastSource {
    Rss,
    YouTube,
}

impl ParseFromContinuable<GetLibraryArtistSubscriptionsQuery> for Vec<LibraryArtistSubscription> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryArtistSubscriptionsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_shelf = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
        ))?;
        parse_library_artist_subscriptions(music_shelf)
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryArtistSubscriptionsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_shelf = json_crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
        parse_library_artist_subscriptions(music_shelf)
    }
}

impl ParseFromContinuable<GetLibraryAlbumsQuery> for Vec<SearchResultAlbum> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryAlbumsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let grid_renderer =
            json_crawler.navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID))?;
        parse_library_albums(grid_renderer)
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryAlbumsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let grid_items = json_crawler.navigate_pointer(GRID_CONTINUATION)?;
        parse_library_albums(grid_items)
    }
}

impl ParseFromContinuable<GetLibrarySongsQuery> for Vec<TableListSong> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibrarySongsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_shelf = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            MUSIC_SHELF,
        ))?;
        parse_library_songs(music_shelf)
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibrarySongsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_shelf = json_crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
        parse_library_songs(music_shelf)
    }
}

impl ParseFromContinuable<GetLibraryArtistsQuery> for Vec<LibraryArtist> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryArtistsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler = p.into();
        let maybe_music_shelf = process_library_contents_music_shelf(json_crawler);
        if let Some(music_shelf) = maybe_music_shelf {
            parse_content_list_artists(music_shelf)
        } else {
            Ok((Vec::new(), None))
        }
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryArtistsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler = JsonCrawlerOwned::from(p);
        let music_shelf = json_crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
        parse_content_list_artists(music_shelf)
    }
}

impl ParseFromContinuable<GetLibraryPlaylistsQuery> for Vec<LibraryPlaylist> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryPlaylistsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        // TODO: Implement count and author fields
        let json_crawler = p.into();
        let maybe_grid_renderer = process_library_contents_grid(json_crawler);
        if let Some(grid_renderer) = maybe_grid_renderer {
            parse_library_playlists(grid_renderer)
        } else {
            Ok((vec![], None))
        }
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryPlaylistsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let grid_renderer = json_crawler.navigate_pointer(GRID_CONTINUATION)?;
        parse_library_playlists(grid_renderer)
    }
}

impl ParseFromContinuable<GetLibraryPodcastsQuery> for Vec<LibraryPodcast> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryPodcastsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let maybe_grid_renderer = process_library_contents_grid(json_crawler);
        if let Some(grid_renderer) = maybe_grid_renderer {
            parse_library_podcasts(grid_renderer)
        } else {
            Ok((vec![], None))
        }
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryPodcastsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let grid_renderer = json_crawler.navigate_pointer(GRID_CONTINUATION)?;
        parse_library_podcasts(grid_renderer)
    }
}

impl ParseFromContinuable<GetLibraryChannelsQuery> for Vec<LibraryChannel> {
    fn parse_from_continuable(
        p: ProcessedResult<GetLibraryChannelsQuery>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler = p.into();
        let maybe_music_shelf = process_library_contents_music_shelf(json_crawler);
        if let Some(music_shelf) = maybe_music_shelf {
            parse_content_list_channels(music_shelf)
        } else {
            Ok((Vec::new(), None))
        }
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetLibraryChannelsQuery>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        let json_crawler = JsonCrawlerOwned::from(p);
        let music_shelf = json_crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
        parse_content_list_channels(music_shelf)
    }
}

impl ParseFrom<EditSongLibraryStatusQuery<'_>> for Vec<ApiOutcome> {
    fn parse_from(p: super::ProcessedResult<EditSongLibraryStatusQuery>) -> Result<Self> {
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
            .collect::<CrawlerResult<_>>()
            .map_err(Into::into)
    }
}

fn parse_library_albums(
    mut grid_renderer: JsonCrawlerOwned,
) -> Result<(Vec<SearchResultAlbum>, Option<ContinuationParams<'static>>)> {
    let continuation_params = grid_renderer.take_value_pointer(CONTINUATION_PARAMS).ok();
    let albums = grid_renderer
        .navigate_pointer("/items")?
        .try_into_iter()?
        .map(parse_item_list_album)
        .collect::<Result<_>>()?;
    Ok((albums, continuation_params))
}
fn parse_library_songs(
    mut music_shelf: JsonCrawlerOwned,
) -> Result<(Vec<TableListSong>, Option<ContinuationParams<'static>>)> {
    let continuation_params = music_shelf.take_value_pointer(CONTINUATION_PARAMS).ok();
    let songs = music_shelf
        .navigate_pointer("/contents")?
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
        .collect::<Result<_>>()?;
    Ok((songs, continuation_params))
}
fn parse_library_artist_subscriptions(
    mut music_shelf: JsonCrawlerOwned,
) -> Result<(
    Vec<LibraryArtistSubscription>,
    Option<ContinuationParams<'static>>,
)> {
    let continuation_params = music_shelf.take_value_pointer(CONTINUATION_PARAMS).ok();
    let subscriptions = music_shelf
        .navigate_pointer("/contents")?
        .try_into_iter()?
        .map(parse_content_list_artist_subscription)
        .collect::<Result<_>>()?;
    Ok((subscriptions, continuation_params))
}

fn parse_library_playlists(
    mut grid_renderer: JsonCrawlerOwned,
) -> Result<(Vec<LibraryPlaylist>, Option<ContinuationParams<'static>>)> {
    let continuation_params = grid_renderer.take_value_pointer(CONTINUATION_PARAMS).ok();
    let playlists = grid_renderer
        .navigate_pointer("/items")?
        .try_into_iter()?
        // First result is just a link to create a new playlist.
        .skip(1)
        .filter_map(|item| parse_content_list_playlist(item).transpose())
        .collect::<Result<_>>()?;
    Ok((playlists, continuation_params))
}
fn parse_library_podcasts(
    mut grid_renderer: impl JsonCrawler,
) -> Result<(Vec<LibraryPodcast>, Option<ContinuationParams<'static>>)> {
    let continuation_params = grid_renderer.take_value_pointer(CONTINUATION_PARAMS).ok();
    let res = grid_renderer
        .navigate_pointer("/items")?
        .try_into_iter()?
        // First result is just a link to create a new podcast.
        .skip(1)
        .filter_map(|item| parse_content_list_podcast(item).transpose())
        .collect::<Result<_>>()?;
    Ok((res, continuation_params))
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
) -> Result<LibraryArtistSubscription> {
    let mut data = json_crawler.borrow_pointer(MRLIR)?;
    let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let name = parse_flex_column_item(&mut data, 0, 0)?;
    let subscribers = parse_flex_column_item(&mut data, 1, 0)?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    Ok(LibraryArtistSubscription {
        name,
        subscribers,
        channel_id,
        thumbnails,
    })
}

fn parse_content_list_artists(
    mut json_crawler: JsonCrawlerOwned,
) -> Result<(Vec<LibraryArtist>, Option<ContinuationParams<'static>>)> {
    let continuation_params = json_crawler.take_value_pointer(CONTINUATION_PARAMS).ok();
    let artists = json_crawler
        .navigate_pointer("/contents")?
        .try_iter_mut()?
        .map(|item| {
            let mut data = item.navigate_pointer(MRLIR)?;
            let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let artist = parse_flex_column_item(&mut data, 0, 0)?;
            let byline = parse_flex_column_item(&mut data, 1, 0)?;
            Ok(LibraryArtist {
                channel_id,
                artist,
                byline,
            })
        })
        .collect::<Result<_>>()?;
    Ok((artists, continuation_params))
}

fn parse_content_list_channels(
    mut json_crawler: JsonCrawlerOwned,
) -> Result<(Vec<LibraryChannel>, Option<ContinuationParams<'static>>)> {
    let continuation_params = json_crawler.take_value_pointer(CONTINUATION_PARAMS).ok();
    let artists = json_crawler
        .navigate_pointer("/contents")?
        .try_iter_mut()?
        .map(|item| {
            let mut data = item.navigate_pointer(MRLIR)?;
            let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let title = parse_flex_column_item(&mut data, 0, 0)?;
            let subscribers = parse_flex_column_item(&mut data, 1, 0)?;
            let thumbnails = data.take_value_pointer(THUMBNAILS)?;
            Ok(LibraryChannel {
                title,
                subscribers,
                channel_id,
                thumbnails,
            })
        })
        .collect::<Result<_>>()?;
    Ok((artists, continuation_params))
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
        .take_value_pointers(&["/text/simpleText", "/text/runs/0/text"])?;
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

fn parse_content_list_playlist(item: JsonCrawlerOwned) -> Result<Option<LibraryPlaylist>> {
    // TODO: Implement count and author fields
    let mut mtrir = item.navigate_pointer(MTRIR)?;
    let title: String = mtrir.take_value_pointer(TITLE_TEXT)?;
    // There are some potential special playlist results. This is one
    // way to filter them out.
    // TODO: i18n or more robust method of filtering.
    if title.eq_ignore_ascii_case("liked music") || title.eq_ignore_ascii_case("episodes for later")
    {
        return Ok(None);
    }
    let playlist_id: PlaylistID = mtrir
        .borrow_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?
        // ytmusicapi uses range index [2:] here but doesn't seem to be required.
        // Revisit later if we crash.
        .take_value()?;
    let thumbnails: Vec<Thumbnail> = mtrir.take_value_pointer(THUMBNAIL_RENDERER)?;
    let mut subtitle = mtrir.navigate_pointer("/subtitle")?;
    let tracks = subtitle.take_value_pointer("/runs/2/text")?;
    let mut author_run = subtitle.navigate_pointer("/runs/0")?;
    let author = author_run.take_value_pointer("/text")?;
    let author_id = author_run.take_value_pointer(NAVIGATION_BROWSE_ID).ok();
    Ok(Some(LibraryPlaylist {
        playlist_id,
        title,
        thumbnails,
        tracks,
        author_id,
        author,
    }))
}

fn parse_content_list_podcast(item: impl JsonCrawler) -> Result<Option<LibraryPodcast>> {
    let mut mtrir = item.navigate_pointer(MTRIR)?;
    let title: String = mtrir.take_value_pointer(TITLE_TEXT)?;
    // There are some potential non-podcast special playlist results. This is one
    // way to filter them out.
    // TODO: i18n or more robust method of filtering.
    if title.eq_ignore_ascii_case("new episodes")
        || title.eq_ignore_ascii_case("Episodes for Later")
    {
        return Ok(None);
    }
    let podcast_id: PodcastID = mtrir
        .borrow_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?
        // ytmusicapi uses range index [2:] here but doesn't seem to be required.
        // Revisit later if we crash.
        .take_value()?;
    let thumbnails: Vec<Thumbnail> = mtrir.take_value_pointer(THUMBNAIL_RENDERER)?;
    let maybe_badge_icon = mtrir
        .take_value_pointer::<YoutubeMusicBadgeRendererIcon>(SUBTITLE_BADGE_ICON)
        .ok();
    let podcast_source = match maybe_badge_icon {
        Some(YoutubeMusicBadgeRendererIcon::Rss) => PodcastSource::Rss,
        _ => PodcastSource::YouTube,
    };
    let channels = mtrir
        .navigate_pointer("/subtitle/runs")?
        .try_into_iter()?
        .map(parse_podcast_channel)
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(LibraryPodcast {
        title,
        thumbnails,
        channels,
        podcast_id,
        podcast_source,
    }))
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;

    #[tokio::test]
    async fn test_library_playlists_dummy_json() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_playlists.json",
            "./test_json/get_library_playlists_continuation_mock.json",
            "./test_json/get_library_playlists_output.txt",
            crate::query::GetLibraryPlaylistsQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_artists_dummy_json() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_artists.json",
            "./test_json/get_library_artists_continuation_mock.json",
            "./test_json/get_library_artists_output.txt",
            crate::query::GetLibraryArtistsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_albums() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_albums_20240701.json",
            "./test_json/get_library_albums_continuation_mock.json",
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
    async fn test_get_library_songs_continuation() {
        parse_continuations_test!(
            "./test_json/get_library_songs_continuation_20240910.json",
            "./test_json/get_library_songs_continuation_20240910_output.txt",
            crate::query::GetLibrarySongsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_artist_subscriptions() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_artist_subscriptions_20240701.json",
            "./test_json/get_library_artist_subscriptions_continuation_mock.json",
            "./test_json/get_library_artist_subscriptions_20240701_output.txt",
            crate::query::GetLibraryArtistSubscriptionsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_podcasts() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_podcasts_20250626.json",
            "./test_json/get_library_podcasts_continuation_20250626.json",
            "./test_json/get_library_podcasts_20250626_output.txt",
            crate::query::GetLibraryPodcastsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_channels() {
        parse_with_matching_continuation_test!(
            "./test_json/get_library_channels_20250626.json",
            "./test_json/get_library_channels_continuation_20250626.json",
            "./test_json/get_library_channels_20250626_output.txt",
            crate::query::GetLibraryChannelsQuery::default(),
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
