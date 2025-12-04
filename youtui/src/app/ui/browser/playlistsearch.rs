use super::shared_components::{BrowserSearchAction, FilterAction, SortAction};
use crate::app::AppCallback;
use crate::app::component::actionhandler::{
    ActionHandler, ComponentEffect, KeyRouter, Scrollable, TextHandler, YoutuiEffect,
};
use crate::app::server::api::{GetArtistSongsProgressUpdate, GetPlaylistSongsProgressUpdate};
use crate::app::server::{
    GetArtistSongs, GetPlaylistSongs, HandleApiError, SearchArtists, SearchPlaylists,
};
use crate::app::structures::SongListComponent;
use crate::app::ui::ListStatus;
use crate::app::ui::action::{AppAction, TextEntryAction};
use crate::app::ui::browser::playlistsearch::search_panel::{
    BrowserPlaylistsAction, NonPodcastSearchResultPlaylist, PlaylistSearchPanel,
};
use crate::app::ui::browser::playlistsearch::songs_panel::{
    BrowserPlaylistSongsAction, PlaylistSongsPanel,
};
use crate::app::view::{ListView, TableView};
use crate::config::Config;
use crate::config::keymap::Keymap;
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use std::mem;
use tracing::{error, warn};
use ytmapi_rs::common::{AlbumID, ArtistChannelID, PlaylistID, Thumbnail, YoutubeID};
use ytmapi_rs::parse::{
    AlbumSong, ParsedSongAlbum, ParsedSongArtist, PlaylistItem, SearchResultArtist,
    SearchResultPlaylist,
};

const MAX_PLAYLIST_SONGS: usize = 1000;

pub mod search_panel;
pub mod songs_panel;

pub struct PlaylistSearchBrowser {
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub playlist_search_panel: PlaylistSearchPanel,
    pub playlist_songs_panel: PlaylistSongsPanel,
}
impl_youtui_component!(PlaylistSearchBrowser);

#[derive(PartialEq, Default)]
pub enum InputRouting {
    #[default]
    Playlist,
    Song,
}

impl InputRouting {
    pub fn left(&self) -> Self {
        Self::Playlist
    }
    pub fn right(&self) -> Self {
        Self::Song
    }
}

impl Scrollable for PlaylistSearchBrowser {
    fn increment_list(&mut self, amount: isize) {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.increment_list(amount),
            InputRouting::Song => self.playlist_songs_panel.increment_list(amount),
        }
    }
    fn is_scrollable(&self) -> bool {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.is_scrollable(),
            InputRouting::Song => self.playlist_songs_panel.is_scrollable(),
        }
    }
}

impl TextHandler for PlaylistSearchBrowser {
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.is_text_handling(),
            InputRouting::Song => self.playlist_songs_panel.is_text_handling(),
        }
    }
    fn get_text(&self) -> &str {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.get_text(),
            InputRouting::Song => self.playlist_songs_panel.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.replace_text(text),
            InputRouting::Song => self.playlist_songs_panel.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.input_routing {
            InputRouting::Playlist => self.playlist_search_panel.is_text_handling(),
            InputRouting::Song => self.playlist_songs_panel.is_text_handling(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.input_routing {
            InputRouting::Playlist => self
                .playlist_search_panel
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.playlist_search_panel)),
            InputRouting::Song => self
                .playlist_songs_panel
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.playlist_songs_panel)),
        }
    }
}
impl ActionHandler<FilterAction> for PlaylistSearchBrowser {
    fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            FilterAction::Close => self.playlist_songs_panel.toggle_filter(),
            FilterAction::Apply => self.playlist_songs_panel.apply_filter(),
            FilterAction::ClearFilter => self.playlist_songs_panel.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<SortAction> for PlaylistSearchBrowser {
    fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            SortAction::SortSelectedAsc => self.playlist_songs_panel.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => self.playlist_songs_panel.handle_sort_cur_desc(),
            SortAction::Close => self.playlist_songs_panel.close_sort(),
            SortAction::ClearSort => self.playlist_songs_panel.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserPlaylistsAction> for PlaylistSearchBrowser {
    fn apply_action(&mut self, action: BrowserPlaylistsAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserPlaylistsAction::DisplaySelectedPlaylist => self.get_songs(),
        }
    }
}
impl ActionHandler<BrowserSearchAction> for PlaylistSearchBrowser {
    fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSearchAction::PrevSearchSuggestion => {
                self.playlist_search_panel.search.increment_list(-1)
            }
            BrowserSearchAction::NextSearchSuggestion => {
                self.playlist_search_panel.search.increment_list(1)
            }
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserPlaylistSongsAction> for PlaylistSearchBrowser {
    fn apply_action(
        &mut self,
        action: BrowserPlaylistSongsAction,
    ) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserPlaylistSongsAction::PlaySong => return self.play_song().into(),
            BrowserPlaylistSongsAction::PlaySongs => return self.play_songs().into(),
            BrowserPlaylistSongsAction::AddSongToPlaylist => {
                return self.add_song_to_playlist().into();
            }
            BrowserPlaylistSongsAction::AddSongsToPlaylist => {
                return self.add_songs_to_playlist().into();
            }
            BrowserPlaylistSongsAction::Sort => self.playlist_songs_panel.handle_pop_sort(),
            BrowserPlaylistSongsAction::Filter => self.playlist_songs_panel.toggle_filter(),
        }
        YoutuiEffect::new_no_op()
    }
}
impl KeyRouter<AppAction> for PlaylistSearchBrowser {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        self.playlist_search_panel
            .get_all_keybinds(config)
            .chain(self.playlist_songs_panel.get_all_keybinds(config))
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.input_routing {
            InputRouting::Playlist => {
                Either::Left(self.playlist_search_panel.get_active_keybinds(config))
            }
            InputRouting::Song => {
                Either::Right(self.playlist_songs_panel.get_active_keybinds(config))
            }
        }
    }
}

