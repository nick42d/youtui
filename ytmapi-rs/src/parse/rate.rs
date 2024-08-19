use super::ParseFrom;
use crate::query::rate::{RatePlaylistQuery, RateSongQuery};
use json_crawler::{JsonCrawler, JsonCrawlerIterator, JsonCrawlerOwned};

impl<'a> ParseFrom<RateSongQuery<'a>> for () {
    fn parse_from(_: super::ProcessedResult<RateSongQuery<'a>>) -> crate::Result<Self> {
        // Passing an invalid video ID with Like or Dislike will throw a 400 error which
        // is caught by AuthToken. Youtube does no checking on Indifferent, even
        // an invalid video ID will return no error code. Therefore, if we've
        // passed error validation at AuthToken, it's OK to return ApiSuccess here.
        Ok(())
    }
}
impl<'a> ParseFrom<RatePlaylistQuery<'a>> for () {
    fn parse_from(p: super::ProcessedResult<RatePlaylistQuery<'a>>) -> crate::Result<Self> {
        // Passing an invalid playlist ID to Like or Indifferent will throw a 404 error
        // which is caught by AuthToken. Youtube does no checking on
        // Indifferent, even an invalid PlaylistID will return success.
        let json_crawler = JsonCrawlerOwned::from(p);
        // TODO: Error type
        json_crawler
            .navigate_pointer("/actions")?
            .try_into_iter()?
            .find_path("/addToToastAction")
            .map(|_| ())
            .map_err(Into::into)
    }
}
#[cfg(test)]
mod tests {
    use crate::common::VideoID;
    use crate::{
        auth::BrowserToken,
        common::{PlaylistID, YoutubeID},
        query::rate::{RatePlaylistQuery, RateSongQuery},
    };

    #[tokio::test]
    async fn test_rate_song_like() {
        parse_test_value!(
            "./test_json/rate_song_like_20240710.json",
            (),
            RateSongQuery::new(VideoID::from_raw(""), crate::common::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_dislike() {
        parse_test_value!(
            "./test_json/rate_song_dislike_20240710.json",
            (),
            RateSongQuery::new(VideoID::from_raw(""), crate::common::LikeStatus::Disliked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_indifferent() {
        parse_test_value!(
            "./test_json/rate_song_indifferent_20240710.json",
            (),
            RateSongQuery::new(
                VideoID::from_raw(""),
                crate::common::LikeStatus::Indifferent
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_like() {
        parse_test_value!(
            "./test_json/rate_playlist_like_20240710.json",
            (),
            RatePlaylistQuery::new(PlaylistID::from_raw(""), crate::common::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_dislike() {
        parse_test_value!(
            "./test_json/rate_playlist_dislike_20240710.json",
            (),
            RatePlaylistQuery::new(
                PlaylistID::from_raw(""),
                crate::common::LikeStatus::Disliked
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_indifferent() {
        parse_test_value!(
            "./test_json/rate_playlist_indifferent_20240710.json",
            (),
            RatePlaylistQuery::new(
                PlaylistID::from_raw(""),
                crate::common::LikeStatus::Indifferent
            ),
            BrowserToken
        );
    }
}
