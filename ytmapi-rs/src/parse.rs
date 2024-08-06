//! Results from parsing Innertube queries.
//! # Implementation example
//! Implementation example is pending refactoring of ProcessedResult to remove
//! leaking external type `serde_json::Value`.
//! See [`crate::json`] for documentation related to the Json type.
//! ```no_run
//! # struct GetDateQuery;
//! use serde::Deserialize;
//! #[derive(Debug, Deserialize)]
//! struct Date {
//!     date_string: String,
//!     date_timestamp: usize,
//! }
//! impl ytmapi_rs::parse::ParseFrom<GetDateQuery> for Date {
//!     fn parse_from(
//!         p: ytmapi_rs::parse::ProcessedResult<GetDateQuery>,
//!     ) -> ytmapi_rs::Result<Self> {
//!         Date::deserialize(p.json)
//!     }
//! }
//! ```
use crate::{
    auth::{AuthToken, BrowserToken, OAuthToken},
    common::{AlbumID, AlbumType, Explicit, PlaylistID, PodcastID, ProfileID, Thumbnail, VideoID},
    crawler::JsonCrawlerBorrowed,
    error,
    json::Json,
    nav_consts::*,
    process::{self, process_flex_column_item},
    query::Query,
    ChannelID,
};
use crate::{RawResult, Result};
use const_format::concatcp;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub use album::*;
pub use artist::*;
pub use history::*;
pub use library::*;
pub use playlists::*;
pub use recommendations::*;
pub use upload::*;

mod album;
mod artist;
mod continuations;
mod history;
mod library;
mod playlists;
mod rate;
mod recommendations;
mod search;
mod upload;

