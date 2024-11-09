use super::{parse_flex_column_item, ParseFrom, ProcessedResult, DISPLAY_POLICY};
use crate::common::{
    AlbumID, AlbumType, ArtistChannelID, EpisodeID, Explicit, PlaylistID, PodcastID, ProfileID,
    SearchSuggestion, SuggestionType, TextRun, Thumbnail, VideoID,
};
use crate::nav_consts::{
    BADGE_LABEL, LIVE_BADGE_LABEL, MUSIC_CARD_SHELF, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
    PLAYLIST_ITEM_VIDEO_ID, PLAY_BUTTON, SECTION_LIST, SUBTITLE, SUBTITLE2, TAB_CONTENT,
    THUMBNAILS, TITLE_TEXT,
};
use crate::parse::EpisodeDate;
use crate::process::flex_column_item_pointer;
use crate::query::*;
use crate::youtube_enums::PlaylistEndpointParams;
use crate::{Error, Result};
use const_format::concatcp;
use filteredsearch::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    FilteredSearch, FilteredSearchType, PlaylistsFilter, PodcastsFilter, ProfilesFilter,
    SongsFilter, VideosFilter,
};
use itertools::Itertools;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
/// Dynamically defined top result.
/// Some fields are optional as they are not defined for all result types.
// In future, may be possible to make this type safe.
// TODO: Add endpoint id.
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
    /// Generic tagline that can appear on top results
    pub byline: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// An artist search result.
