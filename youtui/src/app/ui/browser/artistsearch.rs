use super::shared_components::{BrowserSearchAction, FilterAction, SortAction};
use crate::app::component::actionhandler::{
    ActionHandler, ComponentEffect, KeyRouter, Scrollable, TextHandler, YoutuiEffect,
};
use crate::app::server::api::GetArtistSongsProgressUpdate;
use crate::app::server::{GetArtistSongs, HandleApiError, SearchArtists};
use crate::app::structures::SongListComponent;
use crate::app::ui::action::{AppAction, TextEntryAction};
use crate::app::ui::ListStatus;
use crate::app::view::{ListView, TableView};
use crate::app::AppCallback;
use crate::config::keymap::Keymap;
use crate::config::Config;
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use search_panel::{ArtistSearchPanel, BrowserArtistsAction};
use songs_panel::{AlbumSongsPanel, BrowserArtistSongsAction};
use std::mem;
use tracing::{error, warn};
use ytmapi_rs::common::{AlbumID, ArtistChannelID, Thumbnail};
use ytmapi_rs::parse::{AlbumSong, ParsedSongAlbum, ParsedSongArtist, SearchResultArtist};

pub mod search_panel;
pub mod songs_panel;

pub struct ArtistSearchBrowser {
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub artist_search_panel: ArtistSearchPanel,
    pub album_songs_panel: AlbumSongsPanel,
}
impl_youtui_component!(ArtistSearchBrowser);

#[derive(PartialEq, Default)]
pub enum InputRouting {
    #[default]
    Artist,
    Song,
}

impl InputRouting {
    pub fn left(&self) -> Self {
        match self {
            Self::Song => Self::Artist,
            Self::Artist => Self::Artist,
        }
    }
    pub fn right(&self) -> Self {
        match self {
            Self::Artist => Self::Song,
            Self::Song => Self::Song,
        }
    }
}

impl Scrollable for ArtistSearchBrowser {
    fn increment_list(&mut self, amount: isize) {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.increment_list(amount),
            InputRouting::Song => self.album_songs_panel.increment_list(amount),
        }
    }
    fn is_scrollable(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.is_scrollable(),
            InputRouting::Song => self.album_songs_panel.is_scrollable(),
        }
    }
}

impl TextHandler for ArtistSearchBrowser {
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.is_text_handling(),
            InputRouting::Song => self.album_songs_panel.is_text_handling(),
        }
    }
    fn get_text(&self) -> &str {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.get_text(),
            InputRouting::Song => self.album_songs_panel.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.replace_text(text),
            InputRouting::Song => self.album_songs_panel.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_search_panel.is_text_handling(),
            InputRouting::Song => self.album_songs_panel.is_text_handling(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.input_routing {
            InputRouting::Artist => self
                .artist_search_panel
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.artist_search_panel)),
            InputRouting::Song => self
                .album_songs_panel
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.album_songs_panel)),
        }
    }
}
impl ActionHandler<FilterAction> for ArtistSearchBrowser {
    fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            FilterAction::Close => self.album_songs_panel.toggle_filter(),
            FilterAction::Apply => self.album_songs_panel.apply_filter(),
            FilterAction::ClearFilter => self.album_songs_panel.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<SortAction> for ArtistSearchBrowser {
    fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            SortAction::SortSelectedAsc => self.album_songs_panel.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => self.album_songs_panel.handle_sort_cur_desc(),
            SortAction::Close => self.album_songs_panel.close_sort(),
            SortAction::ClearSort => self.album_songs_panel.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserArtistsAction> for ArtistSearchBrowser {
    fn apply_action(&mut self, action: BrowserArtistsAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserArtistsAction::DisplaySelectedArtistAlbums => self.get_songs(),
        }
    }
}
impl ActionHandler<BrowserSearchAction> for ArtistSearchBrowser {
    fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSearchAction::PrevSearchSuggestion => {
                self.artist_search_panel.search.increment_list(-1)
            }
            BrowserSearchAction::NextSearchSuggestion => {
                self.artist_search_panel.search.increment_list(1)
            }
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserArtistSongsAction> for ArtistSearchBrowser {
    fn apply_action(&mut self, action: BrowserArtistSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserArtistSongsAction::PlayAlbum => return self.play_album().into(),
            BrowserArtistSongsAction::PlaySong => return self.play_song().into(),
            BrowserArtistSongsAction::PlaySongs => return self.play_songs().into(),
            BrowserArtistSongsAction::AddAlbumToPlaylist => {
                return self.add_album_to_playlist().into()
            }
            BrowserArtistSongsAction::AddSongToPlaylist => {
                return self.add_song_to_playlist().into()
            }
            BrowserArtistSongsAction::AddSongsToPlaylist => {
                return self.add_songs_to_playlist().into()
            }
            BrowserArtistSongsAction::Sort => self.album_songs_panel.handle_pop_sort(),
            BrowserArtistSongsAction::Filter => self.album_songs_panel.toggle_filter(),
        }
        YoutuiEffect::new_no_op()
    }
}
impl KeyRouter<AppAction> for ArtistSearchBrowser {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        self.artist_search_panel
            .get_all_keybinds(config)
            .chain(self.album_songs_panel.get_all_keybinds(config))
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.input_routing {
            InputRouting::Artist => {
                Either::Left(self.artist_search_panel.get_active_keybinds(config))
            }
            InputRouting::Song => Either::Right(self.album_songs_panel.get_active_keybinds(config)),
        }
    }
}

