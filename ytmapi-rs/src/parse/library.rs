use super::{
    parse_item_text, parse_playlist_items, ParseFrom, ProcessedResult, SearchResultAlbum,
    SongResult, BADGE_LABEL, SUBTITLE, SUBTITLE2, SUBTITLE3, SUBTITLE_BADGE_LABEL, THUMBNAILS,
    TWO_COLUMN,
};
use crate::common::library::{LibraryArtist, Playlist};
use crate::common::{AlbumType, Explicit, PlaylistID};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{
    GRID, GRID_ITEMS, ITEM_SECTION, MRLIR, MTRIR, MUSIC_SHELF, NAVIGATION_BROWSE_ID, SECTION_LIST,
    SECTION_LIST_ITEM, SINGLE_COLUMN_TAB, THUMBNAIL_RENDERER, TITLE, TITLE_TEXT,
};
use crate::query::{
    GetLibraryAlbumsQuery, GetLibraryArtistSubscriptionsQuery, GetLibraryArtistsQuery,
    GetLibraryPlaylistsQuery, GetLibrarySongsQuery,
};
use crate::{Result, Thumbnail};
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

impl ParseFrom<GetLibrarySongsQuery> for Vec<SongResult> {
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
        .map(|r| parse_item_list_albums(r))
        .collect()
}
fn parse_library_songs(
    json_crawler: JsonCrawler,
) -> std::prelude::v1::Result<Vec<SongResult>, crate::Error> {
    let mut contents = json_crawler.navigate_pointer(concatcp!(
        SINGLE_COLUMN_TAB,
        SECTION_LIST_ITEM,
        MUSIC_SHELF,
        "/contents"
    ))?;
    parse_playlist_items(super::MusicShelfContents {
        json: contents.borrow_mut(),
    })
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
        .map(|r| parse_content_list_artist_subscriptions(r))
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

fn parse_item_list_albums(mut json_crawler: JsonCrawler) -> Result<SearchResultAlbum> {
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

fn parse_content_list_artist_subscriptions(
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