/// Describes how to parse the ProcessedResult from a Query into the target
/// type.
// By requiring ParseFrom to also implement Debug, this simplifies our Query ->
// String API.
pub trait ParseFrom<Q>: Debug + Sized {
    fn parse_from(p: ProcessedResult<Q>) -> crate::Result<Self>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EpisodeDate {
    Live,
    Recorded { date: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EpisodeDuration {
    Live,
    Recorded { duration: String },
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResults {
    pub top_results: Vec<TopResult>,
    pub artists: Vec<SearchResultArtist>,
    pub albums: Vec<SearchResultAlbum>,
    pub featured_playlists: Vec<SearchResultFeaturedPlaylist>,
    pub community_playlists: Vec<SearchResultCommunityPlaylist>,
    pub songs: Vec<SearchResultSong>,
    pub videos: Vec<SearchResultVideo>,
    pub podcasts: Vec<SearchResultPodcast>,
    pub episodes: Vec<SearchResultEpisode>,
    pub profiles: Vec<SearchResultProfile>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Each Top Result has it's own type.
pub enum TopResultType {
    Artist,
    Playlist,
    Song,
    Video,
    Station,
    Podcast,
    #[serde(untagged)]
    Album(AlbumType),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Helper enum for parsing different search result types.
enum SearchResultType {
    #[serde(alias = "Top result")]
    TopResult,
    Artists,
    Albums,
    #[serde(alias = "Featured playlists")]
    FeaturedPlaylists,
    #[serde(alias = "Community playlists")]
    CommunityPlaylists,
    Songs,
    Videos,
    Podcasts,
    Episodes,
    Profiles,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSongArtist {
    pub name: String,
    pub id: Option<ChannelID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSongAlbum {
    pub name: String,
    pub id: AlbumID<'static>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Dynamically defined top result.
/// Some fields are optional as they are not defined for all result types.
// In future, may be possible to make this type safe.
pub struct TopResult {
    pub result_name: String,
    /// Both Videos and Songs can have this left out.
    pub result_type: Option<TopResultType>,
    pub thumbnails: Vec<Thumbnail>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<String>,
    pub year: Option<String>,
    pub subscribers: Option<String>,
    pub plays: Option<String>,
    /// Podcast publisher.
    pub publisher: Option<String>,
    // TODO: Add endpoint id.
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// An artist search result.
pub struct SearchResultArtist {
    pub artist: String,
    /// An artist with no subscribers won't contain this field.
    pub subscribers: Option<String>,
    pub browse_id: ChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A podcast search result.
pub struct SearchResultPodcast {
    pub title: String,
    pub publisher: String,
    pub podcast_id: PodcastID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A podcast episode search result.
pub struct SearchResultEpisode {
    pub title: String,
    pub date: EpisodeDate,
    pub channel_name: String,
    pub video_id: VideoID<'static>,
    // Potentially can include link to channel.
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A video search result. May be a video or a video episode of a podcast.
pub enum SearchResultVideo {
    Video {
        title: String,
        /// Note: Either Youtube channel name, or artist name.
        // Potentially can include link to channel.
        channel_name: String,
        video_id: VideoID<'static>,
        views: String,
        length: String,
        thumbnails: Vec<Thumbnail>,
    },
    VideoEpisode {
        // Potentially asame as SearchResultEpisode
        title: String,
        date: EpisodeDate,
        channel_name: String,
        video_id: VideoID<'static>,
        // Potentially can include link to channel.
        thumbnails: Vec<Thumbnail>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A profile search result.
pub struct SearchResultProfile {
    pub title: String,
    pub username: String,
    pub profile_id: ProfileID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// An album search result.
pub struct SearchResultAlbum {
    pub title: String,
    pub artist: String,
    pub year: String,
    pub explicit: Explicit,
    pub album_id: AlbumID<'static>,
    pub album_type: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultSong {
    // Potentially can include links to artist and album.
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub plays: String,
    pub explicit: Explicit,
    pub video_id: VideoID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// A playlist search result may be a featured or community playlist.
pub enum SearchResultPlaylist {
    Featured(SearchResultFeaturedPlaylist),
    Community(SearchResultCommunityPlaylist),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A community playlist search result.
pub struct SearchResultCommunityPlaylist {
    pub title: String,
    pub author: String,
    pub views: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A featured playlist search result.
pub struct SearchResultFeaturedPlaylist {
    pub title: String,
    pub author: String,
    pub songs: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

/// A result from the api that has been checked for errors and processed into
/// JSON.
pub struct ProcessedResult<'a, Q> {
    pub query: &'a Q,
    /// The raw string output returned from the web request to YouTube.
    pub source: String,
    /// The result once it has been deserialized from Json and processed to
    /// remove errors.
    pub json: Json,
}

impl<'a, Q: Query<A>, A: AuthToken> TryFrom<RawResult<'a, Q, A>> for ProcessedResult<'a, Q> {
    type Error = crate::Error;
    fn try_from(value: RawResult<'a, Q, A>) -> Result<Self> {
        let RawResult {
            json: source,
            query,
            ..
        } = value;
        let json = match source.as_str() {
            // Workaround for Get request returning empty string.
            "" => serde_json::Value::Null,
            other => serde_json::from_str(other)
                .map_err(|e| error::Error::response(format!("{:?}", e)))?,
        };
        let json = Json::new(json);
        Ok(Self {
            query,
            source,
            json,
        })
    }
}

impl<'a, Q> ProcessedResult<'a, Q> {
    pub(crate) fn destructure(self) -> (&'a Q, String, serde_json::Value) {
        let ProcessedResult {
            query,
            source,
            json,
        } = self;
        (query, source, json.inner)
    }
    pub(crate) fn clone_json(self) -> String {
        serde_json::to_string_pretty(&self.json)
            .expect("Serialization of serde_json::value should not fail")
    }
    pub(crate) fn get_json(&self) -> &serde_json::Value {
        &self.json.inner
    }
    // Only required when running tests
    #[cfg(test)]
    pub(crate) fn get_query(&self) -> &Q {
        self.query
    }
}

impl<'a, Q> ProcessedResult<'a, Q> {
    pub fn parse_into<O: ParseFrom<Q>>(self) -> Result<O> {
        O::parse_from(self)
    }
}

// Should take FlexColumnItem? or Data?. Regular serde_json::Value could tryInto
// fixedcolumnitem also. Not sure if this should error.
// XXX: I think this should return none instead of error.
fn parse_song_artists(
    data: &mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<Vec<ParsedSongArtist>> {
    let flex_item_runs =
        process::process_flex_column_item(data, col_idx)?.navigate_pointer("/text/runs")?;
    flex_item_runs
        .into_array_iter_mut()?
        .step_by(2)
        .map(|mut item| parse_song_artist(&mut item))
        .collect()
}

fn parse_song_artist(data: &mut JsonCrawlerBorrowed) -> Result<ParsedSongArtist> {
    Ok(ParsedSongArtist {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
    })
}

fn parse_song_album(data: &mut JsonCrawlerBorrowed, col_idx: usize) -> Result<ParsedSongAlbum> {
    Ok(ParsedSongAlbum {
        name: parse_flex_column_item(data, col_idx, 0)?,
        id: process_flex_column_item(data, col_idx)?
            .take_value_pointer(concatcp!("/text/runs/0", NAVIGATION_BROWSE_ID))?,
    })
}

fn parse_flex_column_item<T: DeserializeOwned>(
    item: &mut JsonCrawlerBorrowed,
    col_idx: usize,
    run_idx: usize,
) -> Result<T> {
    // Consider early return over the and_then calls.
    let pointer = format!("/text/runs/{run_idx}/text");
    process_flex_column_item(item, col_idx)?.take_value_pointer(pointer)
}

mod lyrics {
    use super::{ParseFrom, ProcessedResult};

    use crate::common::browsing::Lyrics;
    use crate::crawler::JsonCrawler;
    use crate::nav_consts::{DESCRIPTION, DESCRIPTION_SHELF, RUN_TEXT, SECTION_LIST_ITEM};
    use crate::query::lyrics::GetLyricsQuery;
    use const_format::concatcp;

    impl<'a> ParseFrom<GetLyricsQuery<'a>> for Lyrics {
        fn parse_from(p: ProcessedResult<GetLyricsQuery<'a>>) -> crate::Result<Self> {
            let json_crawler: JsonCrawler = p.into();
            let mut description_shelf = json_crawler.navigate_pointer(concatcp!(
                "/contents",
                SECTION_LIST_ITEM,
                DESCRIPTION_SHELF
            ))?;
            Ok(Lyrics {
                lyrics: description_shelf.take_value_pointer(DESCRIPTION)?,
                source: description_shelf.take_value_pointer(concatcp!("/footer", RUN_TEXT))?,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::{
            auth::BrowserToken,
            common::{browsing::Lyrics, LyricsID, YoutubeID},
            process_json,
            query::lyrics::GetLyricsQuery,
        };

        #[tokio::test]
        async fn test_get_lyrics_query() {
            // Intro - Notorious BIG - Ready To Die
            let path = std::path::Path::new("./test_json/get_lyrics_20231219.json");
            let file = tokio::fs::read_to_string(path)
                .await
                .expect("Expect file read to pass during tests");
            // Blank query has no bearing on function
            let query = GetLyricsQuery::new(LyricsID::from_raw(""));
            let output = process_json::<_, BrowserToken>(file, query).unwrap();
            assert_eq!(
                output,
                Lyrics {
                    lyrics: "Push \r\nCome on, she almost there push, come on\r\nCome on, come on, push, it's almost there \r\nOne more time, come one\r\nCome on, push, baby, one more time \r\nHarder, harder, push it harder \r\nPush, push, come on \r\nOne more time, here it goes \r\nI see the head\r\nYeah, come on\r\nYeah, yeah\r\nYou did it, baby, yeah\r\n\r\nBut if you lose, don't ask no questions why\r\nThe only game you know is do or die\r\nAh-ha-ha\r\nHard to understand what a hell of a man\r\n\r\nHip hop the hippie the hippie\r\nTp the hip hop and you don't stop \r\nRock it out, baby bubba, to the boogie, the bang-bang\r\nThe boogie to the boogie that be\r\nNow what you hear is not a test, I'm rappin', to the beat \r\n\r\nGoddamn it, Voletta, what the fuck are you doin'?\r\nYou can't control that goddamn boy? (What?)\r\nI just saw Mr. Johnson, he told me he caught the motherfucking boy shoplifting \r\nWhat the fuck are you doing? (Kiss my black ass, motherfucker)\r\nYou can't control that god-, I don't know what the fuck to do with that boy\r\n(What the fuck do you want me to do?)\r\nIf if you can't fucking control that boy, I'ma send him\r\n(All you fucking do is bitch at me)\r\nBitch, bitch, I'ma send his motherfuckin' ass to a group home goddamnit, what?\r\nI'll smack the shit outta you bitch, what, what the fuck?\r\n(Kiss my black ass, motherfucker)\r\nYou're fuckin' up\r\n(Comin' in here smelling like sour socks you, dumb motherfucker) \r\n\r\nWhen I'm bustin' up a party I feel no guilt\r\nGizmo's cuttin' up for thee \r\nSuckers that's down with nei-\r\n\r\nWhat, nigga, you wanna rob them motherfuckin' trains, you crazy? \r\nYes, yes, motherfucker, motherfuckin' right, nigga, yes \r\nNigga, what the fuck, nigga? We gonna get-\r\nNigga, it's eighty-seven nigga, is you dead broke? \r\nYeah, nigga, but, but\r\nMotherfucker, is you broke, motherfucker? \r\nWe need to get some motherfuckin' paper, nigga \r\nNigga it's a train, ain't nobody never robbed no motherfuckin' train \r\nJust listen, man, is your mother givin' you money, nigga? \r\nMy moms don't give me shit nigga, it's time to get paid, nigga \r\nIs you with me? Motherfucker, is you with me? \r\nYeah, I'm with you, nigga, come on \r\nAlright then, nigga, lets make it happen then \r\nAll you motherfuckers get on the fuckin' floor \r\nGet on the motherfuckin' floor\r\nChill, give me all your motherfuckin' money \r\nAnd don't move, nigga\r\nI want the fuckin' jewelry \r\nGive me every fuckin' thing \r\nNigga, I'd shut the fuck up or I'ma blow your motherfuckin' brains out \r\nShut the fuck up, bitch, give me your fuckin' money, motherfucker\r\nFuck you, bitch, get up off that shit \r\nWhat the fuck you holdin' on to that shit for, bitch? \r\n\r\nI get money, money I got\r\nStunts call me honey if they feel real hot\r\n\r\nOpen C-74, Smalls \r\nMr. Smalls, let me walk you to the door \r\nSo how does it feel leavin' us? \r\nCome on, man, what kind of fuckin' question is that, man? \r\nTryin' to get the fuck up out this joint, dog \r\nYeah, yeah, you'll be back \r\nYou niggas always are \r\nGo ahead, man, what the fuck is you hollerin' about? \r\nYou won't see me up in this motherfucker no more \r\nWe'll see \r\nI got big plans nigga, big plans, hahaha".to_string(),
                    source: "Source: LyricFind".to_string()
                }
            );
        }
    }
}
mod watch {
    use const_format::concatcp;

    use crate::{
        common::watch::WatchPlaylist,
        crawler::{JsonCrawler, JsonCrawlerBorrowed},
        nav_consts::{NAVIGATION_PLAYLIST_ID, TAB_CONTENT},
        query::watch::{GetWatchPlaylistQuery, GetWatchPlaylistQueryID},
        Result,
    };

    use super::{ParseFrom, ProcessedResult};

    impl<T: GetWatchPlaylistQueryID> ParseFrom<GetWatchPlaylistQuery<T>> for WatchPlaylist {
        fn parse_from(p: ProcessedResult<GetWatchPlaylistQuery<T>>) -> crate::Result<Self> {
            // TODO: Continuations
            let json_crawler: JsonCrawler = p.into();
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
    // XXX: Only used here!
    fn get_tab_browse_id<'a>(
        watch_next_renderer: &'a mut JsonCrawlerBorrowed,
        tab_id: usize,
    ) -> Result<JsonCrawlerBorrowed<'a>> {
        // TODO: Safe option that returns none if tab doesn't exist.
        let path = format!("/tabs/{tab_id}/tabRenderer/endpoint/browseEndpoint/browseId");
        watch_next_renderer.borrow_pointer(path)
    }
}
mod song {
    use super::ParseFrom;
    use crate::{
        common::SongTrackingUrl, crawler::JsonCrawler, query::song::GetSongTrackingUrlQuery,
    };

    impl<'a> ParseFrom<GetSongTrackingUrlQuery<'a>> for SongTrackingUrl<'static> {
        fn parse_from(
            p: super::ProcessedResult<GetSongTrackingUrlQuery<'a>>,
        ) -> crate::Result<Self> {
            let mut crawler = JsonCrawler::from(p);
            crawler.take_value_pointer("/playbackTracking/videostatsPlaybackUrl/baseUrl")
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::{
            auth::BrowserToken,
            common::{SongTrackingUrl, YoutubeID},
            query::song::GetSongTrackingUrlQuery,
            VideoID,
        };

        #[tokio::test]
        async fn test_get_song_tracking_url_query() {
            let output = SongTrackingUrl::from_raw("https://s.youtube.com/api/stats/playback?cl=655300395&docid=FZ8BxMU3BYc&ei=JSimZqHaNeyB9fwP9oqh0Ak&fexp=&ns=yt&plid=AAYeTNocW-liNkl6&el=detailpage&len=193&of=URbTjA0hNUiM-oZxeU_KzQ&osid=AAAAAYfxXtM%3AAOeUNAZhCDiglWHfELd4I0ksz0dyuGtLVg&uga=m32&vm=CAMQARgBOjJBSHFpSlRJMDQteFk3b0Z2MUZXblN3NTlza3ZKcEhkcXpWeVhhMXl4RGQyZXVFR2twZ2JiQU9BckJGdG4zbDdCcElKTGJHNkt3dlJVX2ZzZGdKMndGR1ZZdk92MVItWWYtUTBOYmdFQnYxd3J6cGJBNzdrZUJXMlQ0QWR4MVo4S1Rza1JTM0hvWGRTd2llYk5xZFd6Nne4AQE");
            parse_test_value!(
                "./test_json/get_song_tracking_url_20240728.json",
                output,
                GetSongTrackingUrlQuery::new(VideoID::from_raw("")).unwrap(),
                BrowserToken
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::SearchQuery;

    #[tokio::test]
    async fn test_all_processed_impl() {
        let query = SearchQuery::new("Beatles");
        let source = "{\"name\": \"John Doe\"}".to_string();
        let p = ProcessedResult {
            query: &query,
            source: source.clone(),
            json: serde_json::from_str(source.as_str()).unwrap(),
        };
        assert_eq!(&query, p.get_query());
    }
}
