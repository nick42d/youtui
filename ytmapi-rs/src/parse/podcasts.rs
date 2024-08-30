use super::ParseFrom;
use crate::query::{
    GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetEpisodesPlaylistQuery,
    GetPodcastQuery,
};

impl ParseFrom<GetChannelQuery> for () {
    fn parse_from(p: crate::ProcessedResult<GetChannelQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl<'a> ParseFrom<GetChannelEpisodesQuery<'a>> for () {
    fn parse_from(p: crate::ProcessedResult<GetChannelEpisodesQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl ParseFrom<GetPodcastQuery> for () {
    fn parse_from(p: crate::ProcessedResult<GetPodcastQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl ParseFrom<GetEpisodeQuery> for () {
    fn parse_from(p: crate::ProcessedResult<GetEpisodeQuery>) -> crate::Result<Self> {
        todo!()
    }
}
impl ParseFrom<GetEpisodesPlaylistQuery> for () {
    fn parse_from(p: crate::ProcessedResult<GetEpisodesPlaylistQuery>) -> crate::Result<Self> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        query::{
            GetChannelEpisodesQuery, GetChannelQuery, GetEpisodeQuery, GetEpisodesPlaylistQuery,
            GetPodcastQuery,
        },
    };

    #[tokio::test]
    async fn test_get_channel() {
        parse_test_value!(
            "./test_json/get_channel_20240830.json",
            "./test_json/get_channel_20240830_output.txt",
            GetChannelQuery::new(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_channel_episodes() {
        parse_test!(
            "./test_json/get_channel_episodes_20240830.json",
            "./test_json/get_channel_episodes_20240830_output.txt",
            GetChannelEpisodesQuery::new(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_podcast() {
        parse_test_value!(
            "./test_json/get_podcast_20240830.json",
            "./test_json/get_podcast_20240830_output.txt",
            GetPodcastQuery::new(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_episode() {
        parse_test_value!(
            "./test_json/get_episode_20240830.json",
            "./test_json/get_episode_20240830_output.txt",
            GetEpisodeQuery::new(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_episodes_playlist() {
        parse_test_value!(
            "./test_json/get_episodes_playlist_20240830.json",
            "./test_json/get_episodes_playlist_20240830_output.txt",
            GetEpisodesPlaylistQuery::new(),
            BrowserToken
        );
    }
}
