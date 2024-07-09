use super::{
    parse_item_text, parse_library_management_items_from_menu, EpisodeDate,
    EpisodeDuration, ParseFrom, ProcessedResult, SearchResultAlbum,
    TableListEpisode, TableListItem, TableListSong, TableListVideo, BADGE_LABEL, LIVE_BADGE_LABEL,
    MENU_LIKE_STATUS, SUBTITLE, SUBTITLE2, SUBTITLE3, SUBTITLE_BADGE_LABEL, TEXT_RUN_TEXT,
    THUMBNAILS,
};
use crate::common::library::{LibraryArtist, Playlist};
use crate::common::{AlbumType, Explicit, PlaylistID};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{
    GRID, GRID_ITEMS, ITEM_SECTION, MENU_ITEMS, MRLIR, MTRIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
    NAVIGATION_PLAYLIST_ID, NAVIGATION_VIDEO_TYPE, PLAY_BUTTON, SECTION_LIST, SECTION_LIST_ITEM,
    SINGLE_COLUMN_TAB, TEXT_RUN, THUMBNAIL_RENDERER, TITLE, TITLE_TEXT, WATCH_VIDEO_ID,
};
use crate::process::{process_fixed_column_item, process_flex_column_item};
use crate::query::{
    GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery,
    GetLibraryPlaylistsQuery, GetLibrarySongsQuery,
};
use crate::{Error, Result, Thumbnail};
use const_format::concatcp;

#[derive(Debug)]
// Very similar to LibraryArtist struct
pub struct GetLibraryArtistSubscription {
    name: String,
    subscribers: String,
    channel_id: String,
    thumbnails: Vec<Thumbnail>,
}

impl ParseFrom<GetLibraryArtistSubscriptionsQuery> for Vec<GetLibraryArtistSubscription> {
    fn parse_from(
        p: ProcessedResult<GetLibraryArtistSubscriptionsQuery>,
    ) -> crate::Result<<GetLibraryArtistSubscriptionsQuery as crate::query::Query>::Output> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_artist_subscriptions(json_crawler)
    }
}

impl ParseFrom<GetLibraryAlbumsQuery> for Vec<SearchResultAlbum> {
    fn parse_from(
        p: ProcessedResult<GetLibraryAlbumsQuery>,
    ) -> crate::Result<<GetLibraryAlbumsQuery as crate::query::Query>::Output> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_albums(json_crawler)
    }
}

impl ParseFrom<GetLibrarySongsQuery> for Vec<TableListSong> {
    fn parse_from(
        p: ProcessedResult<GetLibrarySongsQuery>,
    ) -> crate::Result<<GetLibrarySongsQuery as crate::query::Query>::Output> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_songs(json_crawler)
    }
}

impl ParseFrom<GetLibraryArtistsQuery> for Vec<LibraryArtist> {
    fn parse_from(
        p: ProcessedResult<GetLibraryArtistsQuery>,
    ) -> crate::Result<<GetLibraryArtistsQuery as crate::query::Query>::Output> {
        // TODO: Continuations
        let json_crawler = p.into();
        parse_library_artists(json_crawler)
    }
}

impl ParseFrom<GetLibraryPlaylistsQuery> for Vec<Playlist> {
    fn parse_from(
        p: ProcessedResult<GetLibraryPlaylistsQuery>,
    ) -> crate::Result<<GetLibraryPlaylistsQuery as crate::query::Query>::Output> {
        // TODO: Continuations
        // TODO: Implement count and author fields
        let json_crawler = p.into();
        parse_library_playlist_query(json_crawler)
    }
}

fn parse_library_albums(
    json_crawler: JsonCrawler,
) -> std::prelude::v1::Result<Vec<SearchResultAlbum>, crate::Error> {
    let items = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        GRID_ITEMS
    ))?;
    items
        .into_array_into_iter()?
        .map(parse_item_list_album)
        .collect()
}
fn parse_library_songs(
    json_crawler: JsonCrawler,
) -> std::prelude::v1::Result<Vec<TableListSong>, crate::Error> {
    let contents = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    contents
        .into_array_into_iter()?
        .map(|mut item| {
            let Ok(mut data) = item.borrow_pointer(MRLIR) else {
                return Ok(None);
            };
            let title = super::parse_item_text(&mut data, 0, 0)?;
            if title == "Shuffle all" {
                return Ok(None);
            }
            Ok(Some(parse_table_list_song(
                title,
                item.borrow_pointer(MRLIR)?,
            )?))
        })
        .filter_map(|result_option| result_option.transpose())
        .collect()
}
fn parse_library_artist_subscriptions(
    json_crawler: JsonCrawler,
) -> std::prelude::v1::Result<Vec<GetLibraryArtistSubscription>, crate::Error> {
    let contents = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    contents
        .into_array_into_iter()?
        .map(parse_content_list_artist_subscription)
        .collect()
}

