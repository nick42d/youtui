use super::{ApiSuccess, ParseFrom};
use crate::{
    crawler::JsonCrawler,
    query::rate::{RatePlaylistQuery, RateSongQuery},
    Error,
};

impl<'a> ParseFrom<RateSongQuery<'a>> for ApiSuccess {
    fn parse_from(
        _: super::ProcessedResult<RateSongQuery<'a>>,
    ) -> crate::Result<<RateSongQuery<'a> as crate::query::Query>::Output> {
        // Passing an invalid video ID with Like or Dislike will throw a 400 error which
        // is caught by AuthToken. Youtube does no checking on Indifferent, even
        // an invalid video ID will return no error code. Therefore, if we've
        // passed error validation at AuthToken, it's OK to return ApiSuccess here.
        Ok(ApiSuccess)
    }
}
impl<'a> ParseFrom<RatePlaylistQuery<'a>> for ApiSuccess {
    fn parse_from(
        p: super::ProcessedResult<RatePlaylistQuery<'a>>,
    ) -> crate::Result<<RatePlaylistQuery<'a> as crate::query::Query>::Output> {
        // Passing an invalid playlist ID to Like or Indifferent will throw a 404 error
        // which is caught by AuthToken. Youtube does no checking on
        // Indifferent, even an invalid PlaylistID will return success.
        let json_crawler = JsonCrawler::from(p);
        // TODO: Error type
        json_crawler
            .navigate_pointer("/actions")?
            .into_array_into_iter()?
            .find_map(|a| a.navigate_pointer("/addToToastAction").ok())
            .map(|_| ApiSuccess)
            .ok_or_else(|| Error::other("Expected /actions to contain a /addToToastAction"))
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
        parse_test!(
            "./test_json/rate_song_like_20240710.json",
            "./test_json/rate_song_like_20240710_output.txt",
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_dislike() {
        parse_test!(
            "./test_json/rate_song_dislike_20240710.json",
            "./test_json/rate_song_dislike_20240710_output.txt",
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Disliked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_song_indifferent() {
        parse_test!(
            "./test_json/rate_song_indifferent_20240710.json",
            "./test_json/rate_song_indifferent_20240710_output.txt",
            RateSongQuery::new(VideoID::from_raw(""), crate::parse::LikeStatus::Indifferent),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_like() {
        parse_test!(
            "./test_json/rate_playlist_like_20240710.json",
            "./test_json/rate_playlist_like_20240710_output.txt",
            RatePlaylistQuery::new(PlaylistID::from_raw(""), crate::parse::LikeStatus::Liked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_dislike() {
        parse_test!(
            "./test_json/rate_playlist_dislike_20240710.json",
            "./test_json/rate_playlist_dislike_20240710_output.txt",
            RatePlaylistQuery::new(PlaylistID::from_raw(""), crate::parse::LikeStatus::Disliked),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_rate_playlist_indifferent() {
        parse_test!(
            "./test_json/rate_playlist_indifferent_20240710.json",
            "./test_json/rate_playlist_indifferent_20240710_output.txt",
            RatePlaylistQuery::new(
                PlaylistID::from_raw(""),
                crate::parse::LikeStatus::Indifferent
            ),
            BrowserToken
        );
    }
}
