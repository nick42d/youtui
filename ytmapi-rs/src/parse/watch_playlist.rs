use super::{ParseFrom, ProcessedResult};
use crate::common::{LyricsID, PlaylistID};
use crate::nav_consts::{NAVIGATION_PLAYLIST_ID, TAB_CONTENT};
use crate::query::watch_playlist::GetWatchPlaylistQueryID;
use crate::query::GetWatchPlaylistQuery;
use crate::Result;
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WatchPlaylist {
    // TODO: Implement tracks.
    /// Unimplemented!
    pub _tracks: Vec<()>,
    pub playlist_id: Option<PlaylistID<'static>>,
    pub lyrics_id: LyricsID<'static>,
}

impl<T: GetWatchPlaylistQueryID> ParseFrom<GetWatchPlaylistQuery<T>> for WatchPlaylist {
    fn parse_from(p: ProcessedResult<GetWatchPlaylistQuery<T>>) -> Result<Self> {
        // Should be a Process function not Parse.
        // XXX: Only used here!
        fn get_tab_browse_id<'a>(
            watch_next_renderer: &'a mut JsonCrawlerBorrowed,
            tab_id: usize,
        ) -> Result<JsonCrawlerBorrowed<'a>> {
            // TODO: Safe option that returns none if tab doesn't exist.
            let path = format!("/tabs/{tab_id}/tabRenderer/endpoint/browseEndpoint/browseId");
            watch_next_renderer.borrow_pointer(path).map_err(Into::into)
        }
        // TODO: Continuations
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut watch_next_renderer = json_crawler.navigate_pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer")?;
        let lyrics_id =
            get_tab_browse_id(&mut watch_next_renderer.borrow_mut(), 1)?.take_value()?;
        let mut results = watch_next_renderer.navigate_pointer(concatcp!(
            TAB_CONTENT,
            "/musicQueueRenderer/content/playlistPanelRenderer/contents"
        ))?;
        let playlist_id = results.try_iter_mut()?.find_map(|mut v| {
            v.take_value_pointer(concatcp!(
                "/playlistPanelVideoRenderer",
                NAVIGATION_PLAYLIST_ID
            ))
            .ok()
        });
        Ok(WatchPlaylist {
            _tracks: Vec::new(),
            playlist_id,
            lyrics_id,
        })
    }
}
