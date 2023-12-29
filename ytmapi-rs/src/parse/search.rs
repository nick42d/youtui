use super::{
    parse_item_text, Parse, ProcessedResult, SearchResult, SearchResultAlbum, SearchResultArtist,
    SearchResultCommunityPlaylist, SearchResultEpisode, SearchResultFeaturedPlaylist,
    SearchResultPlaylist, SearchResultPodcast, SearchResultProfile, SearchResultSong,
    SearchResultVideo,
};
use crate::common::{AlbumType, Explicit, SearchSuggestion, SuggestionType, TextRun, YoutubeID};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{
    BADGE_LABEL, LIVE_BADGE_LABEL, NAVIGATION_BROWSE_ID, NAVIGATION_VIDEO_ID,
    PLAYLIST_ITEM_VIDEO_ID, PLAY_BUTTON, SECTION_LIST, THUMBNAILS,
};
use crate::parse::EpisodeDate;
use crate::{query::*, ChannelID, Thumbnail, VideoID};
use crate::{Error, Result};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;
// watchPlaylistEndpoint params within overlay.
const FEATURED_PLAYLIST_ENDPOINT_PARAMS: &str = "wAEB";
const COMMUNITY_PLAYLIST_ENDPOINT_PARAMS: &str = "wAEB8gECKAE%3D";

// May be redundant due to encoding this in type system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchResultType {
    Artist,
    Album(AlbumType), // Does albumtype matter here?
    Playlist,
    Song,
    Video,
    Station,
}
impl TryFrom<&String> for SearchResultType {
    type Error = crate::Error;
    fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            // Dirty hack to get artist outputting
            "\"Artist\"" => Ok(Self::Artist),
            "artist" => Ok(Self::Artist),
            "album" => Ok(Self::Album(AlbumType::Album)),
            "ep" => Ok(Self::Album(AlbumType::EP)),
            "single" => Ok(Self::Album(AlbumType::Single)),
            "playlist" => Ok(Self::Playlist),
            "song" => Ok(Self::Song),
            "video" => Ok(Self::Video),
            "station" => Ok(Self::Station),
            // TODO: Better error
            _ => Err(Error::other(format!(
                "Unable to parse SearchResultType {value}"
            ))),
        }
    }
}