pub struct SearchResultArtist {
    pub artist: String,
    /// An artist with no subscribers won't contain this field.
    pub subscribers: Option<String>,
    pub browse_id: ArtistChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A podcast search result.
pub struct SearchResultPodcast {
    pub title: String,
    pub publisher: String,
    pub podcast_id: PodcastID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A podcast episode search result.
pub struct SearchResultEpisode {
    pub title: String,
    pub date: EpisodeDate,
    pub channel_name: String,
    pub episode_id: EpisodeID<'static>,
    // Potentially can include link to channel.
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A video search result. May be a video or a video episode of a podcast.
pub enum SearchResultVideo {
    #[non_exhaustive]
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
    #[non_exhaustive]
    VideoEpisode {
        // Potentially asame as SearchResultEpisode
        title: String,
        date: EpisodeDate,
        channel_name: String,
        episode_id: EpisodeID<'static>,
        // Potentially can include link to channel.
        thumbnails: Vec<Thumbnail>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A profile search result.
pub struct SearchResultProfile {
    pub title: String,
    pub username: String,
    pub profile_id: ProfileID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct SearchResultSong {
    // Potentially can include links to artist and album.
    pub title: String,
    pub artist: String,
    // Album field can be optional - see https://github.com/nick42d/youtui/issues/174
    pub album: Option<String>,
    pub duration: String,
    pub plays: String,
    pub explicit: Explicit,
    pub video_id: VideoID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
// A playlist search result may be a featured or community playlist.
pub enum SearchResultPlaylist {
    Featured(SearchResultFeaturedPlaylist),
    Community(SearchResultCommunityPlaylist),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A community playlist search result.
pub struct SearchResultCommunityPlaylist {
    pub title: String,
    pub author: String,
    pub views: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A featured playlist search result.
pub struct SearchResultFeaturedPlaylist {
    pub title: String,
    pub author: String,
    pub songs: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

// TODO: Type safety
fn parse_basic_search_result_from_section_list_contents(
    mut section_list_contents: BasicSearchSectionListContents,
) -> Result<SearchResults> {
    // Imperative solution, may be able to make more functional.
    let mut top_results = Vec::new();
    let mut artists = Vec::new();
    let mut albums = Vec::new();
    let mut featured_playlists = Vec::new();
    let mut community_playlists = Vec::new();
    let mut songs = Vec::new();
    let mut videos = Vec::new();
    let mut podcasts = Vec::new();
    let mut episodes = Vec::new();
    let mut profiles = Vec::new();

    let music_card_shelf = section_list_contents
        .0
        .try_iter_mut()?
        .find_path(MUSIC_CARD_SHELF)
        .ok();
    if let Some(music_card_shelf) = music_card_shelf {
        top_results = parse_top_results_from_music_card_shelf_contents(music_card_shelf)?
    }
    let results_iter = section_list_contents
        .0
        .try_into_iter()?
        .filter_map(|item| item.navigate_pointer(MUSIC_SHELF).ok());

    for mut category in results_iter {
        match category.take_value_pointer::<SearchResultType>(TITLE_TEXT)? {
            SearchResultType::TopResult => {
                top_results = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .filter_map(|r| parse_top_result_from_music_shelf_contents(r).transpose())
                    .collect::<Result<Vec<TopResult>>>()?;
            }
            // TODO: Use a navigation constant
            SearchResultType::Artists => {
                artists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_artist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultArtist>>>()?;
            }
            SearchResultType::Albums => {
                albums = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_album_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultAlbum>>>()?
            }
            SearchResultType::FeaturedPlaylists => {
                featured_playlists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_featured_playlist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultFeaturedPlaylist>>>()?
            }
            SearchResultType::CommunityPlaylists => {
                community_playlists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_community_playlist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultCommunityPlaylist>>>()?
            }
            SearchResultType::Songs => {
                songs = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_song_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultSong>>>()?
            }
            SearchResultType::Videos => {
                videos = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .filter_map(|r| {
                        parse_video_search_result_from_music_shelf_contents(r).transpose()
                    })
                    .collect::<Result<Vec<SearchResultVideo>>>()?
            }
            SearchResultType::Podcasts => {
                podcasts = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_podcast_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultPodcast>>>()?
            }
            SearchResultType::Episodes => {
                episodes = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_episode_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultEpisode>>>()?
            }
            SearchResultType::Profiles => {
                profiles = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_profile_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultProfile>>>()?
            }
        }
    }
    Ok(SearchResults {
        top_results,
        artists,
        albums,
        featured_playlists,
        community_playlists,
        songs,
        videos,
        podcasts,
        episodes,
        profiles,
    })
}

fn parse_top_results_from_music_card_shelf_contents(
    mut music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Vec<TopResult>> {
    let mut results = Vec::new();
    // Begin - first result parsing
    let result_name = music_shelf_contents.take_value_pointer(TITLE_TEXT)?;
    let subtitle: String = music_shelf_contents.take_value_pointer(SUBTITLE)?;
    let subtitle_2: Option<String> = music_shelf_contents.take_value_pointer(SUBTITLE2).ok();
    // Deserialize without taking ownership of subtitle - not possible with
    // JsonCrawler::take_value_pointer().
    // TODO: add methods like borrow_value_pointer() to JsonCrawler.
    let result_type_result: std::result::Result<_, serde::de::value::Error> =
        TopResultType::deserialize(subtitle.as_str().into_deserializer());
    let result_type = result_type_result.ok();
    // Possibly artists only.
    let subscribers = subtitle_2;
    let byline = match result_type {
        Some(_) => None,
        None => Some(subtitle),
    };
    // Imperative solution, may be able to make more functional.
    let publisher = None;
    let artist = None;
    let album = None;
    let duration = None;
    let year = None;
    let plays = None;
    let thumbnails: Vec<Thumbnail> = music_shelf_contents.take_value_pointer(THUMBNAILS)?;
    let first_result = TopResult {
        // Assuming that in non-card case top result always has a result type.
        result_type,
        subscribers,
        thumbnails,
        result_name,
        publisher,
        artist,
        album,
        duration,
        year,
        plays,
        byline,
    };
    // End - first result parsing.
    results.push(first_result);
    // Other results may not exist.
    if let Ok(mut contents) = music_shelf_contents.navigate_pointer("/contents") {
        contents
            .try_iter_mut()?
            .filter_map(|r| parse_top_result_from_music_shelf_contents(r).transpose())
            .try_for_each(|r| -> Result<()> {
                results.push(r?);
                Ok(())
            })?;
    }
    Ok(results)
}
// TODO: Tests
fn parse_top_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Option<TopResult>> {
    // This is the "More from YouTube" seperator
    if music_shelf_contents.path_exists("/messageRenderer") {
        return Ok(None);
    };
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let result_name = parse_flex_column_item(&mut mrlir, 0, 0)?;
    // It's possible to have artist name in the first position instead of a
    // TopResultType. There may be a way to differentiate this even further.
    let flex_1_0: String = parse_flex_column_item(&mut mrlir, 1, 0)?;
    // Deserialize without taking ownership of flex_1_0 - not possible with
    // JsonCrawler::take_value_pointer().
    // TODO: add methods like borrow_value_pointer() to JsonCrawler.
    let result_type_result: std::result::Result<_, serde::de::value::Error> =
        TopResultType::deserialize(flex_1_0.as_str().into_deserializer());
    let result_type = result_type_result.ok();
    // Imperative solution, may be able to make more functional.
    let mut subscribers = None;
    let mut publisher = None;
    let mut artist = None;
    let mut album = None;
    let mut duration = None;
    let mut year = None;
    let mut plays = None;
    match result_type {
        // XXX: Perhaps also populate Artist field.
        Some(TopResultType::Artist) => {
            subscribers = Some(parse_flex_column_item(&mut mrlir, 1, 2)?)
        }
        Some(TopResultType::Album(_)) => {
            // XXX: Perhaps also populate Album field.
            artist = Some(parse_flex_column_item(&mut mrlir, 1, 2)?);
            year = Some(parse_flex_column_item(&mut mrlir, 1, 4)?);
        }
        Some(TopResultType::Playlist) => todo!(),
        Some(TopResultType::Song) => {
            artist = Some(parse_flex_column_item(&mut mrlir, 1, 2)?);
            album = Some(parse_flex_column_item(&mut mrlir, 1, 4)?);
            duration = Some(parse_flex_column_item(&mut mrlir, 1, 6)?);
            // This does not show up in all Card renderer results and so we'll define it as
            // optional. TODO: Could make this more type safe in future.
            plays = parse_flex_column_item(&mut mrlir, 1, 8).ok();
        }
        Some(TopResultType::Video) => todo!(),
        Some(TopResultType::Station) => todo!(),
        Some(TopResultType::Podcast) => publisher = Some(parse_flex_column_item(&mut mrlir, 1, 2)?),
        None => {
            artist = Some(flex_1_0);
            album = Some(parse_flex_column_item(&mut mrlir, 1, 2)?);
            duration = Some(parse_flex_column_item(&mut mrlir, 1, 4)?);
            // This does not show up in all Card renderer results and so we'll define it as
            // optional. TODO: Could make this more type safe in future.
            plays = parse_flex_column_item(&mut mrlir, 1, 6).ok();
        }
    }
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(Some(TopResult {
        result_type,
        subscribers,
        thumbnails,
        result_name,
        publisher,
        artist,
        album,
        duration,
        year,
        plays,
        byline: None,
    }))
}
// TODO: Type safety
// TODO: Tests
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let artist = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let subscribers = parse_flex_column_item(&mut mrlir, 1, 2).ok();
    let browse_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultArtist {
        artist,
        subscribers,
        thumbnails,
        browse_id,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_profile_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultProfile> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let username = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let profile_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultProfile {
        title,
        username,
        profile_id,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_album_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultAlbum> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let album_type = parse_flex_column_item(&mut mrlir, 1, 0)?;

    // Artist can comprise of multiple runs, delimited by " • ".
    // See https://github.com/nick42d/youtui/issues/171
    let (artist, year) = mrlir
        .borrow_pointer(format!("{}/text/runs", flex_column_item_pointer(1)))?
        .try_expect(
            "album result should contain 3 string fields delimited by ' • '",
            |flex_column_1| {
                Ok(flex_column_1
                    .try_iter_mut()?
                    // First field is album_type which we parsed above, so skip it and the
                    // delimiter.
                    .skip(2)
                    .map(|mut field| field.take_value_pointer::<String>("/text"))
                    .collect::<json_crawler::CrawlerResult<String>>()?
                    .split(" • ")
                    .map(ToString::to_string)
                    .collect_tuple::<(String, String)>())
            },
        )?;

    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let browse_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultAlbum {
        artist,
        thumbnails,
        album_id: browse_id,
        title,
        year,
        album_type,
        explicit,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_song_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultSong> {
    // The byline comprises multiple fields delimited by " • ".
    // See https://github.com/nick42d/youtui/issues/171.
    // Album field is optional. See https://github.com/nick42d/youtui/issues/174
    /// Tuple makeup: (artist, album, duration)
    fn parse_song_fields(
        mrlir: &mut impl JsonCrawler,
    ) -> json_crawler::CrawlerResult<Option<(String, Option<String>, String)>> {
        let mut fields_vec = mrlir
            .try_iter_mut()?
            .map(|mut field| field.take_value_pointer::<String>("/text"))
            .collect::<json_crawler::CrawlerResult<String>>()?
            .rsplit(" • ")
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let Some(artist) = fields_vec.pop() else {
            return Ok(None);
        };
        let Some(album_or_duration) = fields_vec.pop() else {
            return Ok(None);
        };
        if let Some(duration) = fields_vec.pop() {
            return Ok(Some((artist, Some(album_or_duration), duration)));
        }
        Ok(Some((artist, None, album_or_duration)))
    }

    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;

    let (artist, album, duration) = mrlir
        .borrow_pointer(format!("{}/text/runs", flex_column_item_pointer(1)))?
        .try_expect(
            "Song result should contain 2 or 3 string fields delimited by ' • '",
            parse_song_fields,
        )?;

    let plays = parse_flex_column_item(&mut mrlir, 2, 0)?;

    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultSong {
        artist,
        thumbnails,
        title,
        explicit,
        plays,
        album,
        video_id,
        duration,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_video_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Option<SearchResultVideo>> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Handle not available case
    if let Ok("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") = mrlir
        .take_value_pointer::<String>(DISPLAY_POLICY)
        .as_deref()
    {
        return Ok(None);
    };
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let first_field: String = parse_flex_column_item(&mut mrlir, 1, 0)?;
    // Handle video podcasts - seems to be 2 different ways to display these.
    match first_field.as_str() {
        "Video" => {
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let views = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let length = parse_flex_column_item(&mut mrlir, 1, 6)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::Video {
                title,
                channel_name,
                views,
                length,
                thumbnails,
                video_id,
            }))
        }
        "Episode" => {
            //TODO: Handle live episode
            let date = EpisodeDate::Recorded {
                date: parse_flex_column_item(&mut mrlir, 1, 2)?,
            };
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::VideoEpisode {
                title,
                channel_name,
                date,
                thumbnails,
                episode_id: video_id,
            }))
        }
        _ => {
            // Assume that if a watch endpoint exists, it's a video.
            if mrlir.path_exists("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint") {

            let views = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let length = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::Video {
                            title,
                            channel_name: first_field,
                            views,
                            length,
                            thumbnails,
                            video_id,
                        }))
            } else {
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::VideoEpisode {
                            title,
                            channel_name,
                        //TODO: Handle live episode
                            date: EpisodeDate::Recorded { date: first_field },
                            thumbnails,
                            episode_id: video_id,
                        }))
            }
        }
    }
}
// TODO: Type safety
// TODO: Tests
fn parse_podcast_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPodcast> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let publisher = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let podcast_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultPodcast {
        title,
        publisher,
        podcast_id,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_episode_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultEpisode> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let date = if mrlir.path_exists(LIVE_BADGE_LABEL) {
        EpisodeDate::Live
    } else {
        EpisodeDate::Recorded {
            date: parse_flex_column_item(&mut mrlir, 1, 0)?,
        }
    };
    let channel_name = match date {
        EpisodeDate::Live => parse_flex_column_item(&mut mrlir, 1, 0)?,
        EpisodeDate::Recorded { .. } => parse_flex_column_item(&mut mrlir, 1, 2)?,
    };
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultEpisode {
        title,
        date,
        episode_id: video_id,
        channel_name,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_featured_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultFeaturedPlaylist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let author = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let songs = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let playlist_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultFeaturedPlaylist {
        title,
        author,
        playlist_id,
        songs,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_community_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultCommunityPlaylist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let author = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let views = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let playlist_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultCommunityPlaylist {
        title,
        author,
        playlist_id,
        views,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
// TODO: Generalize using other parse functions.
fn parse_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPlaylist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let author = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let playlist_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    // The playlist search contains a mix of Community and Featured playlists.
    let playlist_params: PlaylistEndpointParams = mrlir.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchPlaylistEndpoint/params"
    ))?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    let playlist = match playlist_params {
        PlaylistEndpointParams::Featured => {
            SearchResultPlaylist::Featured(SearchResultFeaturedPlaylist {
                title,
                author,
                songs: parse_flex_column_item(&mut mrlir, 1, 2)?,
                playlist_id,
                thumbnails,
            })
        }
        PlaylistEndpointParams::Community => {
            SearchResultPlaylist::Community(SearchResultCommunityPlaylist {
                title,
                author,
                views: parse_flex_column_item(&mut mrlir, 1, 2)?,
                playlist_id,
                thumbnails,
            })
        }
    };
    Ok(playlist)
}

// TODO: Rename FilteredSearchSectionContents
struct SectionContentsCrawler(JsonCrawlerOwned);
struct BasicSearchSectionListContents(JsonCrawlerOwned);
// In this case, we've searched and had no results found.
// We are being quite explicit here to avoid a false positive.
// See tests for an example.
// TODO: Test this function itself.
fn section_contents_is_empty(section_contents: &mut SectionContentsCrawler) -> Result<bool> {
    Ok(section_contents
        .0
        .try_iter_mut()?
        .any(|item| item.path_exists("/itemSectionRenderer/contents/0/didYouMeanRenderer")))
}
// TODO: Consolidate these two functions into single function.
// TODO: This could be implemented with a non-mutable array also.
fn section_list_contents_is_empty(
    section_contents: &mut BasicSearchSectionListContents,
) -> Result<bool> {
    let is_empty = section_contents
        .0
        .try_iter_mut()?
        .filter(|item| item.path_exists(MUSIC_CARD_SHELF) || item.path_exists(MUSIC_SHELF))
        .count()
        == 0;
    Ok(is_empty)
}
impl<'a, S: UnfilteredSearchType> TryFrom<ProcessedResult<'a, SearchQuery<'a, S>>>
    for BasicSearchSectionListContents
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, S>>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = value.into();
        let section_list_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST
        ))?;
        Ok(BasicSearchSectionListContents(section_list_contents))
    }
}
impl<'a, F: FilteredSearchType> TryFrom<ProcessedResult<'a, SearchQuery<'a, FilteredSearch<F>>>>
    for SectionContentsCrawler
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = value.into();
        let section_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST,
        ))?;
        Ok(SectionContentsCrawler(section_contents))
    }
}
// XXX: Should this also contain query type?
struct FilteredSearchMSRContents(JsonCrawlerOwned);
impl TryFrom<SectionContentsCrawler> for FilteredSearchMSRContents {
    type Error = Error;
    fn try_from(value: SectionContentsCrawler) -> std::prelude::v1::Result<Self, Self::Error> {
        let music_shelf_contents = value
            .0
            .try_into_iter()?
            .find_path(concatcp!(MUSIC_SHELF, "/contents"))?;
        Ok(FilteredSearchMSRContents(music_shelf_contents))
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultAlbum> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_album_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultProfile> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_profile_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultArtist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_artist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultSong> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_song_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultVideo> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .filter_map(|a| parse_video_search_result_from_music_shelf_contents(a).transpose())
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultEpisode> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_episode_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultPodcast> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_podcast_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultCommunityPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_community_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultFeaturedPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_featured_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl<'a, S: UnfilteredSearchType> ParseFrom<SearchQuery<'a, S>> for SearchResults {
    fn parse_from(p: ProcessedResult<SearchQuery<'a, S>>) -> crate::Result<Self> {
        let mut section_list_contents = BasicSearchSectionListContents::try_from(p)?;
        if section_list_contents_is_empty(&mut section_list_contents)? {
            return Ok(Self::default());
        }
        parse_basic_search_result_from_section_list_contents(section_list_contents)
    }
}

impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<ArtistsFilter>>> for Vec<SearchResultArtist> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<ProfilesFilter>>> for Vec<SearchResultProfile> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<AlbumsFilter>>> for Vec<SearchResultAlbum> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<SongsFilter>>> for Vec<SearchResultSong> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<SongsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<VideosFilter>>> for Vec<SearchResultVideo> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<VideosFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<EpisodesFilter>>> for Vec<SearchResultEpisode> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<PodcastsFilter>>> for Vec<SearchResultPodcast> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>
    for Vec<SearchResultPlaylist>
{
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>
    for Vec<SearchResultFeaturedPlaylist>
{
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>> for Vec<SearchResultPlaylist> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>,
    ) -> crate::Result<Self> {
        let mut section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}

impl<'a> ParseFrom<GetSearchSuggestionsQuery<'a>> for Vec<SearchSuggestion> {
    fn parse_from(p: ProcessedResult<GetSearchSuggestionsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut suggestions = json_crawler
            .navigate_pointer("/contents/0/searchSuggestionsSectionRenderer/contents")?;
        let mut results = Vec::new();
        for mut s in suggestions.try_iter_mut()? {
            let mut runs = Vec::new();
            if let Ok(mut search_suggestion) =
                s.borrow_pointer("/searchSuggestionRenderer/suggestion/runs")
            {
                for mut r in search_suggestion.try_iter_mut()? {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Bold)?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Normal)?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::Prediction, runs))
            } else {
                for mut r in s
                    .borrow_pointer("/historySuggestionRenderer/suggestion/runs")?
                    .try_iter_mut()?
                {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Bold)?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Normal)?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::History, runs))
            }
        }
        Ok(results)
    }
}
