use crate::{
    common::{
        AlbumID, AlbumType, BrowseID, Explicit, PlaylistID, PlaylistType, Thumbnail, VideoID,
        YoutubeID,
    },
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::*,
    process::{self, process_flex_column_item},
    query::Query,
    ChannelID,
};
use crate::{Error, Result};
pub use album::*;
pub use artist::*;
use const_format::concatcp;
pub use continuations::*;
pub use search::*;

mod album;
mod artist;
mod continuations;
mod search;

#[derive(Debug, Clone)]
pub enum SearchResult<'a> {
    TopResult,
    Song(SearchResultSong<'a>),
    Album(SearchResultAlbum<'a>),
    Playlist(SearchResultPlaylist<'a>),
    Video,
    Artist(SearchResultArtist),
}

#[derive(Debug, Clone)]
pub struct ParsedSongArtist {
    name: String,
    id: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct ParsedSongAlbum {
    pub name: Option<String>,
    id: Option<String>,
}
#[derive(Debug)]
pub struct TopResult {
    result_type: SearchResultType,
    subscribers: Option<String>,
    thumbnails: Option<String>, //own type?
    // XXX: more to come
    artist_info: Option<ParsedSongList>,
}
#[derive(Debug)]
pub struct ParsedSongList {
    artists: Vec<ParsedSongArtist>,
    album: Option<ParsedSongAlbum>,
    views: Option<String>,
    duration: Option<String>, // TODO: Duration as a time
    year: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchResultArtist {
    pub artist: String,
    // Given by calling function, consider removing.
    // pub category: String,
    pub browse_id: Option<ChannelID<'static>>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone)]
pub struct SearchResultAlbum<'a> {
    pub title: String,
    pub artist: String,
    pub year: u32,
    pub explicit: Explicit,
    pub browse_id: Option<ChannelID<'a>>,
    pub album_type: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone)]
pub struct SearchResultSong<'a> {
    pub title: String,
    pub artists: Vec<ParsedSongArtist>,
    pub album: ParsedSongAlbum,
    pub explicit: Explicit,
    pub video_id: Option<VideoID<'a>>,
    pub album_type: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
    pub feedback_tockens: FeedbackTokens,
}

#[derive(Debug, Clone)]
pub struct SearchResultPlaylist<'a> {
    pub title: String,
    pub author: Option<String>,
    pub playlist_type: PlaylistType,
    pub playlist_id: Option<PlaylistID<'a>>,
    pub item_count: u32,
}

#[derive(Debug, Clone)]
pub struct FeedbackTokens;

pub struct ProcessedResult<T>
where
    T: Query,
{
    query: T,
    json_crawler: JsonCrawler,
}
impl<T: Query> ProcessedResult<T> {
    pub fn from_raw(json_crawler: JsonCrawler, query: T) -> Self {
        Self {
            query,
            json_crawler,
        }
    }
    pub fn get_query(&self) -> &T {
        &self.query
    }
    pub fn get_crawler(&self) -> &JsonCrawler {
        &self.json_crawler
    }
}

// Should take FlexColumnItem? or Data?. Regular serde_json::Value could tryInto fixedcolumnitem also.
// Not sure if this should error.
// XXX: I think this should return none instead of error.
fn parse_song_artists(
    data: &mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<Vec<ParsedSongArtist>> {
    let mut artists = Vec::new();
    let Ok(flex_items) = process::process_flex_column_item(data, col_idx) else {
        return Ok(artists);
    };
    let Ok(flex_items_runs) = flex_items.navigate_pointer("/text/runs") else {
        return Ok(artists);
    };
    // https://github.com/sigma67/ytmusicapi/blob/master/ytmusicapi/parsers/songs.py
    // parse_song_artists_runs
    for mut i in flex_items_runs
        .into_array_iter_mut()
        .into_iter()
        .flatten()
        .step_by(2)
    {
        artists.push(ParsedSongArtist {
            name: i.take_value_pointer("/text")?,
            id: i.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
        });
    }
    Ok(artists)
}

fn parse_song_album(data: &mut JsonCrawlerBorrowed, col_idx: usize) -> Result<ParsedSongAlbum> {
    Ok(ParsedSongAlbum {
        name: parse_item_text(data, col_idx, 0).ok(),
        id: process_flex_column_item(data, col_idx)?
            .take_value_pointer(concatcp!("/text/runs/0", NAVIGATION_BROWSE_ID))
            .ok(),
    })
}

// Maybe doesn't need to be function
pub fn parse_thumbnails(thumbnails: &mut JsonCrawlerBorrowed) -> crate::Result<Vec<Thumbnail>> {
    let mut thumb_array = Vec::new();
    for mut thumb_json in thumbnails.as_array_iter_mut()? {
        let thumb = thumb_json.take_value()?;
        thumb_array.push(thumb)
    }
    Ok(thumb_array)
}

fn parse_item_text(
    item: &mut JsonCrawlerBorrowed,
    col_idx: usize,
    run_idx: usize,
) -> Result<String> {
    // Consider early return over the and_then calls.
    let pointer = format!("/text/runs/{run_idx}/text");
    process_flex_column_item(item, col_idx)?.take_value_pointer(pointer)
}

// Looks to only do Artists currently
pub fn parse_search_results<'a>(results: JsonCrawlerBorrowed) -> Result<Vec<SearchResult<'a>>> {
    results
        .into_array_iter_mut()?
        .map(|r| {
            r.navigate_pointer("/musicResponsiveListItemRenderer")
                .and_then(|r| parse_search_result(r, SearchResultType::Artist))
        })
        .collect()
}

// Currently only searches and returns artists.
// TODO: i18n
pub fn parse_search_result<'a>(
    mut data: JsonCrawlerBorrowed,
    _category: SearchResultType,
) -> Result<SearchResult<'a>> {
    // Unsure what this does
    //        default_offset = (not result_type) * 2
    let video_type = data.take_value_pointer::<String, &str>(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        NAVIGATION_VIDEO_TYPE
    ));
    let result_type = match video_type.as_deref() {
        Ok("MUSIC_VIDEO_TYPE_ATV") => SearchResultType::Song,
        Ok(_) => SearchResultType::Video,
        // Note - ASCII lowercase function only here.
        // Should use the try_from method on SearchResultType.
        Err(_) => match parse_item_text(&mut data, 1, 0)?
            .to_ascii_lowercase()
            .as_str()
        {
            "artist" => SearchResultType::Artist,
            "station" => SearchResultType::Station,
            "video" => SearchResultType::Video,
            "song" => SearchResultType::Song,
            "playlist" => SearchResultType::Playlist,
            // Likely one of the multiple "Album" types.
            x => todo!("result type {x} not implemented yet"),
        },
    };
    let _title = match result_type {
        SearchResultType::Artist => None,
        _ => Some(parse_item_text(&mut data, 0, 0)?),
    };
    // Will this find none and error? Note from previously.
    let artist = match result_type {
        //below is some bs with side effects. Don't do it.
        //parse_menu_playlists(data, search_result);
        SearchResultType::Artist => Some(parse_item_text(&mut data, 0, 0)?),
        _ => None,
    };
    let browse_id = data
        .take_value_pointer::<String, &str>(NAVIGATION_BROWSE_ID)
        .map(|s| ChannelID::from_raw(s))
        .ok();
    let thumbnails = data
        .navigate_pointer(THUMBNAILS)
        .and_then(|mut t| parse_thumbnails(&mut t))?;
    let search_result = match result_type {
        SearchResultType::Artist => {
            // TODO: Fix this shit
            let artist = artist
                .ok_or_else(|| Error::other("Artist wasn't found, but it's a required field."))?;
            SearchResult::Artist(SearchResultArtist {
                artist,
                thumbnails,
                // category is given by the calling function. Not sure if we need it here.
                // category,
                browse_id,
            })
        }
        #[allow(unreachable_code, unused_variables)]
        SearchResultType::Album(album_type) => {
            let artist = todo!();
            let year = todo!();
            let title = todo!();
            let explicit = todo!();
            SearchResult::Album(SearchResultAlbum {
                artist,
                browse_id,
                year,
                title,
                explicit,
                album_type,
                thumbnails,
            })
        }
        // Should Playlist take the type in the enum definition?
        #[allow(unreachable_code, unused_variables)]
        SearchResultType::Playlist => {
            let author = todo!();
            let item_count = todo!();
            let title = todo!();
            let playlist_type = todo!();
            SearchResult::Playlist(SearchResultPlaylist {
                playlist_id: browse_id
                    .as_ref()
                    .map(|id| PlaylistID::from_raw(id.get_raw())),
                title,
                author,
                item_count,
                playlist_type,
            })
        }
        _ => todo!("type not yet implemented"),
    };
    Ok(search_result)
}

