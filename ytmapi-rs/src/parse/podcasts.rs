use super::ParseFrom;
use crate::query::{
    GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetNewEpisodesQuery, GetPodcastQuery,
};

// NOTE: This is technically the same page as the GetArtist page. It's possible
// this could be generalised.
impl<'a> ParseFrom<GetChannelQuery<'a>> for () {
    fn parse_from(p: crate::ProcessedResult<GetChannelQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl<'a> ParseFrom<GetChannelEpisodesQuery<'a>> for () {
    fn parse_from(p: crate::ProcessedResult<GetChannelEpisodesQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl<'a> ParseFrom<GetPodcastQuery<'a>> for () {
    fn parse_from(p: crate::ProcessedResult<GetPodcastQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl<'a> ParseFrom<GetEpisodeQuery<'a>> for () {
    fn parse_from(p: crate::ProcessedResult<GetEpisodeQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl ParseFrom<GetNewEpisodesQuery> for () {
    fn parse_from(p: crate::ProcessedResult<GetNewEpisodesQuery>) -> crate::Result<Self> {
        todo!()
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
