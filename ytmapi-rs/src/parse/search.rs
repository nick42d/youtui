use super::{
    parse_item_text, Parse, ProcessedResult, SearchResultAlbum, SearchResultArtist,
    SearchResultCommunityPlaylist, SearchResultEpisode, SearchResultFeaturedPlaylist,
    SearchResultPlaylist, SearchResultPodcast, SearchResultProfile, SearchResultSong,
    SearchResultType, SearchResultVideo, SearchResults, TopResult, TopResultType,
};
use crate::common::{AlbumType, Explicit, SearchSuggestion, SuggestionType, TextRun};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{
    BADGE_LABEL, LIVE_BADGE_LABEL, MUSIC_SHELF, NAVIGATION_BROWSE_ID, PLAYLIST_ITEM_VIDEO_ID,
    PLAY_BUTTON, SECTION_LIST, TAB_CONTENT, THUMBNAILS, TITLE_TEXT,
};
use crate::parse::EpisodeDate;
use crate::{query::*, Thumbnail};
use crate::{Error, Result};
use const_format::concatcp;

#[cfg(test)]
mod tests;
// watchPlaylistEndpoint params within overlay.
const FEATURED_PLAYLIST_ENDPOINT_PARAMS: &str = "wAEB";
const COMMUNITY_PLAYLIST_ENDPOINT_PARAMS: &str = "wAEB8gECKAE%3D";

// TODO: Type safety
// TODO: Tests
fn parse_basic_search_result_from_xx(
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
    for category in section_list_contents
        .0
        .as_array_iter_mut()?
        .map(|r| r.navigate_pointer(MUSIC_SHELF))
    {
        let mut category = category?;
        match SearchResultType::try_from(
            // TODO: Better navigation
            category.take_value_pointer::<String, &str>(TITLE_TEXT)?,
        )? {
            SearchResultType::TopResults => {
                top_results = category
                    .navigate_pointer("/contents")?
                    .as_array_iter_mut()?
                    .map(|r| parse_top_result_from_music_shelf_contents(r))
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
// TODO: Type safety
// TODO: Tests
fn parse_top_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<TopResult> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let result_name = parse_item_text(&mut mrlir, 0, 0)?;
    let result_type = TopResultType::try_from(parse_item_text(&mut mrlir, 1, 0)?)?;
    // Imperative solution, may be able to make more functional.
    let mut subscribers = None;
    let mut artist_info = None;
    let mut publisher = None;
    let mut artist = None;
    let mut album = None;
    let mut duration = None;
    let mut year = None;
    let mut plays = None;
    match result_type {
        // XXX: Perhaps also populate Artist field.
        TopResultType::Artist => subscribers = Some(parse_item_text(&mut mrlir, 1, 2)?),
        TopResultType::Album(_) => {
            // XXX: Perhaps also populate Album field.
            artist = Some(parse_item_text(&mut mrlir, 1, 2)?);
            year = Some(parse_item_text(&mut mrlir, 1, 4)?);
        }
        TopResultType::Playlist => todo!(),
        TopResultType::Song => {
            artist = Some(parse_item_text(&mut mrlir, 1, 2)?);
            album = Some(parse_item_text(&mut mrlir, 1, 4)?);
            duration = Some(parse_item_text(&mut mrlir, 1, 6)?);
            plays = Some(parse_item_text(&mut mrlir, 1, 8)?);
        }
        TopResultType::Video => todo!(),
        TopResultType::Station => todo!(),
        TopResultType::Podcast => publisher = Some(parse_item_text(&mut mrlir, 1, 2)?),
    }
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(TopResult {
        result_type,
        subscribers,
        thumbnails,
        artist_info,
        result_name,
        publisher,
        artist,
        album,
        duration,
        year,
        plays,
    })
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
impl<'a> TryFrom<ProcessedResult<SearchQuery<'a, BasicSearch>>> for BasicSearchSectionListContents {
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, BasicSearch>>) -> Result<Self> {
        let ProcessedResult {
            mut json_crawler, ..
        } = value;
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
        let ProcessedResult {
            mut json_crawler, ..
        } = value;
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
impl<'a> Parse for ProcessedResult<SearchQuery<'a, BasicSearch>> {
    type Output = SearchResults;
    fn parse(self) -> Result<Self::Output> {
        let section_list_contents = BasicSearchSectionListContents::try_from(self)?;
        parse_basic_search_result_from_xx(section_list_contents)
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
