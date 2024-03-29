use const_format::concatcp;

pub const CONTENT: &str = "/contents/0";
pub const RUN_TEXT: &str = "/runs/0/text";
pub const TAB_CONTENT: &str = "/tabs/0/tabRenderer/content";
pub const _TAB_1_CONTENT: &str = "/tabs/1/tabRenderer/content";
pub const SINGLE_COLUMN: &str = "/contents/singleColumnBrowseResultsRenderer";
pub const SECTION_LIST: &str = "/sectionListRenderer/contents";
pub const MUSIC_SHELF: &str = "/musicShelfRenderer";
pub const MUSIC_CARD_SHELF: &str = "/musicCardShelfRenderer";
pub const GRID: &str = "/gridRenderer";
pub const MENU: &str = "/menu/menuRenderer";
pub const MENU_SERVICE: &str = "/menuServiceItemRenderer/serviceEndpoint";
pub const TOGGLE_MENU: &str = "/toggleMenuServiceItemRenderer";
pub const PLAY_BUTTON: &str =
    "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer";
pub const NAVIGATION_BROWSE: &str = "/navigationEndpoint/browseEndpoint";
pub const _PAGE_TYPE: &str =
    "/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType";
pub const _WATCH_VIDEO_ID: &str = "/watchEndpoint/videoId";
pub const NAVIGATION_WATCH_PLAYLIST_ID: &str =
    "/navigationEndpoint/watchPlaylistEndpoint/playlistId";
pub const NAVIGATION_VIDEO_TYPE: &str =
    "/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig,musicVideoType";
pub const TITLE: &str = "/title/runs/0";
pub const _TEXT_RUNS: &str = "/text/runs";
pub const SUBTITLE_RUNS: &str = "/subtitle/runs";
pub const THUMBNAIL: &str = "/thumbnail/thumbnails";
pub const FEEDBACK_TOKEN: &str = "/feedbackEndpoint/feedbackToken";
pub const BADGE_PATH: &str =
    "/0/musicInlineBadgeRenderer/accessibilityData/accessibilityData/label";
pub const LIVE_BADGE_PATH: &str = "/0/liveBadgeRenderer/accessibility/accessibilityData/label";
pub const _CATEGORY_PARAMS: &str =
    "/musicNavigationButtonRenderer/clickCommand/browseEndpoint/params";
pub const MRLIR: &str = "/musicResponsiveListItemRenderer";
pub const MTRIR: &str = "/musicTwoRowItemRenderer";
pub const _TASTE_PROFILE_ITEMS: &str = "/contents/tastebuilderRenderer/contents";
pub const _TASTE_PROFILE_ARTIST: &str = "/title/runs";
pub const _SECTION_LIST_CONTINUATION: &str = "/continuationContents/sectionListContinuation";
pub const HEADER_DETAIL: &str = "/header/musicDetailHeaderRenderer";
pub const DESCRIPTION_SHELF: &str = "/musicDescriptionShelfRenderer";
pub const _CAROUSEL: &str = "/musicCarouselShelfRenderer";
pub const _IMMERSIVE_CAROUSEL: &str = "/musicImmersiveCarouselShelfRenderer";
pub const _FRAMEWORK_MUTATIONS: &str = "/frameworkUpdates/entityBatchUpdate/mutations";
pub const TITLE_TEXT: &str = concatcp!("/title", RUN_TEXT);
pub const _NAVIGATION_VIDEO_ID: &str = concatcp!("/navigationEndpoint", _WATCH_VIDEO_ID);
pub const PLAYLIST_ITEM_VIDEO_ID: &str = "/playlistItemData/videoId";
pub const SINGLE_COLUMN_TAB: &str = concatcp!(SINGLE_COLUMN, TAB_CONTENT);
pub const SECTION_LIST_ITEM: &str = concatcp!("/sectionListRenderer", CONTENT);
pub const ITEM_SECTION: &str = concatcp!("/itemSectionRenderer", CONTENT);
pub const GRID_ITEMS: &str = concatcp!(GRID, "/items");
pub const MENU_ITEMS: &str = concatcp!(MENU, "/items");
pub const MENU_LIKE_STATUS: &str =
    concatcp!(MENU, "/topLevelButtons/0/likeButtonRenderer/likeStatus");
pub const NAVIGATION_BROWSE_ID: &str = concatcp!(NAVIGATION_BROWSE, "/browseId");
pub const NAVIGATION_PLAYLIST_ID: &str = concatcp!("/navigationEndpoint/watchEndpoint/playlistId");
pub const _TEXT_RUN: &str = concatcp!(_TEXT_RUNS, "/0");
pub const _TEXT_RUN_TEXT: &str = concatcp!(_TEXT_RUN, "/text");
pub const SUBTITLE: &str = concatcp!("/subtitle", RUN_TEXT);
pub const SUBTITLE2: &str = concatcp!(SUBTITLE_RUNS, "/2/text");
pub const _SUBTITLE3: &str = concatcp!(SUBTITLE_RUNS, "/4/text");
pub const THUMBNAILS: &str = concatcp!("/thumbnail/musicThumbnailRenderer", THUMBNAIL);
pub const THUMBNAIL_RENDERER: &str =
    concatcp!("/thumbnailRenderer/musicThumbnailRenderer", THUMBNAIL);
pub const THUMBNAIL_CROPPED: &str =
    concatcp!("/thumbnail/croppedSquareThumbnailRenderer", THUMBNAIL);
pub const BADGE_LABEL: &str = concatcp!("/badges", BADGE_PATH);
pub const LIVE_BADGE_LABEL: &str = concatcp!("/badges", LIVE_BADGE_PATH);
pub const SUBTITLE_BADGE_LABEL: &str = concatcp!("/subtitleBadges", BADGE_PATH);
pub const _CATEGORY_TITLE: &str = concatcp!("/musicNavigationButtonRenderer/buttonText", RUN_TEXT);
pub const MENU_PLAYLIST_ID: &str = concatcp!(
    MENU_ITEMS,
    "/0/menuNavigationItemRenderer",
    NAVIGATION_WATCH_PLAYLIST_ID
);
pub const DESCRIPTION: &str = concatcp!("/description", RUN_TEXT);
pub const _CAROUSEL_CONTENTS: &str = concatcp!(_CAROUSEL, "/contents");
pub const CAROUSEL_TITLE: &str = concatcp!("/header/musicCarouselShelfBasicHeaderRenderer", TITLE);
pub const _CARD_SHELF_TITLE: &str =
    concatcp!("/header/musicCardShelfHeaderBasicRenderer", TITLE_TEXT);