pub fn parse_top_result(mut data: JsonCrawlerBorrowed) -> Result<TopResult> {
    // Should be if-let?
    // XXX: The artist from the call to nav has quotation marks around it, causes error when
    // calleing get_search_result_type. I fix this with a hack.
    // TODO: i18n - search results can be in a different language.
    let st: String = data.take_value_pointer(SUBTITLE)?;
    let result_type = SearchResultType::try_from(&st)?;
    let _category = data.take_value_pointer(CARD_SHELF_TITLE)?;
    let thumbnails = data.take_value_pointer(THUMBNAILS).ok();
    let subscribers = if let SearchResultType::Artist = result_type {
        // TODO scrub / split subscribers.
        data.take_value_pointer(SUBTITLE2).ok()
    } else {
        todo!("Only handles Artist currently");
    };

    // TODO: artist_info
    let artist_info = Some(parse_song_runs(&data._take_json_pointer("/title/runs")?)?);
    Ok(TopResult {
        subscribers,
        result_type,
        thumbnails,
        artist_info,
    })
}

fn parse_song_runs(runs: &serde_json::Value) -> Result<ParsedSongList> {
    let mut artists = Vec::new();
    let year = None;
    let mut album = None;
    let views = None;
    let duration = None;
    if let serde_json::Value::Array(a) = runs {
        for (i, r) in a.iter().enumerate() {
            // Uneven items are always separators
            if (i % 2) == 1 {
                continue;
            }
            // TODO: Handle None
            let text = r.get("text").unwrap().to_string();
            // TODO: Handle None
            if let serde_json::Value::Object(_) = r.get("navigationEndpoint").unwrap() {
                // XXX: Is this definitely supposed to be an if let?
                let name = text;
                let id = r.pointer(NAVIGATION_BROWSE_ID).map(|id| id.to_string());
                // album
                // TODO: Cleanup unnecessary allocation
                if id
                    .clone()
                    .map_or(false, |item_id| item_id.contains("release_detail"))
                    || id
                        .clone()
                        .map_or(false, |item_id| item_id.starts_with("MPRE"))
                {
                    album = Some(ParsedSongAlbum {
                        name: Some(name),
                        id,
                    });
                } else {
                    //artist
                    artists.push(ParsedSongArtist { id, name });
                }
            } else {
                // XXX: Note, if artist doesn't have ID, will end up here and panic.
                todo!("Handle non artists or albums");
            }
        }
    } else {
        unreachable!("Assume input is valid");
    }
    Ok(ParsedSongList {
        artists,
        year,
        album,
        views,
        duration,
    })
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::query::SearchQuery;

    use super::*;

    #[tokio::test]
    async fn test_all_processed_impl() {
        let query = SearchQuery::new("Beatles");
        let json_crawler = JsonCrawler::from_json(json!({"name": "John Doe"}));
        let json_crawler_clone = json_crawler.clone();
        let raw = ProcessedResult::from_raw(json_crawler, query.clone());
        assert_eq!(&query, raw.get_query());
        assert_eq!(&json_crawler_clone, raw.get_crawler());
    }
}