fn parse_library_artists(json_crawler: JsonCrawler) -> Result<Vec<LibraryArtist>> {
    if let Some(contents) = process_library_contents_music_shelf(json_crawler) {
        parse_content_list_artists(contents)
    } else {
        Ok(Vec::new())
    }
}

fn parse_library_playlist_query(json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
    if let Some(contents) = process_library_contents_grid(json_crawler) {
        parse_content_list_playlist(contents)
    } else {
        Ok(Vec::new())
    }
}

// Consider returning ProcessedLibraryContents
// TODO: Move to process
fn process_library_contents_grid(mut json_crawler: JsonCrawler) -> Option<JsonCrawler> {
    let section = json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST));
    // Assume empty library in this case.
    if let Ok(section) = section {
        if section.path_exists("itemSectionRenderer") {
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
fn process_library_contents_music_shelf(mut json_crawler: JsonCrawler) -> Option<JsonCrawler> {
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

fn parse_item_list_album(mut json_crawler: JsonCrawler) -> Result<SearchResultAlbum> {
    let mut data = json_crawler.borrow_pointer("/musicTwoRowItemRenderer")?;
    let browse_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails = data.take_value_pointer(THUMBNAIL_RENDERER)?;
    let title = data.take_value_pointer(TITLE_TEXT)?;
    let artist = data.take_value_pointer(SUBTITLE2)?;
    let year = data.take_value_pointer(SUBTITLE3)?;
    let album_type = AlbumType::try_from_str(data.take_value_pointer::<String, &str>(SUBTITLE)?)?;
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
        browse_id,
        album_type,
        thumbnails,
    })
}

fn parse_content_list_artist_subscription(
    mut json_crawler: JsonCrawler,
) -> Result<GetLibraryArtistSubscription> {
    let mut data = json_crawler.borrow_pointer(MRLIR)?;
    let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let name = parse_item_text(&mut data, 0, 0)?;
    let subscribers = parse_item_text(&mut data, 1, 0)?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    Ok(GetLibraryArtistSubscription {
        name,
        subscribers,
        channel_id,
        thumbnails,
    })
}

fn parse_content_list_artists(json_crawler: JsonCrawler) -> Result<Vec<LibraryArtist>> {
    let mut results = Vec::new();
    for result in json_crawler
        .navigate_pointer("/contents")?
        .as_array_iter_mut()?
    {
        let mut data = result.navigate_pointer(MRLIR)?;
        let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
        let artist = parse_item_text(&mut data, 0, 0)?;
        let byline = parse_item_text(&mut data, 1, 0)?;
        results.push(LibraryArtist {
            channel_id,
            artist,
            byline,
        })
    }
    Ok(results)
}
pub(crate) fn parse_table_list_items(contents: JsonCrawler) -> Result<Vec<TableListItem>> {
    contents
        .into_array_into_iter()?
        .filter_map(|item| parse_table_list_item(item).transpose())
        .collect()
}
fn parse_table_list_item(mut json: JsonCrawler) -> Result<Option<TableListItem>> {
    let Ok(mut data) = json.borrow_pointer(MRLIR) else {
        return Ok(None);
    };
    let title = super::parse_item_text(&mut data, 0, 0)?;
    if title == "Shuffle all" {
        return Ok(None);
    }
    let video_type_path = concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        NAVIGATION_VIDEO_TYPE
    );
    let video_type: String = data.take_value_pointer(video_type_path)?;
    let item = match video_type.as_ref() {
        "MUSIC_VIDEO_TYPE_PODCAST_EPISODE" => Some(TableListItem::Episode(
            parse_table_list_episode(title, data)?,
        )),
        // I believe OMV is 'Official Music Video' and UGC is 'User Generated Content'
        "MUSIC_VIDEO_TYPE_UGC" | "MUSIC_VIDEO_TYPE_OMV" => {
            Some(TableListItem::Video(parse_table_list_video(title, data)?))
        }
        // Could be 'Audio Track Video'?
        "MUSIC_VIDEO_TYPE_ATV" => Some(TableListItem::Song(parse_table_list_song(title, data)?)),
        other => {
            return Err(Error::other(format!(
                "Unsupported video type <{other}> at location {}{video_type_path}",
                data.get_path()
            )))
        }
    };
    Ok(item)
}

fn parse_table_list_episode(
    title: String,
    mut data: JsonCrawlerBorrowed,
) -> Result<TableListEpisode> {
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
            let date = parse_item_text(&mut data, 2, 0)?;
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
    let podcast_name = process_flex_column_item(&mut data, 1)?.take_value_pointer(TEXT_RUN_TEXT)?;
    let podcast_id = process_flex_column_item(&mut data, 1)?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String, &str>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    Ok(TableListEpisode {
        video_id,
        duration,
        title,
        like_status,
        thumbnails,
        date,
        podcast_name,
        podcast_id,
        is_available,
    })
}

