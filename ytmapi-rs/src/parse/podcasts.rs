use super::{
    ParseFrom, RUN_TEXT, SECONDARY_SECTION_LIST_ITEM, STRAPLINE_RUNS, TAB_CONTENT, THUMBNAILS,
    THUMBNAIL_RENDERER, TITLE_TEXT, VISUAL_HEADER,
};
use crate::common::{
    EpisodeID, LibraryStatus, PlaylistID, PodcastChannelID, PodcastChannelParams, PodcastID,
    Thumbnail,
};
use crate::nav_consts::{
    CAROUSEL, CAROUSEL_TITLE, DESCRIPTION, DESCRIPTION_SHELF, GRID_ITEMS, MMRLIR, MTRIR,
    MUSIC_SHELF, NAVIGATION_BROWSE, NAVIGATION_BROWSE_ID, PLAYBACK_DURATION_TEXT,
    PLAYBACK_PROGRESS_TEXT, RESPONSIVE_HEADER, SECTION_LIST, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB,
    SUBTITLE, SUBTITLE3, SUBTITLE_RUNS, TITLE, TWO_COLUMN,
};
use crate::query::{
    GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery, GetPodcastQuery,
};
use crate::Result;
use const_format::concatcp;
use itertools::Itertools;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcastChannel {
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub episode_params: Option<PodcastChannelParams<'static>>,
    pub episodes: Vec<Episode>,
    pub podcasts: Vec<GetPodcastChannelPodcast>,
    pub playlists: Vec<GetPodcastChannelPlaylist>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Episode {
    pub title: String,
    pub description: String,
    pub total_duration: String,
    pub remaining_duration: String,
    pub date: String,
    pub episode_id: EpisodeID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcastChannelPodcast {
    pub title: String,
    pub channels: Vec<ParsedPodcastChannel>,
    pub podcast_id: PodcastID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcastChannelPlaylist {
    pub title: String,
    pub channel: ParsedPodcastChannel,
    pub playlist_id: PlaylistID<'static>,
    pub views: String,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expected to change.
pub struct ParsedPodcastChannel {
    pub name: String,
    pub id: Option<PodcastChannelID<'static>>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expected to change.
pub enum IsSaved {
    Saved,
    NotSaved,
}
#[derive(Eq, PartialEq, Debug, Clone, Deserialize, Serialize, Hash)]
// Intentionally not marked non_exhaustive - not expected to change.
pub enum PodcastChannelTopResult {
    #[serde(rename = "Latest episodes")]
    Episodes,
    Podcasts,
    Playlists,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcast {
    pub channels: Vec<ParsedPodcastChannel>,
    pub title: String,
    pub description: String,
    // TODO: How to add a podcast to library?
    pub library_status: LibraryStatus,
    pub episodes: Vec<Episode>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetEpisode {
    pub podcast_name: String,
    pub podcast_id: PodcastID<'static>,
    pub title: String,
    pub date: String,
    pub total_duration: String,
    pub remaining_duration: String,
    pub saved: IsSaved,
    pub description: String,
}

// NOTE: This is technically the same page as the GetArtist page. It's possible
// this could be generalised.
impl ParseFrom<GetChannelQuery<'_>> for GetPodcastChannel {
    fn parse_from(p: crate::ProcessedResult<GetChannelQuery>) -> Result<Self> {
        fn parse_podcast(crawler: impl JsonCrawler) -> Result<GetPodcastChannelPodcast> {
            let mut podcast = crawler.navigate_pointer(MTRIR)?;
            let title = podcast.take_value_pointer(TITLE_TEXT)?;
            let podcast_id = podcast.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let thumbnails = podcast.take_value_pointer(THUMBNAIL_RENDERER)?;
            let channels = podcast
                .navigate_pointer(SUBTITLE_RUNS)?
                .try_into_iter()?
                .map(parse_podcast_channel)
                .collect::<Result<Vec<_>>>()?;
            Ok(GetPodcastChannelPodcast {
                title,
                channels,
                podcast_id,
                thumbnails,
            })
        }
        fn parse_playlist(crawler: impl JsonCrawler) -> Result<GetPodcastChannelPlaylist> {
            let mut podcast = crawler.navigate_pointer(MTRIR)?;
            let title = podcast.take_value_pointer(TITLE_TEXT)?;
            let playlist_id = podcast.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let thumbnails = podcast.take_value_pointer(THUMBNAIL_RENDERER)?;
            let views = podcast.take_value_pointer(SUBTITLE3)?;
            let channel =
                parse_podcast_channel(podcast.navigate_pointer(SUBTITLE_RUNS)?.navigate_index(2)?)?;
            Ok(GetPodcastChannelPlaylist {
                title,
                channel,
                thumbnails,
                playlist_id,
                views,
            })
        }
        let mut json_crawler = JsonCrawlerOwned::from(p);
        let mut header = json_crawler.borrow_pointer(VISUAL_HEADER)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let thumbnails = header.take_value_pointer(THUMBNAILS)?;
        let mut podcasts = Vec::new();
        let mut episodes = Vec::new();
        let mut playlists = Vec::new();
        let mut episode_params = None;
        // I spent a good few hours trying to make this declarative. It this stage this
        // seems to be more readable and more efficient. The best declarative approach I
        // could find used a collect into a HashMap and the process_results()
        // function...
        for carousel in json_crawler
            .borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .try_into_iter()?
            .map(|item| item.navigate_pointer(CAROUSEL))
        {
            let mut carousel = carousel?;
            match carousel
                .take_value_pointer::<PodcastChannelTopResult>(concatcp!(CAROUSEL_TITLE, "/text"))?
            {
                PodcastChannelTopResult::Episodes => {
                    episode_params = carousel.take_value_pointer(concatcp!(
                        CAROUSEL_TITLE,
                        NAVIGATION_BROWSE,
                        "/params"
                    ))?;
                    episodes = carousel
                        .navigate_pointer("/contents")?
                        .try_into_iter()?
                        .map(parse_episode)
                        .collect::<Result<_>>()?;
                }
                PodcastChannelTopResult::Podcasts => {
                    podcasts = carousel
                        .navigate_pointer("/contents")?
                        .try_into_iter()?
                        .map(parse_podcast)
                        .collect::<Result<_>>()?;
                }
                PodcastChannelTopResult::Playlists => {
                    playlists = carousel
                        .navigate_pointer("/contents")?
                        .try_into_iter()?
                        .map(parse_playlist)
                        .collect::<Result<_>>()?;
                }
            }
        }
        Ok(GetPodcastChannel {
            title,
            thumbnails,
            episode_params,
            episodes,
            podcasts,
            playlists,
        })
    }
}
impl ParseFrom<GetChannelEpisodesQuery<'_>> for Vec<Episode> {
    fn parse_from(p: crate::ProcessedResult<GetChannelEpisodesQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID_ITEMS))?
            .try_into_iter()?
            .map(parse_episode)
            .collect()
    }
}
impl ParseFrom<GetPodcastQuery<'_>> for GetPodcast {
    fn parse_from(p: crate::ProcessedResult<GetPodcastQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        let mut two_column = json_crawler.navigate_pointer(TWO_COLUMN)?;
        let episodes = two_column
            .borrow_pointer(concatcp!(
                "/secondaryContents",
                SECTION_LIST_ITEM,
                MUSIC_SHELF,
                "/contents"
            ))?
            .try_into_iter()?
            .map(parse_episode)
            .collect::<Result<_>>()?;
        let mut responsive_header = two_column.navigate_pointer(concatcp!(
            TAB_CONTENT,
            SECTION_LIST_ITEM,
            RESPONSIVE_HEADER,
        ))?;
        let library_status = match responsive_header
            .take_value_pointer::<bool>("/buttons/1/toggleButtonRenderer/isToggled")?
        {
            true => LibraryStatus::InLibrary,
            false => LibraryStatus::NotInLibrary,
        };
        let channels = responsive_header
            .borrow_pointer(STRAPLINE_RUNS)?
            .try_into_iter()?
            .map(parse_podcast_channel)
            .collect::<Result<_>>()?;
        let mut description_shelf =
            responsive_header.navigate_pointer(concatcp!("/description", DESCRIPTION_SHELF))?;
        let description = description_shelf.take_value_pointer(DESCRIPTION)?;
        let title = description_shelf.take_value_pointer(concatcp!("/header", RUN_TEXT))?;
        Ok(GetPodcast {
            channels,
            title,
            description,
            library_status,
            episodes,
        })
    }
}
impl ParseFrom<GetEpisodeQuery<'_>> for GetEpisode {
    fn parse_from(p: crate::ProcessedResult<GetEpisodeQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        let mut two_column = json_crawler.navigate_pointer(TWO_COLUMN)?;
        let mut responsive_header = two_column.borrow_pointer(concatcp!(
            TAB_CONTENT,
            SECTION_LIST_ITEM,
            RESPONSIVE_HEADER,
        ))?;
        let title = responsive_header.take_value_pointer(TITLE_TEXT)?;
        let date = responsive_header.take_value_pointer(SUBTITLE)?;
        let total_duration = responsive_header.take_value_pointer(
            "/progress/musicPlaybackProgressRenderer/playbackProgressText/runs/1/text",
        )?;
        let remaining_duration = responsive_header.take_value_pointer(
            "/progress/musicPlaybackProgressRenderer/durationText/runs/1/text",
        )?;
        let saved = match responsive_header
            .take_value_pointer::<bool>("/buttons/0/toggleButtonRenderer/isToggled")?
        {
            true => IsSaved::Saved,
            false => IsSaved::NotSaved,
        };
        let mut strapline = responsive_header.navigate_pointer(concatcp!(STRAPLINE_RUNS, "/0"))?;
        let podcast_name = strapline.take_value_pointer("/text")?;
        let podcast_id = strapline.take_value_pointer(NAVIGATION_BROWSE_ID)?;
        let description = two_column
            .navigate_pointer(concatcp!(
                SECONDARY_SECTION_LIST_ITEM,
                DESCRIPTION_SHELF,
                "/description/runs"
            ))?
            .try_into_iter()?
            .map(|mut item| item.take_value_pointer::<String>("/text"))
            .process_results(|iter| iter.collect())?;
        Ok(GetEpisode {
            title,
            date,
            total_duration,
            remaining_duration,
            saved,
            description,
            podcast_name,
            podcast_id,
        })
    }
}
impl ParseFrom<GetNewEpisodesQuery> for Vec<Episode> {
    fn parse_from(p: crate::ProcessedResult<GetNewEpisodesQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        json_crawler
            .navigate_pointer(concatcp!(
                TWO_COLUMN,
                "/secondaryContents",
                SECTION_LIST_ITEM,
                MUSIC_SHELF,
                "/contents"
            ))?
            .try_into_iter()?
            .map(parse_episode)
            .collect()
    }
}

fn parse_podcast_channel(mut data: impl JsonCrawler) -> Result<ParsedPodcastChannel> {
    Ok(ParsedPodcastChannel {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
    })
}

fn parse_episode(crawler: impl JsonCrawler) -> Result<Episode> {
    let mut episode = crawler.navigate_pointer(MMRLIR)?;
    let description = episode.take_value_pointer(DESCRIPTION)?;
    let total_duration = episode.take_value_pointer(PLAYBACK_DURATION_TEXT)?;
    let remaining_duration = episode.take_value_pointer(PLAYBACK_PROGRESS_TEXT)?;
    let date = episode.take_value_pointer(SUBTITLE)?;
    let thumbnails = episode.take_value_pointer(THUMBNAILS)?;
    let mut title_run = episode.navigate_pointer(TITLE)?;
    let title = title_run.take_value_pointer("/text")?;
    let episode_id = title_run.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    Ok(Episode {
        title,
        description,
        total_duration,
        remaining_duration,
        date,
        episode_id,
        thumbnails,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{EpisodeID, PodcastChannelID, PodcastChannelParams, PodcastID, YoutubeID};
    use crate::query::{
        GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery,
        GetPodcastQuery,
    };

    #[tokio::test]
    async fn test_get_channel() {
        parse_test!(
            "./test_json/get_channel_20240830.json",
            "./test_json/get_channel_20240830_output.txt",
            GetChannelQuery::new(PodcastChannelID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_channel_episodes() {
        parse_test!(
            "./test_json/get_channel_episodes_20240830.json",
            "./test_json/get_channel_episodes_20240830_output.txt",
            GetChannelEpisodesQuery::new(
                PodcastChannelID::from_raw(""),
                PodcastChannelParams::from_raw("")
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_podcast() {
        parse_test!(
            "./test_json/get_podcast_20240830.json",
            "./test_json/get_podcast_20240830_output.txt",
            GetPodcastQuery::new(PodcastID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_episode() {
        parse_test!(
            "./test_json/get_episode_20240830.json",
            "./test_json/get_episode_20240830_output.txt",
            GetEpisodeQuery::new(EpisodeID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_new_episodes() {
        parse_test!(
            "./test_json/get_new_episodes_20240830.json",
            "./test_json/get_new_episodes_20240830_output.txt",
            GetNewEpisodesQuery,
            BrowserToken
        );
    }
}