impl<'a> Parse for ProcessedResult<SearchQuery<'a, BasicSearch>> {
    type Output = Vec<super::SearchResult>;
    fn parse(self) -> Result<Self::Output> {
        let ProcessedResult {
            mut json_crawler, ..
        } = self;
        todo!();
    }
}
// TODO: Type safety
// TODO: Tests
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let artist = parse_item_text(&mut mrlir, 0, 0)?;
    let subscribers = parse_item_text(&mut mrlir, 1, 2)?;
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let username = parse_item_text(&mut mrlir, 1, 2)?;
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
    let artist = parse_item_text(&mut mrlir, 0, 0)?;
    let album_type = parse_item_text(&mut mrlir, 1, 0).and_then(|a| AlbumType::try_from_str(a))?;
    let title = parse_item_text(&mut mrlir, 1, 2)?;
    let year = parse_item_text(&mut mrlir, 1, 4)?;
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
        browse_id,
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let artist = parse_item_text(&mut mrlir, 1, 0)?;
    let album = parse_item_text(&mut mrlir, 1, 2)?;
    let duration = parse_item_text(&mut mrlir, 1, 4)?;
    let plays = parse_item_text(&mut mrlir, 2, 0)?;
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let channel_name = parse_item_text(&mut mrlir, 1, 0)?;
    let views = parse_item_text(&mut mrlir, 1, 2)?;
    let length = parse_item_text(&mut mrlir, 1, 4)?;
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultVideo {
        title,
        channel_name,
        views,
        length,
        thumbnails,
        video_id,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_podcast_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPodcast> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let publisher = parse_item_text(&mut mrlir, 1, 0)?;
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let date = if mrlir.path_exists(LIVE_BADGE_LABEL) {
        EpisodeDate::Live
    } else {
        EpisodeDate::Recorded {
            date: parse_item_text(&mut mrlir, 1, 0)?,
        }
    };
    let channel_name = match date {
        EpisodeDate::Live => parse_item_text(&mut mrlir, 1, 0)?,
        EpisodeDate::Recorded { .. } => parse_item_text(&mut mrlir, 1, 2)?,
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let author = parse_item_text(&mut mrlir, 1, 0)?;
    let songs = parse_item_text(&mut mrlir, 1, 2)?;
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let author = parse_item_text(&mut mrlir, 1, 0)?;
    let views = parse_item_text(&mut mrlir, 1, 2)?;
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
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let author = parse_item_text(&mut mrlir, 1, 0)?;
    let playlist_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    // The playlist search contains a mix of Community and Featured playlists.
    let playlist_params: String = mrlir.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchPlaylistEndpoint/params"
    ))?;
    let playlist_params_str = playlist_params.as_str();
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    let playlist = match playlist_params_str {
        FEATURED_PLAYLIST_ENDPOINT_PARAMS => {
            SearchResultPlaylist::Featured(SearchResultFeaturedPlaylist {
                title,
                author,
                songs: parse_item_text(&mut mrlir, 1, 2)?,
                playlist_id,
                thumbnails,
            })
        }
        COMMUNITY_PLAYLIST_ENDPOINT_PARAMS => {
            SearchResultPlaylist::Community(SearchResultCommunityPlaylist {
                title,
                author,
                views: parse_item_text(&mut mrlir, 1, 2)?,
                playlist_id,
                thumbnails,
            })
        }
        other => {
            return Err(Error::other(format!(
                "Unexpected playlist params: {}",
                other
            )));
        }
    };
    Ok(playlist)
}

// TODO: Rename FilteredSearchSectionContents
struct SectionContentsCrawler(JsonCrawler);
// In this case, we've searched and had no results found.
// We are being quite explicit here to avoid a false positive.
// See tests for an example.
// TODO: Test this function.
fn section_contents_is_empty(section_contents: &SectionContentsCrawler) -> bool {
    section_contents
        .0
        .path_exists("/itemSectionRenderer/contents/0/didYouMeanRenderer")
}
impl<'a, F: FilteredSearchType> TryFrom<ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>>
    for SectionContentsCrawler
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>) -> Result<Self> {
        let ProcessedResult {
            mut json_crawler, ..
        } = value;
        let section_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content",
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
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>> {
    type Output = Vec<SearchResultArtist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<ProfilesFilter>>> {
    type Output = Vec<SearchResultProfile>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>> {
    type Output = Vec<SearchResultAlbum>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<SongsFilter>>> {
    type Output = Vec<SearchResultSong>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<VideosFilter>>> {
    type Output = Vec<SearchResultVideo>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<EpisodesFilter>>> {
    type Output = Vec<SearchResultEpisode>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<PodcastsFilter>>> {
    type Output = Vec<SearchResultPodcast>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>> {
    type Output = Vec<SearchResultPlaylist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>> {
    type Output = Vec<SearchResultFeaturedPlaylist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>> {
    type Output = Vec<SearchResultPlaylist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}

impl<'a> Parse for ProcessedResult<GetSearchSuggestionsQuery<'a>> {
    type Output = Vec<SearchSuggestion>;
    fn parse(self) -> Result<Self::Output> {
        let ProcessedResult { json_crawler, .. } = self;
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
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Bold(s))?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Normal(s))?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::Prediction, runs))
            } else {
                for mut r in s
                    .borrow_pointer("/historySuggestionRenderer/suggestion/runs")?
                    .into_array_iter_mut()?
                {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Bold(s))?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Normal(s))?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::History, runs))
            }
        }
        Ok(results)
    }
}

// Continuation functions for future use
fn get_continuations(res: &SearchResult) {}

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
