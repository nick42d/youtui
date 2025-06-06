use super::{
    parse_library_management_items_from_menu, parse_upload_song_album, parse_upload_song_artists,
    EpisodeDate, EpisodeDuration, ParseFrom, ParsedSongAlbum, ParsedUploadArtist,
    ParsedUploadSongAlbum, BADGE_LABEL, DELETION_ENTITY_ID, MENU_ITEMS, MENU_LIKE_STATUS, MRLIR,
    MUSIC_SHELF, TEXT_RUN_TEXT, THUMBNAILS, TITLE_TEXT,
};
use crate::common::{
    ApiOutcome, ArtistChannelID, EpisodeID, Explicit, FeedbackTokenRemoveFromHistory,
    LibraryManager, LikeStatus, PlaylistID, Thumbnail, UploadEntityID, VideoID,
};
use crate::nav_consts::{
    FEEDBACK_TOKEN, LIVE_BADGE_LABEL, MENU_SERVICE, NAVIGATION_BROWSE_ID, NAVIGATION_PLAYLIST_ID,
    NAVIGATION_VIDEO_TYPE, PLAY_BUTTON, SECTION_LIST, SINGLE_COLUMN_TAB, TEXT_RUN, WATCH_VIDEO_ID,
};
use crate::parse::parse_flex_column_item;
use crate::process::{fixed_column_item_pointer, flex_column_item_pointer};
use crate::query::{AddHistoryItemQuery, GetHistoryQuery, RemoveHistoryItemsQuery};
use crate::youtube_enums::YoutubeMusicVideoType;
use crate::Result;
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct HistoryPeriod {
    pub period_name: String,
    pub items: Vec<HistoryItem>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum HistoryItem {
    Song(HistoryItemSong),
    Video(HistoryItemVideo),
    Episode(HistoryItemEpisode),
    UploadSong(HistoryItemUploadSong),
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// Could this alternatively be Result<Song>?
// May need to be enum to track 'Not Available' case.
pub struct HistoryItemSong {
    pub video_id: VideoID<'static>,
    pub album: ParsedSongAlbum,
    pub duration: String,
    /// Some songs may not have library management features. There could be
    /// various resons for this.
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub artists: Vec<super::ParsedSongArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<super::Thumbnail>,
    pub explicit: Explicit,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
    pub feedback_token_remove: FeedbackTokenRemoveFromHistory<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct HistoryItemVideo {
    pub video_id: VideoID<'static>,
    pub duration: String,
    pub title: String,
    // Could be 'ParsedVideoChannel'
    pub channel_name: String,
    pub channel_id: ArtistChannelID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<super::Thumbnail>,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
    pub feedback_token_remove: FeedbackTokenRemoveFromHistory<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct HistoryItemEpisode {
    pub episode_id: EpisodeID<'static>,
    // May be live or non-live...
    pub date: EpisodeDate,
    pub duration: EpisodeDuration,
    pub title: String,
    pub podcast_name: String,
    pub podcast_id: PlaylistID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<super::Thumbnail>,
    pub is_available: bool,
    pub feedback_token_remove: FeedbackTokenRemoveFromHistory<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// May need to be enum to track 'Not Available' case.
// TODO: Move to common
pub struct HistoryItemUploadSong {
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub album: ParsedUploadSongAlbum,
    pub duration: String,
    pub like_status: LikeStatus,
    pub title: String,
    pub artists: Vec<ParsedUploadArtist>,
    pub thumbnails: Vec<Thumbnail>,
    pub feedback_token_remove: FeedbackTokenRemoveFromHistory<'static>,
}

impl ParseFrom<GetHistoryQuery> for Vec<HistoryPeriod> {
    fn parse_from(p: super::ProcessedResult<GetHistoryQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        let contents = json_crawler.navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        contents
            .try_into_iter()?
            .map(parse_history_period)
            .collect()
    }
}
impl ParseFrom<RemoveHistoryItemsQuery<'_>> for Vec<ApiOutcome> {
    fn parse_from(p: super::ProcessedResult<RemoveHistoryItemsQuery>) -> Result<Self> {
        let json_crawler = JsonCrawlerOwned::from(p);
        json_crawler
            .navigate_pointer("/feedbackResponses")?
            .try_into_iter()?
            .map(|mut response| {
                response
                    .take_value_pointer::<bool>("/isProcessed")
                    .map(|p| {
                        if p {
                            return ApiOutcome::Success;
                        }
                        // Better handled in another way...
                        ApiOutcome::Failure
                    })
            })
            .rev()
            .collect::<json_crawler::CrawlerResult<_>>()
            .map_err(Into::into)
    }
}
impl ParseFrom<AddHistoryItemQuery<'_>> for () {
    fn parse_from(_: crate::parse::ProcessedResult<AddHistoryItemQuery>) -> crate::Result<Self> {
        // Api only returns an empty string, no way of validating if correct or not.
        Ok(())
    }
}

fn parse_history_period(json: JsonCrawlerOwned) -> Result<HistoryPeriod> {
    let mut data = json.navigate_pointer(MUSIC_SHELF)?;
    let period_name = data.take_value_pointer(TITLE_TEXT)?;
    let items = data
        .navigate_pointer("/contents")?
        .try_into_iter()?
        .filter_map(|item| parse_history_item(item).transpose())
        .collect::<Result<_>>()?;
    Ok(HistoryPeriod { period_name, items })
}
fn parse_history_item(mut json: JsonCrawlerOwned) -> Result<Option<HistoryItem>> {
    let Ok(mut data) = json.borrow_pointer(MRLIR) else {
        return Ok(None);
    };
    let title = super::parse_flex_column_item(&mut data, 0, 0)?;
    if title == "Shuffle all" {
        return Ok(None);
    }
    let video_type_path = concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        NAVIGATION_VIDEO_TYPE
    );
    let video_type: YoutubeMusicVideoType = data.take_value_pointer(video_type_path)?;
    let item = match video_type {
        // NOTE - Possible for History, but most likely not possible for Library.
        YoutubeMusicVideoType::Upload => Some(HistoryItem::UploadSong(
            parse_history_item_upload_song(title, data)?,
        )),
        // NOTE - Possible for Library, but most likely not possible for History.
        YoutubeMusicVideoType::Episode => Some(HistoryItem::Episode(parse_history_item_episode(
            title, data,
        )?)),
        YoutubeMusicVideoType::Ugc | YoutubeMusicVideoType::Omv => {
            Some(HistoryItem::Video(parse_history_item_video(title, data)?))
        }
        YoutubeMusicVideoType::Atv => {
            Some(HistoryItem::Song(parse_history_item_song(title, data)?))
        }
    };
    Ok(item)
}

fn parse_history_item_episode(
    title: String,
    mut data: JsonCrawlerBorrowed,
) -> Result<HistoryItemEpisode> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let is_live = data.path_exists(LIVE_BADGE_LABEL);
    let (duration, date) = match is_live {
        true => (EpisodeDuration::Live, EpisodeDate::Live),
        false => {
            let date = parse_flex_column_item(&mut data, 2, 0)?;
            let duration = data
                .borrow_pointer(fixed_column_item_pointer(0))?
                .take_value_pointers(&["/text/simpleText", "/text/runs/0/text"])?;
            (
                EpisodeDuration::Recorded { duration },
                EpisodeDate::Recorded { date },
            )
        }
    };
    let podcast_name = parse_flex_column_item(&mut data, 1, 0)?;
    let podcast_id = data
        .borrow_pointer(flex_column_item_pointer(1))?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    let feedback_token_remove = data
        .navigate_pointer(MENU_ITEMS)?
        .try_into_iter()?
        .find_path(concatcp!(MENU_SERVICE, FEEDBACK_TOKEN))?
        .take_value()?;
    Ok(HistoryItemEpisode {
        episode_id: video_id,
        duration,
        title,
        like_status,
        thumbnails,
        date,
        podcast_name,
        podcast_id,
        is_available,
        feedback_token_remove,
    })
}
fn parse_history_item_video(
    title: String,
    mut data: JsonCrawlerBorrowed,
) -> Result<HistoryItemVideo> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let channel_name = parse_flex_column_item(&mut data, 1, 0)?;
    let channel_id = data
        .borrow_pointer(flex_column_item_pointer(1))?
        .take_value_pointer(concatcp!(TEXT_RUN, NAVIGATION_BROWSE_ID))?;
    let duration = data
        .borrow_pointer(fixed_column_item_pointer(0))?
        .take_value_pointers(&["/text/simpleText", "/text/runs/0/text"])?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    let mut menu = data.navigate_pointer(MENU_ITEMS)?;
    let playlist_id = menu.take_value_pointer(concatcp!(
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    let feedback_token_remove = menu
        .try_into_iter()?
        .find_path(concatcp!(MENU_SERVICE, FEEDBACK_TOKEN))?
        .take_value()?;
    Ok(HistoryItemVideo {
        video_id,
        duration,
        title,
        like_status,
        thumbnails,
        playlist_id,
        is_available,
        channel_name,
        channel_id,
        feedback_token_remove,
    })
}
fn parse_history_item_upload_song(
    title: String,
    mut data: JsonCrawlerBorrowed,
) -> Result<HistoryItemUploadSong> {
    let duration = data
        .borrow_pointer(fixed_column_item_pointer(0))?
        .take_value_pointer(TEXT_RUN_TEXT)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint/watchEndpoint/videoId"
    ))?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let artists = parse_upload_song_artists(data.borrow_mut(), 1)?;
    let album = parse_upload_song_album(data.borrow_mut(), 2)?;
    let mut menu = data.navigate_pointer(MENU_ITEMS)?;
    let entity_id = menu
        .try_iter_mut()?
        .find_path(DELETION_ENTITY_ID)?
        .take_value()?;
    let feedback_token_remove = menu
        .try_into_iter()?
        .find_path(concatcp!(MENU_SERVICE, FEEDBACK_TOKEN))?
        .take_value()?;
    Ok(HistoryItemUploadSong {
        entity_id,
        video_id,
        album,
        duration,
        like_status,
        title,
        artists,
        thumbnails,
        feedback_token_remove,
    })
}
fn parse_history_item_song(
    title: String,
    mut data: JsonCrawlerBorrowed,
) -> Result<HistoryItemSong> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let library_management =
        parse_library_management_items_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let artists = super::parse_song_artists(&mut data, 1)?;
    let album = super::parse_song_album(&mut data, 2)?;
    let duration = data
        .borrow_pointer(fixed_column_item_pointer(0))?
        .take_value_pointers(&["/text/simpleText", "/text/runs/0/text"])?;
    let thumbnails = data.take_value_pointer(THUMBNAILS)?;
    let is_available = data
        .take_value_pointer::<String>("/musicItemRendererDisplayPolicy")
        .map(|m| m != "MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        .unwrap_or(true);
    let explicit = if data.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let mut menu = data.navigate_pointer(MENU_ITEMS)?;
    let playlist_id = menu.take_value_pointer(concatcp!(
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    let feedback_token_remove = menu
        .try_into_iter()?
        .find_path(concatcp!(MENU_SERVICE, FEEDBACK_TOKEN))?
        .take_value()?;
    Ok(HistoryItemSong {
        video_id,
        duration,
        library_management,
        title,
        artists,
        like_status,
        thumbnails,
        explicit,
        album,
        playlist_id,
        is_available,
        feedback_token_remove,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{SongTrackingUrl, YoutubeID};
    use crate::query::AddHistoryItemQuery;

    #[tokio::test]
    async fn test_add_history_item_query() {
        let source = String::new();
        crate::process_json::<_, BrowserToken>(
            source,
            AddHistoryItemQuery::new(SongTrackingUrl::from_raw("")),
        )
        .unwrap();
    }
    #[tokio::test]
    async fn test_get_history() {
        parse_test!(
            "./test_json/get_history_20240701.json",
            "./test_json/get_history_20240701_output.txt",
            crate::query::GetHistoryQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_history_with_upload_song() {
        parse_test!(
            "./test_json/get_history_20240713.json",
            "./test_json/get_history_20240713_output.txt",
            crate::query::GetHistoryQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_remove_history_items() {
        parse_test!(
            "./test_json/remove_history_items_20240704.json",
            "./test_json/remove_history_items_20240704_output.txt",
            crate::query::RemoveHistoryItemsQuery::new(Vec::new()),
            BrowserToken
        );
    }
}
