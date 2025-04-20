use super::shared_components::{
    get_adjusted_list_column, BrowserSearchAction, FilterAction, FilterManager, SearchBlock,
    SortAction, SortManager,
};
use crate::app::structures::SongListComponent;
use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, ComponentEffect, DominantKeyRouter, KeyRouter, Scrollable,
            TextHandler, YoutuiEffect,
        },
        server::{HandleApiError, SearchSongs},
        structures::{AlbumSongsList, ListSong},
        ui::action::{AppAction, TextEntryAction},
        view::{
            Filter, FilterString, Loadable, SortDirection, SortableTableView, TableFilterCommand,
            TableSortCommand, TableView,
        },
        AppCallback,
    },
    config::{keymap::Keymap, Config},
};
use anyhow::{bail, Result};
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use tracing::warn;
use ytmapi_rs::parse::SearchResultSong;

const MAX_SONG_SEARCH_RESULTS: usize = 100;

pub struct SongSearchBrowser {
    pub input_routing: InputRouting,
    song_list: AlbumSongsList,
    cur_selected: usize,
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
pub enum InputRouting {
    List,
    #[default]
    Search,
    Filter,
    Sort,
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
            InputRouting::Filter => true,
            InputRouting::Search => true,
            InputRouting::List => false,
            InputRouting::Sort => false,
        }
    }
    fn get_text(&self) -> &str {
        match self.input_routing {
            InputRouting::Filter => self.filter.get_text(),
            InputRouting::Search => self.search.get_text(),
            InputRouting::List => "",
            InputRouting::Sort => "",
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.input_routing {
            InputRouting::Search => self.search.replace_text(text),
            InputRouting::Filter => self.filter.replace_text(text),
            InputRouting::List => (),
            InputRouting::Sort => (),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.input_routing {
            InputRouting::Search => self.search.clear_text(),
            InputRouting::Filter => self.filter.clear_text(),
            InputRouting::List => false,
            InputRouting::Sort => false,
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.input_routing {
            InputRouting::Search => self
                .search
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.search)),
            InputRouting::Filter => self
                .filter
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.filter)),
            InputRouting::List => None,
            InputRouting::Sort => None,
        }
    }
}
impl ActionHandler<FilterAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            FilterAction::Close => self.toggle_filter(),
            FilterAction::Apply => self.apply_filter(),
            FilterAction::ClearFilter => self.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<SortAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            SortAction::SortSelectedAsc => self.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => self.handle_sort_cur_desc(),
            SortAction::Close => self.close_sort(),
            SortAction::ClearSort => self.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserSearchAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSearchAction::SearchArtist => todo!(),
            BrowserSearchAction::PrevSearchSuggestion => todo!(),
            BrowserSearchAction::NextSearchSuggestion => todo!(),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserSongsAction> for SongSearchBrowser {
    async fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSongsAction::Filter => todo!(),
            BrowserSongsAction::Sort => todo!(),
            BrowserSongsAction::PlaySong => todo!(),
            BrowserSongsAction::PlaySongs => todo!(),
            BrowserSongsAction::AddSongToPlaylist => todo!(),
            BrowserSongsAction::AddSongsToPlaylist => todo!(),
        }
        YoutuiEffect::new_no_op()
    }
}
impl KeyRouter<AppAction> for SongSearchBrowser {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        std::iter::once(&self.keybinds)
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        match self.input_routing {
            InputRouting::List => std::iter::once(&self.keybinds),
            InputRouting::Search => todo!(),
            InputRouting::Filter => todo!(),
            InputRouting::Sort => todo!(),
        }
    }
}
impl SongListComponent for SongSearchBrowser {
    fn get_song_from_idx(&self, idx: usize) -> Option<&crate::app::structures::ListSong> {
        todo!()
    }
}
impl Loadable for SongSearchBrowser {
    fn is_loading(&self) -> bool {
        todo!()
    }
}
impl TableView for SongSearchBrowser {
    fn get_selected_item(&self) -> usize {
        todo!()
    }
    fn get_state(&self) -> TableState {
        todo!()
    }
    fn get_title(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    fn get_layout(&self) -> &[crate::app::view::BasicConstraint] {
        todo!()
    }
    fn get_highlighted_row(&self) -> Option<usize> {
        todo!()
    }
    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = crate::app::view::TableItem> + '_> {
        todo!()
    }
    fn get_headings(&self) -> Box<dyn Iterator<Item = &'static str>> {
        todo!()
    }
}
impl SortableTableView for SongSearchBrowser {
    fn get_sortable_columns(&self) -> &[usize] {
        &[]
    }
    fn get_sort_commands(&self) -> &[TableSortCommand] {
        &self.sort.sort_commands
    }
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()> {
        // TODO: Maintain a view only struct, for easier rendering of this.
        if !self.get_sortable_columns().contains(&sort_command.column) {
            bail!(format!("Unable to sort column {}", sort_command.column,));
        }
        // Map the column of ArtistAlbums to a column of List and sort
        self.song_list.sort(
            get_adjusted_list_column(sort_command.column, Self::subcolumns_of_vec())?,
            sort_command.direction,
        );
        // Remove commands that already exist for the same column, as this new command
        // will trump the old ones. Slightly naive - loops the whole vec, could
        // short circuit.
        self.sort
            .sort_commands
            .retain(|cmd| cmd.column != sort_command.column);
        self.sort.sort_commands.push(sort_command);
        Ok(())
    }
    fn clear_sort_commands(&mut self) {
        todo!()
    }
    fn get_filterable_columns(&self) -> &[usize] {
        todo!()
    }
    fn get_filtered_items(&self) -> Box<dyn Iterator<Item = crate::app::view::TableItem> + '_> {
        todo!()
    }
    fn get_filter_commands(&self) -> &[TableFilterCommand] {
        todo!()
    }
    fn push_filter_command(&mut self, filter_command: TableFilterCommand) {
        todo!()
    }
    fn clear_filter_commands(&mut self) {
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
            cur_selected: Default::default(),
        }
    }
    pub fn subcolumns_of_vec() -> &'static [usize] {
        todo!();
    }
    pub fn apply_sort_commands(&mut self) -> Result<()> {
        for c in self.sort.sort_commands.iter() {
            if !self.get_sortable_columns().contains(&c.column) {
                bail!(format!("Unable to sort column {}", c.column,));
            }
            self.song_list.sort(
                get_adjusted_list_column(c.column, Self::subcolumns_of_vec())?,
                c.direction,
            );
        }
        Ok(())
    }
    pub fn get_filtered_list_iter(&self) -> Box<dyn Iterator<Item = &ListSong> + '_> {
        let mapped_filterable_cols: Vec<_> = self
            .get_filterable_columns()
            .iter()
            .map(|c| Self::subcolumns_of_vec().get(*c))
            .collect();
        Box::new(self.song_list.get_list_iter().filter(move |ls| {
            // Naive implementation.
            // TODO: Do this in a single pass and optimise.
            self.get_filter_commands().iter().fold(true, |acc, e| {
                let match_found = match e {
                    TableFilterCommand::All(f) => {
                        let mut filterable_cols_iter =
                            ls.get_fields_iter().enumerate().filter_map(|(i, f)| {
                                if mapped_filterable_cols.contains(&Some(&i)) {
                                    Some(f)
                                } else {
                                    None
                                }
                            });
                        match f {
                            Filter::Contains(s) => filterable_cols_iter.any(|item| s.is_in(item)),
                            Filter::NotContains(_) => todo!(),
                            Filter::Equal(_) => todo!(),
                        }
                    }
                    TableFilterCommand::Column { .. } => todo!(),
                };
                // If we find a match for each filter, can display the row.
                acc && match_found
            })
        }))
    }
    pub fn apply_filter(&mut self) {
        let filter = self.filter.get_text().to_string();
        self.filter.shown = false;
        self.input_routing = InputRouting::List;
        let cmd = TableFilterCommand::All(crate::app::view::Filter::Contains(
            FilterString::CaseInsensitive(filter),
        ));
        self.filter.filter_commands.push(cmd);
        // Need to match current selected row to length of list.
        // Naive method to count the iterator. Consider making iterator exact sized...
        self.cur_selected = self
            .cur_selected
            .min(self.get_filtered_items().count().saturating_sub(1))
    }
    pub fn clear_filter(&mut self) {
        self.filter.shown = false;
        self.input_routing = InputRouting::List;
        self.filter.filter_commands.clear();
    }
    fn open_sort(&mut self) {
        self.sort.shown = true;
        self.input_routing = InputRouting::Sort;
    }
    pub fn toggle_filter(&mut self) {
        let shown = self.filter.shown;
        if !shown {
            // We need to set cur back to 0  and clear text somewhere and I'd prefer to do
            // it at the time of showing, so it cannot be missed.
            self.filter.filter_text.clear();
            self.input_routing = InputRouting::Filter;
        } else {
            self.input_routing = InputRouting::List;
        }
        self.filter.shown = !shown;
    }
    pub fn close_sort(&mut self) {
        self.sort.shown = false;
        self.input_routing = InputRouting::List;
    }
    pub fn handle_pop_sort(&mut self) {
        // If no sortable columns, should we not handle this command?
        self.sort.cur = 0;
        self.open_sort();
    }
    pub fn handle_clear_sort(&mut self) {
        self.close_sort();
        self.clear_sort_commands();
    }
    pub fn handle_sort_cur_asc(&mut self) {
        // TODO: Better error handling
        let Some(column) = self.get_sortable_columns().get(self.sort.cur) else {
            warn!("Tried to index sortable columns but was out of range");
            return;
        };
        if let Err(e) = self.push_sort_command(TableSortCommand {
            column: *column,
            direction: SortDirection::Asc,
        }) {
            warn!("Tried to sort a column that is not sortable - error {e}")
        };
        self.close_sort();
    }
    pub fn handle_sort_cur_desc(&mut self) {
        // TODO: Better error handling
        let Some(column) = self.get_sortable_columns().get(self.sort.cur) else {
            warn!("Tried to index sortable columns but was out of range");
            return;
        };
        if let Err(e) = self.push_sort_command(TableSortCommand {
            column: *column,
            direction: SortDirection::Desc,
        }) {
            warn!("Tried to sort a column that is not sortable - error {e}")
        };
        self.close_sort();
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
            Ok(songs) => {
                this.replace_song_list(songs);
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
        let cur_song_idx = self.get_selected_item();
        if let Some(cur_song) = self.get_song_from_idx(cur_song_idx) {
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
        let cur_idx = self.get_selected_item();
        let song_list = self
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
        let cur_idx = self.get_selected_item();
        let song_list = self
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
        let cur_idx = self.get_selected_item();
        if let Some(cur_song) = self.get_song_from_idx(cur_idx) {
            return (
                AsyncTask::new_no_op(),
                Some(AppCallback::AddSongsToPlaylist(vec![cur_song.clone()])),
            );
        }
        (AsyncTask::new_no_op(), None)
    }
    pub fn replace_song_list(&mut self, song_list: Vec<SearchResultSong>) {
        self.song_list.append_raw_search_result_songs(song_list);
        self.increment_list(0);
    }
}