mod lyrics {
    use const_format::concatcp;

    use crate::common::browsing::Lyrics;
    use crate::nav_consts::{DESCRIPTION, DESCRIPTION_SHELF, RUN_TEXT, SECTION_LIST_ITEM};
    use crate::query::lyrics::GetLyricsQuery;
    use crate::Result;

    use super::ProcessedResult;

    impl<'a> ProcessedResult<GetLyricsQuery<'a>> {
        pub fn parse(self) -> Result<Lyrics> {
            let ProcessedResult { json_crawler, .. } = self;
            let mut description_shelf = json_crawler.navigate_pointer(concatcp!(
                "/contents",
                SECTION_LIST_ITEM,
                DESCRIPTION_SHELF
            ))?;
            Ok(Lyrics::new(
                description_shelf.take_value_pointer(DESCRIPTION)?,
                description_shelf.take_value_pointer(concatcp!("/footer", RUN_TEXT))?,
            ))
        }
    }
}
mod library {
    use super::ProcessedResult;
    use crate::common::library::Playlist;
    use crate::common::PlaylistID;
    use crate::crawler::JsonCrawler;
    use crate::nav_consts::{
        GRID, ITEM_SECTION, MTRIR, NAVIGATION_BROWSE_ID, SECTION_LIST, SECTION_LIST_ITEM,
        SINGLE_COLUMN_TAB, THUMBNAIL_RENDERER, TITLE, TITLE_TEXT,
    };
    use crate::query::library::GetLibraryPlaylistQuery;
    use crate::{Result, Thumbnail};
    use const_format::concatcp;