fn parse_table_list_video(title: String, mut data: JsonCrawlerBorrowed) -> Result<TableListVideo> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let channel_name = process_flex_column_item(&mut data, 1)?.take_value_pointer(TEXT_RUN_TEXT)?;
    let channel_id = process_flex_column_item(&mut data, 1)?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let duration = process_fixed_column_item(&mut data, 0).and_then(|mut i| {
        i.take_value_pointer("/text/simpleText")
            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
    })?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String, &str>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(TableListVideo {
        video_id,
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
fn parse_table_list_song(title: String, mut data: JsonCrawlerBorrowed) -> Result<TableListSong> {
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

fn parse_content_list_playlist(json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
    // TODO: Implement count and author fields
    let mut results = Vec::new();
    for result in json_crawler
        .navigate_pointer("/items")?
        .as_array_iter_mut()?
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
            let runs = subtitle.borrow_pointer("/runs")?.into_array_iter_mut()?;
            // Extract description from runs.
            // Collect the iterator of Result<String> into a single Result<String>
            description = Some(
                runs.map(|mut c| c.take_value_pointer::<String, &str>("/text"))
                    .collect::<Result<String>>()?,
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
    use crate::{
        auth::BrowserToken,
        common::library::{LibraryArtist, Playlist},
        query::{GetLibraryArtistsQuery, GetLibraryPlaylistsQuery},
        YtMusic,
    };
    use serde_json::json;

    // Consider if the parse function itself should be removed from impl.
    #[test]
    fn test_library_playlists_dummy_json() {
        let testfile = std::fs::read_to_string("test_json/get_library_playlists.json").unwrap();
        let result =
            YtMusic::<BrowserToken>::process_json(testfile, GetLibraryPlaylistsQuery {}).unwrap();
        let expected = json!([
          {
            "playlist_id": "VLLM",
            "title": "Liked Music",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-music-@192.png"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-music-@576.png"
              }
            ],
            "count": null,
            "description": "Auto playlist",
            "author": null
          },
          {
            "playlist_id": "VLPLCZQcydUIP07hMOwAXIag92l76d3z3Thv",
            "title": "Listen later",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://yt3.ggpht.com/oGdMcu3X8XKqSc9QMRqV3rqznKuPScNylHcqmKiBfLE1TZ7gkqFJRwQX2rAiWyAOuLPM614fSDo=s192"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://yt3.ggpht.com/oGdMcu3X8XKqSc9QMRqV3rqznKuPScNylHcqmKiBfLE1TZ7gkqFJRwQX2rAiWyAOuLPM614fSDo=s576"
              }
            ],
            "count": null,
            "description": "Nick Dowsett • 20 tracks",
            "author": null
          },
          {
            "playlist_id": "VLRDCLAK5uy_lRzD6ZcGWU_ef3r4y7ifNYLiGmCCX_jIk",
            "title": "Deadly Hotlist",
            "thumbnails": [
              {
                "height": 226,
                "width": 226,
                "url": "https://lh3.googleusercontent.com/HJoX79I4ngSCHXjzEWHwWpvwlK2cMhbezyKN8I-lH06APDbjIAUymVCI1VmeB5EcrNwglLAB0Edlt1KL=w226-h226-l90-rj"
              },
              {
                "height": 544,
                "width": 544,
                "url": "https://lh3.googleusercontent.com/HJoX79I4ngSCHXjzEWHwWpvwlK2cMhbezyKN8I-lH06APDbjIAUymVCI1VmeB5EcrNwglLAB0Edlt1KL=w544-h544-l90-rj"
              }
            ],
            "count": null,
            "description": "YouTube Music • 50 songs",
            "author": null
          },
          {
            "playlist_id": "VLSE",
            "title": "Episodes for Later",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/saved-episodes-@192.png"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/saved-episodes-@576.png"
              }
            ],
            "count": null,
            "description": "Episodes you save for later",
            "author": null
          }
        ]);
        let expected: Vec<Playlist> = serde_json::from_value(expected).unwrap();
        assert_eq!(result, expected);
    }
    #[test]
    fn test_library_artists_dummy_json() {
        let testfile = std::fs::read_to_string("test_json/get_library_artists.json").unwrap();
        let result =
            YtMusic::<BrowserToken>::process_json(testfile, GetLibraryArtistsQuery::default())
                .unwrap();
        let expected = json!(
            [
                {
                  "channel_id" : "MPLAUCprAFmT0C6O4X0ToEXpeFTQ",
                  "artist": "Kendrick Lamar",
                  "byline": "16 songs"
                },
                {
                  "channel_id" : "MPLAUC_yH_GaGHZk9ewo5ghQA75w",
                  "artist": "Dream Theater",
                  "byline": "1 song"
                },
                {
                  "channel_id" : "MPLAUCHUlZT-VoVWIID4xcJZ5s6g",
                  "artist": "Nils Frahm",
                  "byline": "1 song"
                },
            ]
        );
        let expected: Vec<LibraryArtist> = serde_json::from_value(expected).unwrap();
        assert_eq!(result, expected);
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
}
