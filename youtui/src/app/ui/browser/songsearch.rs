use super::{
    artistsearch::search_panel::BrowserSearchAction,
    shared_components::{FilterAction, FilterManager, SearchBlock, SortAction, SortManager},
};
use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, ComponentEffect, DominantKeyRouter, KeyRouter, Scrollable,
            TextHandler, YoutuiEffect,
        },
        server::{HandleApiError, SearchSongs},
        ui::action::{AppAction, TextEntryAction},
        AppCallback,
    },
    config::{keymap::Keymap, Config},
};
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use ytmapi_rs::parse::SearchResultSong;

const MAX_SONG_SEARCH_RESULTS: usize = 100;

pub struct SongSearchBrowser {
    pub input_routing: InputRouting,
    song_list: Vec<SearchResultSong>,
    search_popped: bool,
    search: SearchBlock,
    widget_state: TableState,
    pub sort: SortManager,
    pub filter: FilterManager,
    keybinds: Keymap<AppAction>,
}
impl_youtui_component!(SongSearchBrowser);

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    AddSongToPlaylist,
    AddSongsToPlaylist,
}

impl Action for BrowserSongsAction {
    fn context(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        todo!()
    }
}

#[derive(Default)]
enum InputRouting {
    Search,
    #[default]
    List,
}

impl Scrollable for SongSearchBrowser {
    fn increment_list(&mut self, amount: isize) {
        todo!()
    }
    fn is_scrollable(&self) -> bool {
        todo!()
    }
}
impl TextHandler for SongSearchBrowser {
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Search => todo!(),
            InputRouting::List => todo!(),
        }
    }
    fn get_text(&self) -> &str {
        match self.input_routing {
            InputRouting::Search => todo!(),
            InputRouting::List => todo!(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.input_routing {
            InputRouting::Search => todo!(),
            InputRouting::List => todo!(),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.input_routing {
            InputRouting::Search => todo!(),
            InputRouting::List => todo!(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.input_routing {
            InputRouting::Search => todo!(),
            InputRouting::List => todo!(),
        }
    }
}
impl ActionHandler<FilterAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            FilterAction::Close => self.album_songs_list.toggle_filter(),
            FilterAction::Apply => self.album_songs_list.apply_filter(),
            FilterAction::ClearFilter => self.album_songs_list.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<SortAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            SortAction::SortSelectedAsc => self.album_songs_list.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => self.album_songs_list.handle_sort_cur_desc(),
            SortAction::Close => self.album_songs_list.close_sort(),
            SortAction::ClearSort => self.album_songs_list.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserSearchAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        todo!()
    }
}
impl ActionHandler<BrowserSongsAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        todo!()
    }
}
impl KeyRouter<AppAction> for SongSearchBrowser {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        todo!()
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        if self.dominant_keybinds_active() {
            return Either::Left(self.get_dominant_keybinds());
        }
        todo!()
    }
}
impl DominantKeyRouter<AppAction> for SongSearchBrowser {
    fn dominant_keybinds_active(&self) -> bool {
        todo!()
    }
    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        todo!()
    }
}
impl SongSearchBrowser {
    pub fn new(config: &Config) -> Self {
        Self {
            input_routing: Default::default(),
            song_list: Default::default(),
            search_popped: Default::default(),
            search: Default::default(),
            widget_state: Default::default(),
            sort: Default::default(),
            filter: Default::default(),
            keybinds: Default::default(),
        }
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        if self.is_text_handling()
            && self.search_popped
            && matches!(self.input_routing, InputRouting::Search)
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
        if self.search_popped {
            self.search_popped = false;
            self.input_routing = InputRouting::List;
        } else {
            self.search_popped = true;
            self.input_routing = InputRouting::Search;
        }
    }
    pub fn search(&mut self) -> ComponentEffect<Self> {
        self.search_popped = false;
        self.input_routing = InputRouting::List;
        let search_query = self.search.get_text().to_string();
        self.search.clear_text();

        let handler = |this: &mut Self, results| match results {
            Ok(artists) => {
                this.replace_song_list(artists);
                AsyncTask::new_no_op()
            }
            Err(error) => AsyncTask::new_future(
                HandleApiError {
                    error,
                    // To avoid needing to clone search query to use in the error message, this
                    // error message is minimal.
                    message: "Error recieved searching songs".to_string(),
                },
                |_, _| {},
                None,
            ),
        };
        AsyncTask::new_future_chained(
            SearchSongs(search_query),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn play_song(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_song_idx = self.song_list.get_selected_item();
        if let Some(cur_song) = self.song_list.get_song_from_idx(cur_song_idx) {
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
        let cur_idx = self.album_songs_list.get_selected_item();
        let song_list = self
            .album_songs_list
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        (
            AsyncTask::new_no_op(),
            Some(AppCallback::AddSongsToPlaylistAndPlay(song_list)),
        )
    }
    pub fn add_songs_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        let song_list = self
            .album_songs_list
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
        let cur_idx = self.album_songs_list.get_selected_item();
        if let Some(cur_song) = self.album_songs_list.get_song_from_idx(cur_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylist(vec![cur_song.clone()])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn replace_song_list(&mut self, song_list: Vec<SearchResultSong>) {
        self.song_list = song_list;
        self.increment_list(0);
    }
}
