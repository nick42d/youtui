use super::{ParseFrom, THUMBNAILS, THUMBNAIL_RENDERER, TITLE_TEXT, VISUAL_HEADER};
use crate::{
    common::{PodcastChannelID, PodcastChannelParams, PodcastID, Thumbnail, VideoID},
    nav_consts::{
        CAROUSEL, CAROUSEL_CONTENTS, CAROUSEL_TITLE, DESCRIPTION, MMRLIR, MTRIR, NAVIGATION_BROWSE,
        NAVIGATION_BROWSE_ID, PLAYBACK_DURATION_TEXT, PLAYBACK_PROGRESS_TEXT, SECTION_LIST,
        SINGLE_COLUMN_TAB, SUBTITLE, SUBTITLE_RUNS, TITLE,
    },
    parse::podcasts,
    query::{
        GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery,
        GetPodcastQuery,
    },
    utils, Result,
};
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PodcastChannel {
    title: String,
    thumbnails: Vec<Thumbnail>,
    episode_params: Option<PodcastChannelParams<'static>>,
    episodes: Vec<PodcastChannelEpisode>,
    podcasts: Vec<PodcastChannelPodcast>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PodcastChannelEpisode {
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
pub struct PodcastChannelPodcast {
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
pub struct Podcast {
    // There can be multiple of these - put these into an array
    channel: Vec<ParsedPodcastChannel>,
    title: String,
    description: String,
    saved: IsSaved,
    episodes: Vec<PodcastChannelEpisode>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetEpisode {
    channel: Vec<ParsedPodcastChannel>,
    title: String,
    date: String,
    duration: String,
    saved: IsSaved,
    description: String,
}

// NOTE: This is technically the same page as the GetArtist page. It's possible
// this could be generalised.
impl<'a> ParseFrom<GetChannelQuery<'a>> for PodcastChannel {
    fn parse_from(p: crate::ProcessedResult<GetChannelQuery>) -> Result<Self> {
        fn parse_podcast(crawler: impl JsonCrawler) -> Result<PodcastChannelPodcast> {
            let mut podcast = crawler.navigate_pointer(MTRIR)?;
            let title = podcast.take_value_pointer(TITLE_TEXT)?;
            let podcast_id = podcast.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let thumbnails = podcast.take_value_pointer(THUMBNAIL_RENDERER)?;
            let channels = podcast
                .navigate_pointer(SUBTITLE_RUNS)?
                .try_into_iter()?
                .map(parse_podcast_channel)
                .collect::<Result<Vec<_>>>()?;
            Ok(PodcastChannelPodcast {
                title,
                channels,
                podcast_id,
                thumbnails,
            })
        }
        fn parse_episode(crawler: impl JsonCrawler) -> Result<PodcastChannelEpisode> {
            let mut episode = crawler.navigate_pointer(MMRLIR)?;
            let description = episode.take_value_pointer(DESCRIPTION)?;
            let total_duration = episode.take_value_pointer(PLAYBACK_DURATION_TEXT)?;
            let remaining_duration = episode.take_value_pointer(PLAYBACK_PROGRESS_TEXT)?;
            let date = episode.take_value_pointer(SUBTITLE)?;
            let thumbnails = episode.take_value_pointer(THUMBNAILS)?;
            let mut title_run = episode.navigate_pointer(TITLE)?;
            let title = title_run.take_value_pointer("/text")?;
            let video_id = title_run.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            Ok(PodcastChannelEpisode {
                title,
                description,
                total_duration,
                remaining_duration,
                date,
                video_id,
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
        Ok(PodcastChannel {
            title,
            thumbnails,
            episode_params,
            episodes,
            podcasts,
        })
    }
}
impl<'a> ParseFrom<GetChannelEpisodesQuery<'a>> for Vec<PodcastChannelEpisode> {
    fn parse_from(p: crate::ProcessedResult<GetChannelEpisodesQuery>) -> Result<Self> {
        todo!()
    }
}
impl<'a> ParseFrom<GetPodcastQuery<'a>> for Podcast {
    fn parse_from(p: crate::ProcessedResult<GetPodcastQuery>) -> Result<Self> {
        Ok(Podcast {
            channel: todo!(),
            title: todo!(),
            description: todo!(),
            saved: todo!(),
            episodes: todo!(),
        })
    }
}
impl<'a> ParseFrom<GetEpisodeQuery<'a>> for GetEpisode {
    fn parse_from(p: crate::ProcessedResult<GetEpisodeQuery>) -> Result<Self> {
        Ok(GetEpisode {
            channel: todo!(),
            title: todo!(),
            date: todo!(),
            duration: todo!(),
            saved: todo!(),
            description: todo!(),
        })
    }
}
impl ParseFrom<GetNewEpisodesQuery> for Podcast {
    fn parse_from(p: crate::ProcessedResult<GetNewEpisodesQuery>) -> Result<Self> {
        Ok(Podcast {
            channel: todo!(),
            title: todo!(),
            description: todo!(),
            saved: todo!(),
            episodes: todo!(),
        })
    }
}

fn parse_podcast_channel(mut data: impl JsonCrawler) -> Result<ParsedPodcastChannel> {
    Ok(ParsedPodcastChannel {
        name: data.take_value_pointer("/text")?,
        id: data.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
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
