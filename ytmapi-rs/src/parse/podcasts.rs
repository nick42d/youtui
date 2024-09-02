use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

use super::{ParseFrom, THUMBNAILS, THUMBNAIL_RENDERER, TITLE_TEXT, VISUAL_HEADER};
use crate::{
    common::{PodcastChannelID, PodcastChannelParams, PodcastID, Thumbnail, VideoID},
    nav_consts::{SECTION_LIST, SINGLE_COLUMN_TAB},
    query::{
        GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery,
        GetPodcastQuery,
    },
    Result,
};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PodcastChannel {
    title: String,
    thumbnails: Vec<Thumbnail>,
    episode_params: PodcastChannelParams<'static>,
    episodes: Vec<PodcastChannelEpisode>,
    podcasts: Vec<PodcastChannelPodcast>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PodcastChannelEpisode {
    title: String,
    description: String,
    duration: String,
    date: String,
    video_id: VideoID<'static>,
    thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PodcastChannelPodcast {
    title: String,
    channel: Vec<ParsedPodcastChannel>,
    podcast_id: PodcastID<'static>,
    thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expected to change.
pub struct ParsedPodcastChannel {
    name: String,
    id: PodcastChannelID<'static>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
// Intentionally not marked non_exhaustive - not expected to change.
pub enum IsSaved {
    Saved,
    NotSaved,
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
        fn parse_podcast(crawler: impl JsonCrawler) -> Result<PodcastChannelEpisode> {
            todo!()
        }
        let json_crawler = JsonCrawlerOwned::from(p);
        let header = json_crawler.borrow_pointer(VISUAL_HEADER)?;
        let title = header.take_value_pointer(TITLE_TEXT)?;
        let thumbnails = header.take_value_pointer(THUMBNAILS)?;
        let contents = json_crawler
            .borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .try_into_iter()?;
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
