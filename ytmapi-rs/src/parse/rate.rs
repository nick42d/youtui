use super::TryParseFrom;
use crate::{
    crawler::{JsonCrawler, JsonCrawlerIterator},
    query::rate::{RatePlaylistQuery, RateSongQuery},
};

impl<'a> TryParseFrom<RateSongQuery<'a>> for () {
    fn parse_from(_: super::ProcessedResult<RateSongQuery<'a>>) -> crate::Result<Self> {
        // Passing an invalid video ID with Like or Dislike will throw a 400 error which
        // is caught by AuthToken. Youtube does no checking on Indifferent, even
        // an invalid video ID will return no error code. Therefore, if we've
        // passed error validation at AuthToken, it's OK to return ApiSuccess here.
        Ok(())
    }
}
impl<'a> TryParseFrom<RatePlaylistQuery<'a>> for () {
    fn parse_from(p: super::ProcessedResult<RatePlaylistQuery<'a>>) -> crate::Result<Self> {
        // Passing an invalid playlist ID to Like or Indifferent will throw a 404 error
        // which is caught by AuthToken. Youtube does no checking on
        // Indifferent, even an invalid PlaylistID will return success.
        let json_crawler = JsonCrawler::from(p);
        // TODO: Error type
        json_crawler
            .navigate_pointer("/actions")?
            .into_array_into_iter()?
            .find_path("/addToToastAction")
            .map(|_| ())
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{PlaylistID, YoutubeID},
        query::rate::{RatePlaylistQuery, RateSongQuery},
        VideoID,
    };

    #[tokio::test]
    async fn test_rate_song_like() {
        parse_test_value!(
            "./test_json/rate_song_like_20240710.json",
            (),
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_dislike() {
        parse_test_value!(
            "./test_json/rate_song_dislike_20240710.json",
            (),
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Disliked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_indifferent() {
        parse_test_value!(
            "./test_json/rate_song_indifferent_20240710.json",
            (),
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Indifferent),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_like() {
        parse_test_value!(
            "./test_json/rate_playlist_like_20240710.json",
            (),
            RatePlaylistQuery::new(PlaylistID::from_raw(""), crate::parse::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_dislike() {
        parse_test_value!(
            "./test_json/rate_playlist_dislike_20240710.json",
            (),
            RatePlaylistQuery::new(PlaylistID::from_raw(""), crate::parse::LikeStatus::Disliked),
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
                crate::parse::LikeStatus::Indifferent
            ),
            BrowserToken
        );
    }
}
