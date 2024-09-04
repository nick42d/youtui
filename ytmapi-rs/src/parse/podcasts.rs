use super::{
    ParseFrom, RUN_TEXT, SECONDARY_SECTION_LIST_ITEM, STRAPLINE_RUNS, TAB_CONTENT, THUMBNAILS,
    THUMBNAIL_RENDERER, TITLE_TEXT, VISUAL_HEADER,
};
use crate::{
    common::{
        LibraryStatus, PodcastChannelID, PodcastChannelParams, PodcastID, Thumbnail, VideoID,
    },
    nav_consts::{
        CAROUSEL, CAROUSEL_TITLE, DESCRIPTION, DESCRIPTION_SHELF, GRID_ITEMS, MMRLIR, MTRIR,
        MUSIC_SHELF, NAVIGATION_BROWSE, NAVIGATION_BROWSE_ID, PLAYBACK_DURATION_TEXT,
        PLAYBACK_PROGRESS_TEXT, RESPONSIVE_HEADER, SECTION_LIST, SECTION_LIST_ITEM,
        SINGLE_COLUMN_TAB, SUBTITLE, SUBTITLE_RUNS, TITLE, TWO_COLUMN,
    },
    query::{
        GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery,
        GetPodcastQuery,
    },
    utils, Result,
};
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcastChannel {
    title: String,
    thumbnails: Vec<Thumbnail>,
    episode_params: Option<PodcastChannelParams<'static>>,
    episodes: Vec<Episode>,
    podcasts: Vec<GetPodcastChannelPodcast>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Episode {
    title: String,
    description: String,
    total_duration: String,
    remaining_duration: String,
    date: String,
    video_id: VideoID<'static>,
    thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcastChannelPodcast {
    title: String,
    channels: Vec<ParsedPodcastChannel>,
    podcast_id: PodcastID<'static>,
    thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expected to change.
pub struct ParsedPodcastChannel {
    name: String,
    id: Option<PodcastChannelID<'static>>,
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
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPodcast {
    channels: Vec<ParsedPodcastChannel>,
    title: String,
    description: String,
    // TODO: How to add a podcast to library?
    library_status: LibraryStatus,
    episodes: Vec<Episode>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetEpisode {
    podcast_name: String,
    podcast_id: PodcastID<'static>,
    title: String,
    date: String,
    total_duration: String,
    remaining_duration: String,
    saved: IsSaved,
    description: String,
}

// NOTE: This is technically the same page as the GetArtist page. It's possible
// this could be generalised.
impl<'a> ParseFrom<GetChannelQuery<'a>> for GetPodcastChannel {
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
        let mut json_crawler = JsonCrawlerOwned::from(p);
        let mut header = json_crawler.borrow_pointer(VISUAL_HEADER)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let thumbnails = header.take_value_pointer(THUMBNAILS)?;
        let mut podcasts = Vec::new();
        let mut episodes = Vec::new();
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
            }
        }
        Ok(GetPodcastChannel {
            title,
            thumbnails,
            episode_params,
            episodes,
            podcasts,
        })
    }
}
impl<'a> ParseFrom<GetChannelEpisodesQuery<'a>> for Vec<Episode> {
    fn parse_from(p: crate::ProcessedResult<GetChannelEpisodesQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID_ITEMS))?
            .try_into_iter()?
            .map(parse_episode)
            .collect()
    }
}
impl<'a> ParseFrom<GetPodcastQuery<'a>> for GetPodcast {
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
impl<'a> ParseFrom<GetEpisodeQuery<'a>> for GetEpisode {
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
        let description_iter = two_column
            .navigate_pointer(concatcp!(
                SECONDARY_SECTION_LIST_ITEM,
                DESCRIPTION_SHELF,
                "/description/runs"
            ))?
            .try_into_iter()?
            .map(|mut item| item.take_value_pointer::<String>("/text"));
        let description =
            utils::process_results::process_results(description_iter, |iter| iter.collect())?;
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
    let video_id = title_run.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    Ok(Episode {
        title,
        description,
        total_duration,
        remaining_duration,
        date,
        video_id,
        thumbnails,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{PodcastChannelID, PodcastChannelParams, PodcastID, VideoID, YoutubeID},
        query::{
            GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery,
            GetPodcastQuery,
        },
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
            GetEpisodeQuery::new(VideoID::from_raw("")),
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