    impl<'a> ProcessedResult<GetLibraryPlaylistQuery> {
        // TODO: Continuations
        // TODO: Implement count and author fields
        pub fn parse(self) -> Result<Vec<Playlist>> {
            let ProcessedResult { json_crawler, .. } = self;
            parse_library_playlist_query(json_crawler)
        }
    }

    fn parse_library_playlist_query(mut json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
        if let Some(contents) = process_library_contents(json_crawler) {
            parse_content_list_playlist(contents)
        } else {
            Ok(Vec::new())
        }
    }

    // Consider returning ProcessedLibraryContents
    fn process_library_contents(mut json_crawler: JsonCrawler) -> Option<JsonCrawler> {
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

    fn parse_content_list_playlist(mut json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
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

    mod tests {
        use serde_json::json;

        use crate::{
            common::{library::Playlist, PlaylistID, YoutubeID},
            crawler::JsonCrawler,
            parse::ProcessedResult,
            query::library::GetLibraryPlaylistQuery,
        };

        // Consider if the parse function itself should be removed from impl.
        #[test]
        fn test_standard_json() {
            let testfile = std::fs::read_to_string("test_json/get_library_playlists.json").unwrap();
            let testfile_json = serde_json::from_str(&testfile).unwrap();
            let json_crawler = JsonCrawler::from_json(testfile_json);
            let processed = ProcessedResult::from_raw(json_crawler, GetLibraryPlaylistQuery {});
            let result = processed.parse().unwrap();
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
    }
}
mod watch {
    use const_format::concatcp;

    use crate::{
        common::watch::WatchPlaylist,
        crawler::JsonCrawlerBorrowed,
        nav_consts::{NAVIGATION_PLAYLIST_ID, TAB_CONTENT},
        query::watch::GetWatchPlaylistQuery,
        Result, VideoID,
    };

    use super::ProcessedResult;

    impl<'a> ProcessedResult<GetWatchPlaylistQuery<VideoID<'a>>> {
        // TODO: Continuations
        pub fn parse(self) -> Result<WatchPlaylist> {
            let ProcessedResult { json_crawler, .. } = self;
            let mut watch_next_renderer = json_crawler.navigate_pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer")?;
            let lyrics_id =
                get_tab_browse_id(&mut watch_next_renderer.borrow_mut(), 1)?.take_value()?;
            let mut results = watch_next_renderer.navigate_pointer(concatcp!(
                TAB_CONTENT,
                "/musicQueueRenderer/content/playlistPanelRenderer/contents"
            ))?;
            let playlist_id = results.as_array_iter_mut()?.find_map(|mut v| {
                v.take_value_pointer(concatcp!(
                    "/playlistPanelVideoRenderer",
                    NAVIGATION_PLAYLIST_ID
                ))
                .ok()
            });
            Ok(WatchPlaylist::new(playlist_id, lyrics_id))
        }
    }

    // Should be a Process function not Parse.
    fn get_tab_browse_id<'a>(
        watch_next_renderer: &'a mut JsonCrawlerBorrowed,
        tab_id: usize,
    ) -> Result<JsonCrawlerBorrowed<'a>> {
        // TODO: Safe option that returns none if tab doesn't exist.
        let path = format!("/tabs/{tab_id}/tabRenderer/endpoint/browseEndpoint/browseId");
        watch_next_renderer.borrow_pointer(path)
    }
}
