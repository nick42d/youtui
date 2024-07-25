use std::sync::Arc;

use super::{
    parse_flex_column_item, ParseFrom, ProcessedResult, SearchResultAlbum, SearchResultArtist,
    SearchResultCommunityPlaylist, SearchResultEpisode, SearchResultFeaturedPlaylist,
    SearchResultPlaylist, SearchResultPodcast, SearchResultProfile, SearchResultSong,
    SearchResultType, SearchResultVideo, SearchResults, TopResult, TopResultType,
};
use crate::common::{Explicit, SearchSuggestion, SuggestionType, TextRun};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator};
use crate::nav_consts::{
    BADGE_LABEL, LIVE_BADGE_LABEL, MUSIC_CARD_SHELF, MUSIC_SHELF, NAVIGATION_BROWSE_ID,
    PLAYLIST_ITEM_VIDEO_ID, PLAY_BUTTON, SECTION_LIST, SUBTITLE, SUBTITLE2, TAB_CONTENT,
    THUMBNAILS, TITLE_TEXT,
};
use crate::parse::EpisodeDate;
use crate::youtube_enums::PlaylistEndpointParams;
use crate::{query::*, Thumbnail};
use crate::{Error, Result};
use const_format::concatcp;
use filteredsearch::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    FilteredSearch, FilteredSearchType, PlaylistsFilter, PodcastsFilter, ProfilesFilter,
    SongsFilter, VideosFilter,
};
use serde::de::IntoDeserializer;
use serde::Deserialize;

