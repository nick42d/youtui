use super::{
    DESCRIPTION_SHELF_RUNS, EpisodeDate, EpisodeDuration, ParseFrom, ParsedSongAlbum,
    ParsedUploadArtist, ParsedUploadSongAlbum, ProcessedResult, STRAPLINE_TEXT, TITLE_TEXT,
    TWO_COLUMN, fixed_column_item_pointer, flex_column_item_pointer, parse_flex_column_item,
    parse_library_management_items_from_menu, parse_upload_song_album, parse_upload_song_artists,
};
use crate::common::{
    ApiOutcome, ArtistChannelID, ContinuationParams, EpisodeID, Explicit, LibraryManager,
    LikeStatus, PlaylistID, SetVideoID, Thumbnail, UploadEntityID, VideoID,
};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    APPEND_CONTINUATION_ITEMS, BADGE_LABEL, CONTENT, CONTINUATION_RENDERER_COMMAND,
    DELETION_ENTITY_ID, DISPLAY_POLICY, FACEPILE_AVATAR_URL, FACEPILE_TEXT, LIVE_BADGE_LABEL,
    MENU_ITEMS, MENU_LIKE_STATUS, MRLIR, MUSIC_PLAYLIST_SHELF, NAVIGATION_BROWSE_ID,
    NAVIGATION_PLAYLIST_ID, NAVIGATION_VIDEO_ID, NAVIGATION_VIDEO_TYPE, PLAY_BUTTON,
    PLAYLIST_PANEL_CONTINUATION, PPR, RADIO_CONTINUATION_PARAMS, RESPONSIVE_HEADER, RUN_TEXT,
    SECOND_SUBTITLE_RUNS, SECONDARY_SECTION_LIST_RENDERER, SECTION_LIST_ITEM, TAB_CONTENT,
    TEXT_RUN, TEXT_RUN_TEXT, THUMBNAIL, THUMBNAILS, WATCH_NEXT_CONTENT, WATCH_VIDEO_ID,
};
use crate::query::playlist::{
    CreatePlaylistType, GetPlaylistDetailsQuery, GetWatchPlaylistQueryID, PrivacyStatus,
    SpecialisedQuery,
};
use crate::query::{
    AddPlaylistItemsQuery, CreatePlaylistQuery, DeletePlaylistQuery, EditPlaylistQuery,
    GetPlaylistTracksQuery, GetWatchPlaylistQuery, RemovePlaylistItemsQuery,
};
use crate::youtube_enums::YoutubeMusicVideoType;
use crate::{Error, Result};
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GetPlaylistDetails {
    pub id: PlaylistID<'static>,
    // NOTE: Only present on personal (library) playlists??
    // NOTE: May not be present on old version of API also.
    pub privacy: Option<PrivacyStatus>,
    pub title: String,
    pub description: Option<String>,
    pub author: String,
    pub author_avatar_url: Option<String>,
    pub year: String,
    pub duration: String,
    pub track_count_text: String,
    // NOTE: Seem to be unable to distinguish when views is optional.
    pub views: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
/// Provides a SetVideoID and VideoID for each video added to the playlist.
// Intentionally not marked non_exhaustive - not expecting this to change.
pub struct AddPlaylistItem {
    pub video_id: VideoID<'static>,
    pub set_video_id: SetVideoID<'static>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WatchPlaylistTrack {
    pub title: String,
    pub author: String,
    pub duration: String,
    pub thumbnails: Vec<Thumbnail>,
    pub video_id: VideoID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
// Could this alternatively be Result<Song>?
// May need to be enum to track 'Not Available' case.
pub struct PlaylistSong {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub album: ParsedSongAlbum,
    pub duration: String,
    /// Some songs may not have library management features. There could be
    /// various resons for this.
    pub library_management: Option<LibraryManager>,
    pub title: String,
    pub artists: Vec<super::ParsedSongArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: Explicit,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum PlaylistItem {
    Song(PlaylistSong),
    Video(PlaylistVideo),
    Episode(PlaylistEpisode),
    UploadSong(PlaylistUploadSong),
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PlaylistVideo {
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub title: String,
    // Could be 'ParsedVideoChannel'
    pub channel_name: String,
    pub channel_id: ArtistChannelID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub is_available: bool,
    /// Id of the playlist that will get created when pressing 'Start Radio'.
    pub playlist_id: PlaylistID<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PlaylistEpisode {
    pub episode_id: EpisodeID<'static>,
    pub track_no: usize,
    pub date: EpisodeDate,
    pub duration: EpisodeDuration,
    pub title: String,
    pub podcast_name: String,
    pub podcast_id: PlaylistID<'static>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub is_available: bool,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PlaylistUploadSong {
    pub entity_id: UploadEntityID<'static>,
    pub video_id: VideoID<'static>,
    pub track_no: usize,
    pub duration: String,
    pub album: ParsedUploadSongAlbum,
    pub title: String,
    pub artists: Vec<ParsedUploadArtist>,
    // TODO: Song like feedback tokens.
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for () {
    fn parse_from(_: ProcessedResult<RemovePlaylistItemsQuery<'a>>) -> crate::Result<Self> {
        Ok(())
    }
}
impl<'a, C: CreatePlaylistType> ParseFrom<CreatePlaylistQuery<'a, C>> for PlaylistID<'static> {
    fn parse_from(p: ProcessedResult<CreatePlaylistQuery<'a, C>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
        json_crawler
            .take_value_pointer("/playlistId")
            .map_err(Into::into)
    }
}
impl<'a, T: SpecialisedQuery> ParseFrom<AddPlaylistItemsQuery<'a, T>> for Vec<AddPlaylistItem> {
    fn parse_from(p: ProcessedResult<AddPlaylistItemsQuery<'a, T>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
        let status: ApiOutcome = json_crawler.borrow_pointer("/status")?.take_value()?;
        if let ApiOutcome::Failure = status {
            return Err(Error::status_failed());
        }
        json_crawler
            .navigate_pointer("/playlistEditResults")?
            .try_iter_mut()?
            .map(|r| {
                let mut r = r.navigate_pointer("/playlistEditVideoAddedResultData")?;
                Ok(AddPlaylistItem {
                    video_id: r.take_value_pointer("/videoId")?,
                    set_video_id: r.take_value_pointer("/setVideoId")?,
                })
            })
            .collect()
    }
}
impl<'a> ParseFrom<EditPlaylistQuery<'a>> for ApiOutcome {
    fn parse_from(p: ProcessedResult<EditPlaylistQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        json_crawler
            .navigate_pointer("/status")?
            .take_value()
            .map_err(Into::into)
    }
}
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for () {
    fn parse_from(_: ProcessedResult<DeletePlaylistQuery<'a>>) -> crate::Result<Self> {
        Ok(())
    }
}

impl<'a> ParseFrom<GetPlaylistDetailsQuery<'a>> for GetPlaylistDetails {
    fn parse_from(p: ProcessedResult<GetPlaylistDetailsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        get_playlist_details(json_crawler)
    }
}

impl<'a> ParseFromContinuable<GetPlaylistTracksQuery<'a>> for Vec<PlaylistItem> {
    fn parse_from_continuable(
        p: ProcessedResult<GetPlaylistTracksQuery<'a>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let music_playlist_shelf = json_crawler.navigate_pointer(concatcp!(
            TWO_COLUMN,
            SECONDARY_SECTION_LIST_RENDERER,
            CONTENT,
            MUSIC_PLAYLIST_SHELF,
            "/contents"
        ))?;
        parse_playlist_items(music_playlist_shelf)
    }
    fn parse_continuation(
        p: ProcessedResult<crate::query::GetContinuationsQuery<'_, GetPlaylistTracksQuery<'a>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let continuation_items = json_crawler.navigate_pointer(APPEND_CONTINUATION_ITEMS)?;
        parse_playlist_items(continuation_items)
    }
}

impl<T: GetWatchPlaylistQueryID> ParseFromContinuable<GetWatchPlaylistQuery<T>>
    for Vec<WatchPlaylistTrack>
{
    fn parse_from_continuable(
        p: ProcessedResult<GetWatchPlaylistQuery<T>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut playlist_panel =
            json_crawler.navigate_pointer(concatcp!(WATCH_NEXT_CONTENT, PPR))?;
        let continuation_params = playlist_panel
            .take_value_pointer(RADIO_CONTINUATION_PARAMS)
            .ok();
        let tracks = playlist_panel
            .navigate_pointer("/contents")?
            .try_into_iter()?
            .map(parse_watch_playlist_track)
            .collect::<Result<Vec<_>>>()?;
        Ok((tracks, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<crate::query::GetContinuationsQuery<'_, GetWatchPlaylistQuery<T>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut playlist_panel = json_crawler.navigate_pointer(PLAYLIST_PANEL_CONTINUATION)?;
        let continuation_params = playlist_panel
            .take_value_pointer(RADIO_CONTINUATION_PARAMS)
            .ok();
        let tracks = playlist_panel
            .navigate_pointer("/contents")?
            .try_into_iter()?
            .map(parse_watch_playlist_track)
            .collect::<Result<Vec<_>>>()?;
        Ok((tracks, continuation_params))
    }
}

fn parse_watch_playlist_track(mut item: impl JsonCrawler) -> Result<WatchPlaylistTrack> {
    let video_renderer_paths = [
        "/playlistPanelVideoRenderer",
        "/playlistPanelVideoWrapperRenderer/primaryRenderer/playlistPanelVideoRenderer",
    ];
    item.apply_function_at_paths(
        &video_renderer_paths,
        parse_watch_playlist_track_from_video_renderer,
    )?
}

fn parse_watch_playlist_track_from_video_renderer<C: JsonCrawler>(
    mut video_renderer: C::BorrowTo<'_>,
) -> Result<WatchPlaylistTrack> {
    let title = video_renderer.take_value_pointer(TITLE_TEXT)?;
    let author = video_renderer.take_value_pointer(concatcp!("/shortBylineText", RUN_TEXT))?;
    let duration = video_renderer.take_value_pointer(concatcp!("/lengthText", RUN_TEXT))?;
    let video_id = video_renderer.take_value_pointer(NAVIGATION_VIDEO_ID)?;
    let thumbnails = video_renderer.take_value_pointer(THUMBNAIL)?;
    Ok(WatchPlaylistTrack {
        title,
        author,
        duration,
        thumbnails,
        video_id,
    })
}

pub(crate) fn parse_playlist_song(
    title: String,
    track_no: usize,
    mut data: impl JsonCrawler,
) -> Result<PlaylistSong> {
    let video_id = data.take_value_pointer(concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        WATCH_VIDEO_ID
    ))?;
    let library_management =
        parse_library_management_items_from_menu(data.borrow_pointer(MENU_ITEMS)?)?;
    let like_status = data.take_value_pointer(MENU_LIKE_STATUS)?;
    let artists = super::parse_song_artists(&mut data, 1)?;
    // Some playlist types (Potentially just Featured Playlists) have a 'Plays'
    // field between Artist and Album.
    // TODO: Find a more efficient way, and potentially parse Featured Playlists
    // differently.
    let album_col_idx = if data.path_exists("/flexColumns/3") {
        3
    } else {
        2
    };
    let album = super::parse_song_album(&mut data, album_col_idx)?;
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
    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(PlaylistSong {
        video_id,
        track_no,
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
    })
}
pub(crate) fn parse_playlist_upload_song(
    title: String,
    track_no: usize,
    mut data: impl JsonCrawler,
) -> Result<PlaylistUploadSong> {
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
    Ok(PlaylistUploadSong {
        entity_id,
        video_id,
        album,
        duration,
        like_status,
        title,
        artists,
        thumbnails,
        track_no,
    })
}

pub(crate) fn parse_playlist_episode(
    title: String,
    track_no: usize,
    mut data: impl JsonCrawler,
) -> Result<PlaylistEpisode> {
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
            let duration =
                data.borrow_pointer(fixed_column_item_pointer(0))
                    .and_then(|mut i| {
                        i.take_value_pointer("/text/simpleText")
                            .or_else(|_| i.take_value_pointer("/text/runs/0/text"))
                    })?;
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
    Ok(PlaylistEpisode {
        episode_id: video_id,
        duration,
        title,
        like_status,
        thumbnails,
        date,
        podcast_name,
        podcast_id,
        is_available,
        track_no,
    })
}
pub(crate) fn parse_playlist_video(
    title: String,
    track_no: usize,
    mut data: impl JsonCrawler,
) -> Result<PlaylistVideo> {
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

    let playlist_id = data.take_value_pointer(concatcp!(
        MENU_ITEMS,
        "/0/menuNavigationItemRenderer",
        NAVIGATION_PLAYLIST_ID
    ))?;
    Ok(PlaylistVideo {
        video_id,
        track_no,
        duration,
        title,
        like_status,
        thumbnails,
        playlist_id,
        is_available,
        channel_name,
        channel_id,
    })
}

pub(crate) fn parse_playlist_item(
    track_no: usize,
    mut json: impl JsonCrawler,
) -> Result<Option<PlaylistItem>> {
    let Ok(mut data) = json.borrow_pointer(MRLIR) else {
        return Ok(None);
    };
    let title = super::parse_flex_column_item(&mut data, 0, 0)?;
    if title == "Song deleted" {
        return Ok(None);
    }
    // Handle not available case
    if let Ok("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") =
        data.take_value_pointer::<String>(DISPLAY_POLICY).as_deref()
    {
        return Ok(None);
    };
    let video_type_path = concatcp!(
        PLAY_BUTTON,
        "/playNavigationEndpoint",
        NAVIGATION_VIDEO_TYPE
    );
    let video_type: YoutubeMusicVideoType = data.take_value_pointer(video_type_path)?;
    let item = match video_type {
        YoutubeMusicVideoType::Ugc
        | YoutubeMusicVideoType::Omv
        | YoutubeMusicVideoType::Shoulder
        | YoutubeMusicVideoType::OfficialSourceMusic => Some(PlaylistItem::Video(
            parse_playlist_video(title, track_no, data)?,
        )),
        YoutubeMusicVideoType::Atv => Some(PlaylistItem::Song(parse_playlist_song(
            title, track_no, data,
        )?)),
        YoutubeMusicVideoType::Upload => Some(PlaylistItem::UploadSong(
            parse_playlist_upload_song(title, track_no, data)?,
        )),
        YoutubeMusicVideoType::Episode => Some(PlaylistItem::Episode(parse_playlist_episode(
            title, track_no, data,
        )?)),
    };
    Ok(item)
}
//TODO: Menu entries
pub(crate) fn parse_playlist_items<C>(
    json: C,
) -> Result<(Vec<PlaylistItem>, Option<ContinuationParams<'static>>)>
where
    C: JsonCrawler,
    C::IntoIter: DoubleEndedIterator,
{
    let mut items = json.try_into_iter()?;
    let mut last_item = items.next_back();
    let continuation_params = last_item.as_mut().and_then(|ref mut last_item| {
        last_item
            .take_value_pointer(CONTINUATION_RENDERER_COMMAND)
            .ok()
    });
    let items = items
        .chain(last_item)
        .enumerate()
        .filter_map(|(idx, item)| parse_playlist_item(idx + 1, item).transpose())
        .collect::<Result<_>>()?;
    Ok((items, continuation_params))
}

// NOTE: Similar code to get_album_2024
fn get_playlist_details(json_crawler: JsonCrawlerOwned) -> Result<GetPlaylistDetails> {
    let mut columns = json_crawler.navigate_pointer(TWO_COLUMN)?;
    let header =
        columns.borrow_pointer(concatcp!(TAB_CONTENT, SECTION_LIST_ITEM, RESPONSIVE_HEADER));
    // TODO: Utilise a crawler library function here.
    let mut header = match header {
        Ok(header) => header,
        Err(_) => columns.borrow_pointer(concatcp!(
            TAB_CONTENT,
            SECTION_LIST_ITEM,
            "/musicEditablePlaylistDetailHeaderRenderer/header",
            RESPONSIVE_HEADER
        ))?,
    };
    let title = header.take_value_pointer(TITLE_TEXT)?;
    // STRAPLINE_TEXT to be deprecated in future.
    let author = header.take_value_pointers(&[STRAPLINE_TEXT, FACEPILE_TEXT])?;
    let thumbnails: Vec<Thumbnail> = header.take_value_pointer(THUMBNAILS)?;
    let author_avatar_url: Option<String> = header.take_value_pointer(FACEPILE_AVATAR_URL).ok();
    let description = header
        .borrow_pointer(DESCRIPTION_SHELF_RUNS)
        .and_then(|d| d.try_into_iter())
        .ok()
        .map(|r| {
            r.map(|mut r| r.take_value_pointer::<String>("/text"))
                .collect::<std::result::Result<String, _>>()
        })
        .transpose()?;
    let mut subtitle = header.borrow_pointer("/subtitle/runs")?;
    let subtitle_len = subtitle.try_iter_mut()?.len();
    let privacy = if subtitle_len == 5 {
        Some(subtitle.take_value_pointer("/2/text")?)
    } else {
        None
    };
    let year = subtitle.take_value_pointer(format!("/{}/text", subtitle_len.saturating_sub(1)))?;
    let mut second_subtitle_runs = header.borrow_pointer(SECOND_SUBTITLE_RUNS)?;
    let duration = second_subtitle_runs
        .try_iter_mut()?
        .try_last()?
        .take_value_pointer("/text")?;
    let track_count_text = second_subtitle_runs.try_expect(
        "second subtitle runs should count at least 3 runs",
        |second_subtitle_runs| {
            second_subtitle_runs
                .try_iter_mut()?
                .rev()
                .nth(2)
                .map(|mut run| run.take_value_pointer("/text"))
                .transpose()
        },
    )?;
    let views = second_subtitle_runs
        .try_iter_mut()?
        .rev()
        .nth(4)
        .map(|mut item| item.take_value_pointer("/text"))
        .transpose()?;
    let id = header
        .navigate_pointer("/buttons")?
        .try_into_iter()?
        .find_path("/musicPlayButtonRenderer")?
        .take_value_pointer("/playNavigationEndpoint/watchEndpoint/playlistId")?;
    Ok(GetPlaylistDetails {
        id,
        privacy,
        title,
        description,
        author,
        year,
        duration,
        track_count_text,
        thumbnails,
        views,
        author_avatar_url,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{ApiOutcome, PlaylistID, VideoID, YoutubeID};
    use crate::query::playlist::GetPlaylistDetailsQuery;
    use crate::query::{
        AddPlaylistItemsQuery, EditPlaylistQuery, GetPlaylistTracksQuery, GetWatchPlaylistQuery,
    };
    use crate::{Error, process_json};
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[tokio::test]
    async fn test_add_playlist_items_query_failure() {
        let source_path = Path::new("./test_json/add_playlist_items_failure_20240626.json");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        // Blank query has no bearing on function
        let query = AddPlaylistItemsQuery::new_from_playlist(
            PlaylistID::from_raw(""),
            PlaylistID::from_raw(""),
        );
        let output = process_json::<_, BrowserToken>(source, query);
        let err: crate::Result<()> = Err(Error::status_failed());
        assert_eq!(format!("{:?}", err), format!("{:?}", output));
    }
    #[tokio::test]
    async fn test_add_playlist_items_query() {
        parse_test!(
            "./test_json/add_playlist_items_20240626.json",
            "./test_json/add_playlist_items_20240626_output.txt",
            AddPlaylistItemsQuery::new_from_playlist(
                PlaylistID::from_raw(""),
                PlaylistID::from_raw(""),
            ),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_edit_playlist_title_query() {
        parse_test_value!(
            "./test_json/edit_playlist_title_20240626.json",
            ApiOutcome::Success,
            EditPlaylistQuery::new_title(PlaylistID::from_raw(""), ""),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_playlist_details_query_2024() {
        parse_test!(
            "./test_json/get_playlist_20240624.json",
            "./test_json/get_playlist_details_20240624_output.txt",
            GetPlaylistDetailsQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    // In 2025, playlist channel details were moved from strapline to facepile.
    async fn test_get_playlist_details_query_2025() {
        parse_test!(
            "./test_json/get_playlist_20250604.json",
            "./test_json/get_playlist_details_20250604_output.txt",
            GetPlaylistDetailsQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_playlist_details_query_2024_no_channel_thumbnail() {
        parse_test!(
            "./test_json/get_playlist_no_channel_thumbnail_20240818.json",
            "./test_json/get_playlist_details_no_channel_thumbnail_20240818_output.txt",
            GetPlaylistDetailsQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_playlist_tracks_query() {
        parse_with_matching_continuation_test!(
            "./test_json/get_playlist_20250604.json",
            "./test_json/get_playlist_continuation_20250604.json",
            "./test_json/get_playlist_tracks_20250604_output.txt",
            GetPlaylistTracksQuery::new(PlaylistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_watch_playlist_query() {
        parse_with_matching_continuation_test!(
            "./test_json/get_watch_playlist_20250630.json",
            "./test_json/get_watch_playlist_continuation_20250630.json",
            "./test_json/get_watch_playlist_20250630_output.txt",
            GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw("")),
            BrowserToken
        );
    }
}