impl ArtistSearchBrowser {
    pub fn new() -> Self {
        Self {
            input_routing: Default::default(),
            prev_input_routing: Default::default(),
            artist_search_panel: ArtistSearchPanel::new(),
            album_songs_panel: AlbumSongsPanel::new(),
        }
    }
    pub fn left(&mut self) {
        self.change_routing(self.input_routing.left());
    }
    pub fn right(&mut self) {
        self.change_routing(self.input_routing.right());
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        if self.is_text_handling()
            && self.artist_search_panel.search_popped
            && self.input_routing == InputRouting::Artist
        {
            match action {
                TextEntryAction::Submit => {
                    return self.search();
                }
                // Handled by old handle_text_event_impl.
                //
                // TODO: remove the duplication of responsibilities between this function and
                // handle_text_event_impl.
                TextEntryAction::Left => (),
                TextEntryAction::Right => (),
                TextEntryAction::Backspace => (),
            }
        }
        AsyncTask::new_no_op()
    }
    pub fn handle_toggle_search(&mut self) {
        if self.artist_search_panel.search_popped {
            self.artist_search_panel.close_search();
            self.revert_routing();
        } else {
            self.artist_search_panel.open_search();
            self.change_routing(InputRouting::Artist);
        }
    }
    pub fn search(&mut self) -> ComponentEffect<Self> {
        self.artist_search_panel.close_search();
        let search_query = self.artist_search_panel.search.get_text().to_string();
        self.artist_search_panel.clear_text();

        let handler = |this: &mut Self, results| match results {
            Ok(artists) => {
                this.replace_artist_list(artists);
                AsyncTask::new_no_op()
            }
            Err(error) => AsyncTask::new_future(
                HandleApiError {
                    error,
                    // To avoid needing to clone search query to use in the error message, this
                    // error message is minimal.
                    message: "Error recieved getting artists".to_string(),
                },
                |_, _| {},
                None,
            ),
        };
        AsyncTask::new_future_chained(
            SearchArtists(search_query),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn get_songs(&mut self) -> ComponentEffect<Self> {
        let selected = self.artist_search_panel.get_selected_item();
        self.change_routing(InputRouting::Song);
        self.album_songs_panel.list.clear();

        let Some(cur_artist_id) = self
            .artist_search_panel
            .list
            .get(selected)
            .cloned()
            .map(|a| a.browse_id)
        else {
            tracing::warn!("Tried to get item from list with index out of range");
            return AsyncTask::new_no_op();
        };
        let cur_artist_id_clone = cur_artist_id.clone();
        let handler = |this: &mut Self, item| {
            match item {
                GetArtistSongsProgressUpdate::Loading => this.handle_song_list_loading(),
                GetArtistSongsProgressUpdate::NoSongsFound => this.handle_no_songs_found(),
                GetArtistSongsProgressUpdate::GetArtistAlbumsError(e) => {
                    return this.handle_search_artist_error(cur_artist_id_clone, e)
                }
                GetArtistSongsProgressUpdate::GetAlbumsSongsError { album_id, error } => {
                    return this.handle_get_album_songs_error(cur_artist_id_clone, album_id, error)
                }
                GetArtistSongsProgressUpdate::SongsFound => this.handle_songs_found(),
                GetArtistSongsProgressUpdate::Songs {
                    song_list,
                    album,
                    year,
                    artists,
                    thumbnails,
                } => this.handle_append_song_list(song_list, album, year, artists, thumbnails),
                GetArtistSongsProgressUpdate::AllSongsSent => this.handle_song_list_loaded(),
            }
            AsyncTask::new_no_op()
        };

        AsyncTask::new_stream_chained(
            GetArtistSongs(cur_artist_id),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn play_song(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_song_idx = self.album_songs_panel.get_selected_item();
        if let Some(cur_song) = self.album_songs_panel.get_song_from_idx(cur_song_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylistAndPlay(vec![
                    cur_song.clone()
                ])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn play_songs(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_panel.get_selected_item();
        let song_list = self
            .album_songs_panel
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylistAndPlay(song_list)),
        )

        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub fn add_songs_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_panel.get_selected_item();
        let song_list = self
            .album_songs_panel
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylist(song_list)),
        )
    }
    pub fn add_song_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_panel.get_selected_item();
        if let Some(cur_song) = self.album_songs_panel.get_song_from_idx(cur_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylist(vec![cur_song.clone()])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn add_album_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_panel.get_selected_item();
        let Some(cur_song) = self.album_songs_panel.get_song_from_idx(cur_idx) else {
            return (AsyncTask::new_no_op(), None);
        };
        // Assert: If you're calling this function, all the songs in list must have an
        // album field!
        let Some(ref cur_album) = cur_song.album else {
            error!("Expected album details to be in list but they are missing!");
            return (AsyncTask::new_no_op(), None);
        };
        let song_list = self
            .album_songs_panel
            .list
            // Even if list is filtered, still play the whole album.
            .get_list_iter()
            .filter(|song| {
                song.album
                    .as_ref()
                    .is_some_and(|album| album.as_ref().id == cur_album.id)
            })
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylist(song_list)),
        )
    }
    pub fn play_album(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_panel.get_selected_item();
        let Some(cur_song) = self.album_songs_panel.get_song_from_idx(cur_idx) else {
            return (AsyncTask::new_no_op(), None);
        };
        // Assert: If you're calling this function, all the songs in list must have an
        // album field!
        let Some(ref cur_album) = cur_song.album else {
            error!("Expected album details to be in list but they are missing!");
            return (AsyncTask::new_no_op(), None);
        };
        let song_list = self
            .album_songs_panel
            .list
            // Even if list is filtered, still play the whole album.
            .get_list_iter()
            .filter(|song| {
                song.album
                    .as_ref()
                    .is_some_and(|album| album.as_ref().id == cur_album.id)
            })
            // XXX: Could instead be inside an Rc.
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylistAndPlay(song_list)),
        )

        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub fn handle_search_artist_error(
        &mut self,
        artist_id: ArtistChannelID<'static>,
        error: anyhow::Error,
    ) -> ComponentEffect<Self> {
        self.album_songs_panel.list.state = ListStatus::Error;
        AsyncTask::new_future(
            HandleApiError {
                error,
                message: format!("Error searching for artist {artist_id:?} albums"),
            },
            |_, _| {},
            None,
        )
    }
    // TODO: Handle this in the UI also.
    pub fn handle_get_album_songs_error(
        &mut self,
        artist_id: ArtistChannelID<'static>,
        album_id: AlbumID<'static>,
        error: anyhow::Error,
    ) -> ComponentEffect<Self> {
        warn!("Received a get_album_songs_error. This will be logged but is not visible in the main ui!");
        AsyncTask::new_future(
            HandleApiError {
                error,
                message: format!(
                    "Error getting songs for album {album_id:?}, artist {artist_id:?}"
                ),
            },
            |_, _| {},
            None,
        )
    }
    pub fn handle_song_list_loaded(&mut self) {
        self.album_songs_panel.list.state = ListStatus::Loaded;
    }
    pub fn handle_song_list_loading(&mut self) {
        self.album_songs_panel.list.state = ListStatus::Loading;
    }
    pub fn replace_artist_list(&mut self, artist_list: Vec<SearchResultArtist>) {
        self.artist_search_panel.list = artist_list;
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0);
    }
    pub fn handle_no_songs_found(&mut self) {
        self.album_songs_panel.list.state = ListStatus::Loaded;
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<AlbumSong>,
        album: ParsedSongAlbum,
        year: String,
        artists: Vec<ParsedSongArtist>,
        thumbnails: Vec<Thumbnail>,
    ) {
        self.album_songs_panel
            .list
            .append_raw_album_songs(song_list, album, year, artists, thumbnails);
        // If sort commands exist, sort the list.
        // Naive - can result in multiple calls to sort every time songs are appended.
        if let Err(e) = self.album_songs_panel.apply_all_sort_commands() {
            error!("Error <{e}> sorting album songs panel");
        }
        self.album_songs_panel.list.state = ListStatus::InProgress;
    }
    pub fn handle_songs_found(&mut self) {
        self.album_songs_panel.handle_songs_found()
    }
    fn increment_cur_list(&mut self, increment: isize) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_search_panel.increment_list(increment);
            }
            InputRouting::Song => {
                self.album_songs_panel.increment_list(increment);
            }
        };
    }
    pub fn revert_routing(&mut self) {
        mem::swap(&mut self.input_routing, &mut self.prev_input_routing);
    }
    pub fn change_routing(&mut self, input_routing: InputRouting) {
        self.prev_input_routing = mem::replace(&mut self.input_routing, input_routing);
    }
}