#[cfg(test)]
mod tests;

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
    let results_iter = section_list_contents.0.into_array_into_iter()?;
    let results_iter_peekable = results_iter.clone().peekable();
    // XXX: Naive solution.
    if results_iter_peekable
        .clone()
        .peek()
        .ok_or_else(|| {
            let (source, path) = results_iter.clone().get_context();
            Error::array_size(path, source, 1)
        })?
        .path_exists(MUSIC_CARD_SHELF)
    {
        top_results = parse_top_results_from_music_card_shelf_contents(
            results_iter
                .clone()
                .next()
                .ok_or_else(|| {
                    let (source, path) = results_iter.clone().get_context();
                    Error::array_size(path, source, 1)
                })?
                .borrow_pointer(MUSIC_CARD_SHELF)?,
        )?;
    }

    for category in results_iter.map(|r| r.navigate_pointer(MUSIC_SHELF)) {
        let mut category = category?;
        match category.take_value_pointer::<SearchResultType>(TITLE_TEXT)? {
            SearchResultType::TopResult => {
                top_results = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .filter_map(|r| parse_top_result_from_music_shelf_contents(r).transpose())
                    .collect::<Result<Vec<TopResult>>>()?;
            }
            // TODO: Use a navigation constant
            SearchResultType::Artists => {
                artists = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_artist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultArtist>>>()?;
            }
            SearchResultType::Albums => {
                albums = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_album_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultAlbum>>>()?
            }
            SearchResultType::FeaturedPlaylists => {
                featured_playlists = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_featured_playlist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultFeaturedPlaylist>>>()?
            }
            SearchResultType::CommunityPlaylists => {
                community_playlists = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_community_playlist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultCommunityPlaylist>>>()?
            }
            SearchResultType::Songs => {
                songs = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_song_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultSong>>>()?
            }
            SearchResultType::Videos => {
                videos = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_video_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultVideo>>>()?
            }
            SearchResultType::Podcasts => {
                podcasts = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_podcast_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultPodcast>>>()?
            }
            SearchResultType::Episodes => {
                episodes = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_episode_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultEpisode>>>()?
            }
            SearchResultType::Profiles => {
                profiles = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
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
    let result_type = music_shelf_contents
        .take_value_pointer::<TopResultType>(SUBTITLE)
        .ok();
    // Possibly artists only.
    let subscribers = music_shelf_contents.take_value_pointer(SUBTITLE2)?;
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
    };
    // End - first result parsing.
    results.push(first_result);
    // Other results may not exist.
    if let Ok(mut contents) = music_shelf_contents.navigate_pointer("/contents") {
        contents
            .as_array_iter_mut()?
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
    let artist = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let album_type = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let title = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let year = parse_flex_column_item(&mut mrlir, 1, 4)?;
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
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let artist = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let album = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let duration = parse_flex_column_item(&mut mrlir, 1, 4)?;
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
) -> Result<SearchResultVideo> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
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
            Ok(SearchResultVideo::Video {
                title,
                channel_name,
                views,
                length,
                thumbnails,
                video_id,
            })
        }
        "Episode" => {
            //TODO: Handle live episode
            let date = EpisodeDate::Recorded {
                date: parse_flex_column_item(&mut mrlir, 1, 2)?,
            };
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(SearchResultVideo::VideoEpisode {
                title,
                channel_name,
                date,
                thumbnails,
                video_id,
            })
        }
        _ => {
            // Assume that if a watch endpoint exists, it's a video.
            if mrlir.path_exists("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint") {

            let views = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let length = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(SearchResultVideo::Video {
                title,
                channel_name: first_field,
                views,
                length,
                thumbnails,
                video_id,
            })
            } else {
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(SearchResultVideo::VideoEpisode {
                title,
                channel_name,
            //TODO: Handle live episode
                date: EpisodeDate::Recorded { date: first_field },
                thumbnails,
                video_id,
            })
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
        video_id,
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
struct SectionContentsCrawler(JsonCrawler);
struct BasicSearchSectionListContents(JsonCrawler);
// In this case, we've searched and had no results found.
// We are being quite explicit here to avoid a false positive.
// See tests for an example.
// TODO: Test this function.
fn section_contents_is_empty(section_contents: &SectionContentsCrawler) -> bool {
    section_contents
        .0
        .path_exists("/itemSectionRenderer/contents/0/didYouMeanRenderer")
}
// TODO: Consolidate these two functions into single function.
fn section_list_contents_is_empty(section_contents: &BasicSearchSectionListContents) -> bool {
    section_contents
        .0
        .path_exists("/0/itemSectionRenderer/contents/0/didYouMeanRenderer")
        || section_contents
            .0
            .path_exists("/0/itemSectionRenderer/contents/0/messageRenderer")
}
impl<'a, S: UnfilteredSearchType> TryFrom<ProcessedResult<SearchQuery<'a, S>>>
    for BasicSearchSectionListContents
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, S>>) -> Result<Self> {
        let json_crawler: JsonCrawler = value.into();
        let section_list_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST
        ))?;
        Ok(BasicSearchSectionListContents(section_list_contents))
    }
}
impl<'a, F: FilteredSearchType> TryFrom<ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>>
    for SectionContentsCrawler
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>) -> Result<Self> {
        let json_crawler: JsonCrawler = value.into();
        let section_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST,
            "/0"
        ))?;
        Ok(SectionContentsCrawler(section_contents))
    }
}
// XXX: Should this also contain query type?
struct FilteredSearchMSRContents(JsonCrawler);
impl TryFrom<SectionContentsCrawler> for FilteredSearchMSRContents {
    type Error = Error;
    fn try_from(value: SectionContentsCrawler) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(FilteredSearchMSRContents(
            value.0.navigate_pointer("/musicShelfRenderer/contents")?,
        ))
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
            .map(|a| parse_video_search_result_from_music_shelf_contents(a))
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
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
            .as_array_iter_mut()?
            .map(|a| parse_featured_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl<'a, S: UnfilteredSearchType> ParseFrom<SearchQuery<'a, S>> for SearchResults {
    fn parse_from(p: ProcessedResult<SearchQuery<'a, S>>) -> crate::Result<Self> {
        let section_list_contents = BasicSearchSectionListContents::try_from(p)?;
        if section_list_contents_is_empty(&section_list_contents) {
            return Ok(Self::default());
        }
        parse_basic_search_result_from_section_list_contents(section_list_contents)
    }
}

impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<ArtistsFilter>>> for Vec<SearchResultArtist> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<ProfilesFilter>>> for Vec<SearchResultProfile> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<AlbumsFilter>>> for Vec<SearchResultAlbum> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<SongsFilter>>> for Vec<SearchResultSong> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<SongsFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<VideosFilter>>> for Vec<SearchResultVideo> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<VideosFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<EpisodesFilter>>> for Vec<SearchResultEpisode> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<PodcastsFilter>>> for Vec<SearchResultPodcast> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
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
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
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
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> ParseFrom<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>> for Vec<SearchResultPlaylist> {
    fn parse_from(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>,
    ) -> crate::Result<Self> {
        let section_contents = SectionContentsCrawler::try_from(p)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}

impl<'a> ParseFrom<GetSearchSuggestionsQuery<'a>> for Vec<SearchSuggestion> {
    fn parse_from(p: ProcessedResult<GetSearchSuggestionsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawler = p.into();
        let mut suggestions = json_crawler
            .navigate_pointer("/contents/0/searchSuggestionsSectionRenderer/contents")?;
        let mut results = Vec::new();
        for mut s in suggestions.as_array_iter_mut()? {
            let mut runs = Vec::new();
            if let Ok(search_suggestion) =
                s.borrow_pointer("/searchSuggestionRenderer/suggestion/runs")
            {
                for mut r in search_suggestion.into_array_iter_mut()? {
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
                    .into_array_iter_mut()?
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

fn get_reloadable_continuation_params(json: &mut JsonCrawlerBorrowed) -> Result<String> {
    let ctoken = json.take_value_pointer("/continuations/0/reloadContinuationData/continuation")?;
    Ok(get_continuation_string(ctoken))
}

fn get_continuation_params(
    json: &mut JsonCrawlerBorrowed,
    ctoken_path: Option<&str>,
) -> Result<String> {
    let ctoken = if let Some(ctoken_path) = ctoken_path {
        let key = format!("/continuations/0/next{ctoken_path}/ContinuationData/continuation");
        json.take_value_pointer(key)?
    } else {
        json.take_value_pointer("/continuations/0/next/ContinuationData/continuation")?
    };
    Ok(get_continuation_string(ctoken))
}

fn get_continuation_string(ctoken: String) -> String {
    format!("&ctoken={0}&continuation={0}", ctoken)
}
