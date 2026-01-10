use super::get_sort_keybinds;
use super::shared_components::{
    BrowserSearchAction, FilterAction, FilterManager, SearchBlock, SortAction, SortManager,
    get_adjusted_list_column,
};
use crate::app::AppCallback;
use crate::app::component::actionhandler::{
    Action, ActionHandler, ComponentEffect, KeyRouter, Scrollable, Suggestable, TextHandler,
    YoutuiEffect,
};
use crate::app::server::{HandleApiError, SearchSongs};
use crate::app::structures::{
    BrowserSongsList, ListSong, ListSongDisplayableField, ListStatus, Percentage, SongListComponent,
};
use crate::app::ui::action::{AppAction, TextEntryAction};
use crate::app::view::{
    AdvancedTableView, BasicConstraint, FilterString, HasTitle, Loadable, SortDirection,
    TableFilterCommand, TableSortCommand, TableView,
};
use crate::config::Config;
use crate::config::keymap::Keymap;
use crate::drawutils::get_offset_after_list_resize;
use crate::widgets::ScrollingTableState;
use anyhow::{Result, bail};
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tracing::warn;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::parse::SearchResultSong;

pub struct SongSearchBrowser {
    pub input_routing: InputRouting,
    song_list: BrowserSongsList,
    cur_selected: usize,
    pub search_popped: bool,
    pub search: SearchBlock,
    pub widget_state: ScrollingTableState,
    pub sort: SortManager,
    pub filter: FilterManager,
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
    fn context(&self) -> std::borrow::Cow<'_, str> {
        "Song Search Browser".into()
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            BrowserSongsAction::Filter => "Filter",
            BrowserSongsAction::Sort => "Sort",
            BrowserSongsAction::PlaySong => "Play song",
            BrowserSongsAction::PlaySongs => "Play songs",
            BrowserSongsAction::AddSongToPlaylist => "Add song to playlist",
            BrowserSongsAction::AddSongsToPlaylist => "Add songs to playlist",
        }
        .into()
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

impl Suggestable for SongSearchBrowser {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        self.search.get_search_suggestions()
    }
    fn has_search_suggestions(&self) -> bool {
        self.search.has_search_suggestions()
    }
}