impl PlaylistSearchBrowser {
    pub fn new() -> Self {
        Self {
            input_routing: Default::default(),
            prev_input_routing: Default::default(),
            playlist_search_panel: PlaylistSearchPanel::new(),
            playlist_songs_panel: PlaylistSongsPanel::new(),
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
            && self.playlist_search_panel.search_popped
            && self.input_routing == InputRouting::Playlist
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
        if self.playlist_search_panel.search_popped {
            self.playlist_search_panel.close_search();
            self.revert_routing();
        } else {
            self.playlist_search_panel.open_search();
            self.change_routing(InputRouting::Playlist);
        }
    }
    pub fn search(&mut self) -> ComponentEffect<Self> {
        self.playlist_search_panel.close_search();
        let search_query = self.playlist_search_panel.search.get_text().to_string();
        self.playlist_search_panel.clear_text();

        let handler = |this: &mut Self, results| match results {
            Ok(artists) => {
                this.replace_playlist_list(artists);
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
            SearchPlaylists(search_query),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn get_songs(&mut self) -> ComponentEffect<Self> {
        let selected = self.playlist_search_panel.get_selected_item();
        self.change_routing(InputRouting::Song);
        self.playlist_songs_panel.list.clear();

        let Some(cur_playlist_id) = self
            .playlist_search_panel
            .list
            .get(selected)
            .cloned()
            .map(|a| a.playlist_id)
        else {
            tracing::warn!("Tried to get item from list with index out of range");
            return AsyncTask::new_no_op();
        };

        let cur_playlist_id_clone = cur_playlist_id.clone();
        let handler = |this: &mut Self, item| {
            match item {
                GetPlaylistSongsProgressUpdate::Loading => this.handle_song_list_loading(),
                GetPlaylistSongsProgressUpdate::Songs(playlist_items) => {
                    this.handle_append_song_list(playlist_items)
                }
                GetPlaylistSongsProgressUpdate::GetPlaylistSongsError(e) => {
                    return this.handle_search_playlist_error(cur_playlist_id_clone, e);
                }
                GetPlaylistSongsProgressUpdate::AllSongsSent => this.handle_song_list_loaded(),
                GetPlaylistSongsProgressUpdate::NoSongsFound => this.handle_no_songs_found(),
            }
            AsyncTask::new_no_op()
        };

        AsyncTask::new_stream_chained(
            GetPlaylistSongs {
                playlist_id: cur_playlist_id,
                max_songs: MAX_PLAYLIST_SONGS,
            },
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn play_song(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_song_idx = self.playlist_songs_panel.get_selected_item();
        if let Some(cur_song) = self.playlist_songs_panel.get_song_from_idx(cur_song_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylistAndPlay(vec![
                    cur_song.clone(),
                ])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn play_songs(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.playlist_songs_panel.get_selected_item();
        let song_list = self
            .playlist_songs_panel
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
    pub fn add_songs_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.playlist_songs_panel.get_selected_item();
        let song_list = self
            .playlist_songs_panel
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylist(song_list)),
        )
    }
    pub fn add_song_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.playlist_songs_panel.get_selected_item();
        if let Some(cur_song) = self.playlist_songs_panel.get_song_from_idx(cur_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylist(vec![cur_song.clone()])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn handle_search_playlist_error(
        &mut self,
        playlist_id: PlaylistID<'static>,
        error: anyhow::Error,
    ) -> ComponentEffect<Self> {
        self.playlist_songs_panel.list.state = ListStatus::Error;
        AsyncTask::new_future(
            HandleApiError {
                error,
                message: format!("Error searching for playlist {playlist_id:?} tracks"),
            },
            |_, _| {},
            None,
        )
    }
    pub fn handle_song_list_loaded(&mut self) {
        self.playlist_songs_panel.list.state = ListStatus::Loaded;
    }
    pub fn handle_song_list_loading(&mut self) {
        self.playlist_songs_panel.list.state = ListStatus::Loading;
    }
    pub fn replace_playlist_list(&mut self, playlist_list: Vec<SearchResultPlaylist>) {
        // TODO: See if allocation can be removed.
        self.playlist_search_panel.list = playlist_list
            .into_iter()
            .filter_map(NonPodcastSearchResultPlaylist::new)
            .collect();
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0);
    }
    pub fn handle_no_songs_found(&mut self) {
        self.playlist_songs_panel.list.state = ListStatus::Loaded;
    }
    pub fn handle_append_song_list(&mut self, song_list: Vec<PlaylistItem>) {
        self.playlist_songs_panel
            .list
            .append_raw_playlist_items(song_list);
        // If sort commands exist, sort the list.
        // Naive - can result in multiple calls to sort every time songs are appended.
        if let Err(e) = self.playlist_songs_panel.apply_all_sort_commands() {
            error!("Error <{e}> sorting album songs panel");
        }
        self.playlist_songs_panel.list.state = ListStatus::InProgress;
    }
    fn increment_cur_list(&mut self, increment: isize) {
        match self.input_routing {
            InputRouting::Playlist => {
                self.playlist_search_panel.increment_list(increment);
            }
            InputRouting::Song => {
                self.playlist_songs_panel.increment_list(increment);
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
