use super::ParseFrom;
use crate::common::{PlaylistID, Thumbnail, UserPlaylistsParams, UserVideosParams, VideoID};
use crate::nav_consts::{
    CAROUSEL, CAROUSEL_TITLE, FOREGROUND_THUMBNAIL_RENDERER, GRID_ITEMS, MTRIR, NAVIGATION_BROWSE,
    NAVIGATION_BROWSE_ID, NAVIGATION_VIDEO_ID, SECTION_LIST, SECTION_LIST_ITEM, SINGLE_COLUMN_TAB,
    SUBTITLE2, SUBTITLE3, THUMBNAIL_RENDERER, TITLE_TEXT, VISUAL_HEADER,
};
use crate::query::{GetUserPlaylistsQuery, GetUserQuery, GetUserVideosQuery};
use crate::Result;
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GetUser {
    pub name: String,
    pub videos: Vec<UserVideo>,
    pub thumbnails: Vec<Thumbnail>,
    pub all_videos_params: Option<UserVideosParams<'static>>,
    pub playlists: Vec<UserPlaylist>,
    pub all_playlists_params: Option<UserPlaylistsParams<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserVideo {
    pub title: String,
    pub views: String,
    pub thumbnails: Vec<Thumbnail>,
    pub id: VideoID<'static>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserPlaylist {
    pub title: String,
    pub views: String,
    pub thumbnails: Vec<Thumbnail>,
    pub id: PlaylistID<'static>,
}

impl ParseFrom<GetUserQuery<'_>> for GetUser {
    fn parse_from(p: super::ProcessedResult<GetUserQuery>) -> Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
        let mut header = json_crawler.borrow_pointer(VISUAL_HEADER)?;
        let name = header.take_value_pointer(TITLE_TEXT)?;
        let thumbnails = header.take_value_pointer(FOREGROUND_THUMBNAIL_RENDERER)?;
        let contents = json_crawler.navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        // TODO: i18n
        let mut carousels: HashMap<String, _> = contents
            .try_into_iter()?
            .map(|crawler| {
                let mut carousel = crawler.navigate_pointer(CAROUSEL)?;
                let title = carousel.take_value_pointer(concatcp!(CAROUSEL_TITLE, "/text"))?;
                Ok((title, carousel))
            })
            .collect::<Result<_>>()?;
        let mut maybe_playlists_carousel = carousels.get_mut("Playlists");
        let all_playlists_params =
            maybe_playlists_carousel
                .as_mut()
                .and_then(|playlists_carousel| {
                    playlists_carousel
                        .take_value_pointer(concatcp!(CAROUSEL_TITLE, NAVIGATION_BROWSE, "/params"))
                        .ok()
                });
        let playlists = match maybe_playlists_carousel {
            Some(playlists_carousel) => playlists_carousel
                .borrow_pointer("/contents")?
                .try_into_iter()?
                .map(parse_user_playlist)
                .collect::<Result<_>>()?,
            None => vec![],
        };
        let mut maybe_videos_carousel = carousels.get_mut("Videos");
        let all_videos_params = maybe_videos_carousel.as_mut().and_then(|videos_carousel| {
            videos_carousel
                .take_value_pointer(concatcp!(CAROUSEL_TITLE, NAVIGATION_BROWSE, "/params"))
                .ok()
        });
        let videos = match maybe_videos_carousel {
            Some(videos_carousel) => videos_carousel
                .borrow_pointer("/contents")?
                .try_into_iter()?
                .map(parse_user_video)
                .collect::<Result<_>>()?,
            None => vec![],
        };

        Ok(Self {
            name,
            thumbnails,
            all_videos_params,
            playlists,
            videos,
            all_playlists_params,
        })
    }
}
impl ParseFrom<GetUserPlaylistsQuery<'_>> for Vec<UserPlaylist> {
    fn parse_from(p: super::ProcessedResult<GetUserPlaylistsQuery>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let results = json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID_ITEMS))?
            .try_into_iter()?
            .map(parse_user_playlist)
            .collect::<Result<_>>()?;
        Ok(results)
    }
}
impl ParseFrom<GetUserVideosQuery<'_>> for Vec<UserVideo> {
    fn parse_from(p: super::ProcessedResult<GetUserVideosQuery>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let results = json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID_ITEMS))?
            .try_into_iter()?
            .map(parse_user_video)
            .collect::<Result<_>>()?;
        Ok(results)
    }
}
fn parse_user_video(c: impl JsonCrawler) -> Result<UserVideo> {
    let mut item = c.navigate_pointer(MTRIR)?;
    let title = item.take_value_pointer(TITLE_TEXT)?;
    let views = item.take_value_pointer(SUBTITLE2)?;
    let id = item.take_value_pointer(NAVIGATION_VIDEO_ID)?;
    let thumbnails = item.take_value_pointer(THUMBNAIL_RENDERER)?;
    Ok(UserVideo {
        title,
        views,
        thumbnails,
        id,
    })
}
fn parse_user_playlist(c: impl JsonCrawler) -> Result<UserPlaylist> {
    let mut item = c.navigate_pointer(MTRIR)?;
    let title = item.take_value_pointer(TITLE_TEXT)?;
    let views = item.take_value_pointer(SUBTITLE3)?;
    let id = item.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails = item.take_value_pointer(THUMBNAIL_RENDERER)?;
    Ok(UserPlaylist {
        title,
        views,
        thumbnails,
        id,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{UserChannelID, UserPlaylistsParams, UserVideosParams, YoutubeID};

    #[tokio::test]
    async fn test_get_user() {
        parse_test!(
            "./test_json/get_user_20250707.json",
            "./test_json/get_user_20250707_output.txt",
            crate::query::GetUserQuery::new(UserChannelID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_user_playlists() {
        parse_test!(
            "./test_json/get_user_playlists_20250707.json",
            "./test_json/get_user_playlists_20250707_output.txt",
            crate::query::GetUserPlaylistsQuery::new(
                UserChannelID::from_raw(""),
                UserPlaylistsParams::from_raw("")
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_user_videos() {
        parse_test!(
            "./test_json/get_user_videos_20250707.json",
            "./test_json/get_user_videos_20250707_output.txt",
            crate::query::GetUserVideosQuery::new(
                UserChannelID::from_raw(""),
                UserVideosParams::from_raw("")
            ),
            BrowserToken
        );
    }
}