impl Scrollable for SongSearchBrowser {
    fn increment_list(&mut self, amount: isize) {
        match self.input_routing {
            InputRouting::List => {
                // Naive check using iterator - consider using exact size iterator
                self.cur_selected = self
                    .cur_selected
                    .saturating_add_signed(amount)
                    .min(self.get_filtered_items().count().saturating_sub(1))
            }
            InputRouting::Sort => {
                self.sort.cur = self
                    .sort
                    .cur
                    .saturating_add_signed(amount)
                    .min(self.get_sortable_columns().len().saturating_sub(1));
            }
            InputRouting::Search => warn!("Tried to increment list when in search box"),
            InputRouting::Filter => warn!("Tried to increment list when filter popup shown"),
        }
    }
    fn is_scrollable(&self) -> bool {
        matches!(self.input_routing, InputRouting::Sort | InputRouting::List)
    }
}
impl TextHandler for SongSearchBrowser {
    fn is_text_handling(&self) -> bool {
        matches!(
            self.input_routing,
            InputRouting::Filter | InputRouting::Search
        )
    }
    fn get_text(&self) -> std::option::Option<&str> {
        match self.input_routing {
            InputRouting::Filter => self.filter.get_text(),
            InputRouting::Search => self.search.get_text(),
            InputRouting::List | InputRouting::Sort => None,
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
                .map(|effect| effect.map_frontend(|this: &mut Self| &mut this.search)),
            InputRouting::Filter => self
                .filter
                .handle_text_event_impl(event)
                .map(|effect| effect.map_frontend(|this: &mut Self| &mut this.filter)),
            InputRouting::List => None,
            InputRouting::Sort => None,
        }
    }
}
impl ActionHandler<FilterAction> for SongSearchBrowser {
    fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            FilterAction::Close => self.toggle_filter(),
            FilterAction::Apply => self.apply_filter(),
            FilterAction::ClearFilter => self.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<SortAction> for SongSearchBrowser {
    fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
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
    fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSearchAction::PrevSearchSuggestion => self.search.increment_list(-1),
            BrowserSearchAction::NextSearchSuggestion => self.search.increment_list(1),
        }
        AsyncTask::new_no_op()
    }
}
impl ActionHandler<BrowserSongsAction> for SongSearchBrowser {
    fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserSongsAction::Filter => self.toggle_filter(),
            BrowserSongsAction::Sort => self.handle_pop_sort(),
            BrowserSongsAction::PlaySong => return self.play_song().into(),
            BrowserSongsAction::PlaySongs => return self.play_songs().into(),
            BrowserSongsAction::AddSongToPlaylist => return self.add_song_to_playlist().into(),
            BrowserSongsAction::AddSongsToPlaylist => return self.add_songs_to_playlist().into(),
        }
        YoutuiEffect::new_no_op()
    }
}
impl KeyRouter<AppAction> for SongSearchBrowser {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        [
            &config.keybinds.browser_songs,
            &config.keybinds.browser_search,
        ]
        .into_iter()
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.input_routing {
            InputRouting::List => Either::Left(std::iter::once(&config.keybinds.browser_songs)),
            InputRouting::Search => Either::Left(std::iter::once(&config.keybinds.browser_search)),
            InputRouting::Filter => Either::Left(std::iter::once(&config.keybinds.filter)),
            InputRouting::Sort => Either::Right(get_sort_keybinds(config)),
        }
    }
}
impl SongListComponent for SongSearchBrowser {
    fn get_song_from_idx(&self, idx: usize) -> Option<&crate::app::structures::ListSong> {
        self.get_filtered_list_iter().nth(idx)
    }
}
impl Loadable for SongSearchBrowser {
    fn is_loading(&self) -> bool {
        matches!(
            self.song_list.state,
            crate::app::structures::ListStatus::Loading
        )
    }
}
impl TableView for SongSearchBrowser {
    fn get_selected_item(&self) -> usize {
        self.cur_selected
    }
    fn get_state(&self) -> &ScrollingTableState {
        &self.widget_state
    }
    fn get_layout(&self) -> &[crate::app::view::BasicConstraint] {
        &[
            BasicConstraint::Percentage(Percentage(40)),
            BasicConstraint::Percentage(Percentage(30)),
            BasicConstraint::Percentage(Percentage(30)),
            BasicConstraint::Length(8),
            BasicConstraint::Length(10),
        ]
    }
    fn get_highlighted_row(&self) -> Option<usize> {
        None
    }
    fn get_items(&self) -> impl ExactSizeIterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> {
        self.song_list
            .get_list_iter()
            .map(|ls| ls.get_fields(Self::subcolumns_of_vec()).into_iter())
    }
    fn get_headings(&self) -> impl Iterator<Item = &'static str> {
        ["Song", "Artist", "Album", "Duration", "Plays"].into_iter()
    }
    fn get_mut_state(&mut self) -> &mut ScrollingTableState {
        &mut self.widget_state
    }
}
impl AdvancedTableView for SongSearchBrowser {
    fn get_sortable_columns(&self) -> &[usize] {
        &[0, 1, 2]
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
        self.sort.sort_commands.clear();
    }
    fn get_filter_commands(&self) -> &[TableFilterCommand] {
        &self.filter.filter_commands
    }
    fn push_filter_command(&mut self, filter_command: TableFilterCommand) {
        self.filter.filter_commands.push(filter_command)
    }
    fn clear_filter_commands(&mut self) {
        self.filter.filter_commands.clear()
    }
    fn get_filterable_columns(&self) -> &[usize] {
        &[0, 1, 2]
    }
    fn get_sort_popup_cur(&self) -> usize {
        self.sort.cur
    }
    fn get_filtered_items(&self) -> impl Iterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> {
        // We are doing a lot here every draw cycle!
        self.get_filtered_list_iter()
            .map(|ls| ls.get_fields(Self::subcolumns_of_vec()).into_iter())
    }
    fn sort_popup_shown(&self) -> bool {
        self.sort.shown
    }
    fn filter_popup_shown(&self) -> bool {
        self.filter.shown
    }
    fn get_sort_state(&self) -> &ratatui::widgets::ListState {
        &self.sort.state
    }
    fn get_mut_sort_state(&mut self) -> &mut ratatui::widgets::ListState {
        &mut self.sort.state
    }
    fn get_mut_filter_state(&mut self) -> &mut rat_text::text_input::TextInputState {
        &mut self.filter.filter_text
    }
}
impl HasTitle for SongSearchBrowser {
    fn get_title(&self) -> std::borrow::Cow<'_, str> {
        match self.song_list.state {
            ListStatus::New => "Songs".into(),
            ListStatus::Loading => "Songs - loading".into(),
            ListStatus::InProgress => format!(
                "Songs - {} results - loading",
                self.song_list.get_list_iter().len()
            )
            .into(),
            ListStatus::Loaded => {
                format!("Songs - {} results", self.song_list.get_list_iter().len()).into()
            }
            ListStatus::Error => "Songs - Error receieved".into(),
        }
    }
}
impl SongSearchBrowser {
    pub fn new() -> Self {
        Self {
            input_routing: Default::default(),
            song_list: Default::default(),
            search_popped: true,
            search: Default::default(),
            widget_state: Default::default(),
            sort: Default::default(),
            filter: Default::default(),
            cur_selected: Default::default(),
        }
    }
    pub fn subcolumns_of_vec() -> [ListSongDisplayableField; 5] {
        [
            ListSongDisplayableField::Song,
            ListSongDisplayableField::Artists,
            ListSongDisplayableField::Album,
            ListSongDisplayableField::Duration,
            ListSongDisplayableField::Plays,
        ]
    }
    /// Re-apply all sort commands in the stack in the order they were stored.
    pub fn apply_all_sort_commands(&mut self) -> Result<()> {
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
    pub fn get_filtered_list_iter(&self) -> impl Iterator<Item = &ListSong> + '_ {
        self.song_list.get_list_iter().filter(move |ls| {
            // Naive implementation.
            // TODO: Do this in a single pass and optimise.
            self.get_filter_commands()
                .iter()
                .fold(true, |acc, command| {
                    let match_found = command.matches_row(
                        ls,
                        Self::subcolumns_of_vec(),
                        self.get_filterable_columns(),
                    ); // If we find a match for each filter, can display the row.
                    acc && match_found
                })
        })
    }
    pub fn apply_filter(&mut self) {
        self.filter.shown = false;
        self.input_routing = InputRouting::List;
        let Some(filter) = self.filter.get_text().map(|s| s.to_string()) else {
            // Do nothing if no filter text
            return;
        };
        let cmd = TableFilterCommand::All(crate::app::view::Filter::Contains(
            FilterString::CaseInsensitive(filter),
        ));
        let prev_max_cur = self.get_filtered_items().count().saturating_sub(1);
        let prev_cur = self.cur_selected;
        let prev_offset = self.widget_state.offset();
        self.filter.filter_commands.push(cmd);
        // Clamp current selected row to length of list.
        let new_max_cur = self.get_filtered_items().count().saturating_sub(1);
        self.cur_selected = self.cur_selected.min(new_max_cur);
        // Adjust offset accordingly to ensure if list fits on the screen, offset is
        // zero.
        *self.widget_state.offset_mut() = get_offset_after_list_resize(
            prev_offset,
            prev_cur,
            prev_max_cur,
            self.cur_selected,
            new_max_cur,
        );
    }
    pub fn clear_filter(&mut self) {
        self.filter.shown = false;
        self.input_routing = InputRouting::List;
        self.clear_filter_commands();
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
        let Some(search_query) = self.search.get_text().map(|s| s.to_string()) else {
            // Do nothing if no text
            return AsyncTask::new_no_op();
        };
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
                |_: &mut SongSearchBrowser, _| {},
                None,
            ),
        };
        AsyncTask::new_future(
            SearchSongs(search_query),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn play_song(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
        let cur_song_idx = self.get_selected_item();
        if let Some(cur_song) = self.get_song_from_idx(cur_song_idx) {
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
    pub fn add_songs_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
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
    pub fn add_song_to_playlist(&mut self) -> impl Into<YoutuiEffect<Self>> + use<> {
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
        self.song_list.clear();
        self.song_list.append_raw_search_result_songs(song_list);
        if let Err(e) = self.apply_all_sort_commands() {
            warn!("Tried to sort a column that is not sortable - error {e}")
        };
    }
}
