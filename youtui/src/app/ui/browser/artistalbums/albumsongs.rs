use super::get_adjusted_list_column;
use crate::app::component::actionhandler::{
    ComponentEffect, DominantKeyRouter, DynKeybindsIter, TextHandler,
};
use crate::app::keycommand::KeyCommand;
use crate::app::server::{ArcServer, TaskMetadata};
use crate::app::structures::{ListSong, SongListComponent};
use crate::app::ui::action::AppAction;
use crate::app::ui::browser::{Browser, PAGE_KEY_LINES};
use crate::app::view::{
    Filter, FilterString, SortDirection, SortableTableView, TableFilterCommand, TableSortCommand,
};
use crate::app::{
    component::actionhandler::{Action, KeyRouter},
    structures::{AlbumSongsList, ListStatus, Percentage},
    view::{BasicConstraint, Loadable, Scrollable, TableView},
};
use crate::config::keybinds::{KeyEnum, KeyEnumKey};
use crate::config::Config;
use crate::error::Error;
use crate::Result;
use async_callback_manager::AsyncTask;
use rat_text::text_input::{handle_events, TextInputState};
use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::iter::Iterator;
use tracing::warn;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AlbumSongsInputRouting {
    #[default]
    List,
    Sort,
    Filter,
}

#[derive(Clone)]
pub struct AlbumSongsPanel {
    pub list: AlbumSongsList,
    keybinds: Vec<KeyCommand<AppAction>>,
    pub route: AlbumSongsInputRouting,
    pub sort: SortManager,
    pub filter: FilterManager,
    cur_selected: usize,
    pub widget_state: TableState,
}
impl_youtui_component!(AlbumSongsPanel);

// TODO: refactor
#[derive(Clone)]
pub struct FilterManager {
    filter_commands: Vec<TableFilterCommand>,
    pub filter_text: TextInputState,
    pub shown: bool,
    keybinds: Vec<KeyCommand<AppAction>>,
}
impl_youtui_component!(FilterManager);

// TODO: refactor
#[derive(Clone)]
pub struct SortManager {
    sort_commands: Vec<TableSortCommand>,
    pub shown: bool,
    pub cur: usize,
    keybinds: Vec<KeyCommand<AppAction>>,
}
impl_youtui_component!(SortManager);

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterAction {
    Close,
    ClearFilter,
    Apply,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortAction {
    Close,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}

impl Action for FilterAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Filter".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            FilterAction::Close => "Close Filter",
            FilterAction::Apply => "Apply filter",
            FilterAction::ClearFilter => "Clear filter",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            FilterAction::Close => state.album_songs_list.toggle_filter(),
            FilterAction::Apply => state.album_songs_list.apply_filter(),
            FilterAction::ClearFilter => state.album_songs_list.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}

impl Action for SortAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Filter".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            SortAction::Close => "Close sort",
            SortAction::ClearSort => "Clear sort",
            SortAction::SortSelectedAsc => "Sort ascending",
            SortAction::SortSelectedDesc => "Sort descending",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            SortAction::SortSelectedAsc => state.album_songs_list.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => state.album_songs_list.handle_sort_cur_desc(),
            SortAction::Close => state.album_songs_list.close_sort(),
            SortAction::ClearSort => state.album_songs_list.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}

impl Action for BrowserSongsAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Songs Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match &self {
            BrowserSongsAction::PlaySong => "Play song",
            BrowserSongsAction::PlaySongs => "Play songs",
            BrowserSongsAction::PlayAlbum => "Play album",
            BrowserSongsAction::AddSongToPlaylist => "Add song to playlist",
            BrowserSongsAction::AddSongsToPlaylist => "Add songs to playlist",
            BrowserSongsAction::AddAlbumToPlaylist => "Add album to playlist",
            BrowserSongsAction::Sort => "Sort",
            BrowserSongsAction::Filter => "Filter",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            BrowserSongsAction::PlayAlbum => state.play_album().await,
            BrowserSongsAction::PlaySong => state.play_song().await,
            BrowserSongsAction::PlaySongs => state.play_songs().await,
            BrowserSongsAction::AddAlbumToPlaylist => state.add_album_to_playlist().await,
            BrowserSongsAction::AddSongToPlaylist => state.add_song_to_playlist().await,
            BrowserSongsAction::AddSongsToPlaylist => state.add_songs_to_playlist().await,
            BrowserSongsAction::Sort => state.album_songs_list.handle_pop_sort(),
            BrowserSongsAction::Filter => state.album_songs_list.toggle_filter(),
        }
        AsyncTask::new_no_op()
    }
}

impl SortManager {
    fn new(config: &Config) -> Self {
        Self {
            sort_commands: Default::default(),
            shown: Default::default(),
            cur: Default::default(),
            keybinds: sort_keybinds(config),
        }
    }
}
impl FilterManager {
    fn new(config: &Config) -> Self {
        Self {
            filter_text: Default::default(),
            filter_commands: Default::default(),
            shown: Default::default(),
            keybinds: filter_keybinds(config),
        }
    }
}
impl TextHandler for FilterManager {
    fn is_text_handling(&self) -> bool {
        true
    }
    fn get_text(&self) -> &str {
        self.filter_text.text()
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        self.filter_text.set_text(text)
    }
    fn clear_text(&mut self) -> bool {
        self.filter_text.clear()
    }
    fn handle_event_repr(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match handle_events(&mut self.filter_text, true, event) {
            rat_text::event::TextOutcome::Continue => None,
            rat_text::event::TextOutcome::Unchanged => Some(AsyncTask::new_no_op()),
            rat_text::event::TextOutcome::Changed => Some(AsyncTask::new_no_op()),
            rat_text::event::TextOutcome::TextChanged => Some(AsyncTask::new_no_op()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArtistSongsAction {
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
    Up,
    Down,
    PageUp,
    PageDown,
    SortUp,
    SortDown,
    // Could just be two commands.
    PopSort,
    CloseSort,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
    ToggleFilter,
    ApplyFilter,
    ClearFilter,
}

impl AlbumSongsPanel {
    pub fn new(config: &Config) -> AlbumSongsPanel {
        AlbumSongsPanel {
            keybinds: songs_keybinds(config),
            cur_selected: Default::default(),
            list: Default::default(),
            route: Default::default(),
            sort: SortManager::new(config),
            filter: FilterManager::new(config),
            widget_state: Default::default(),
        }
    }
    pub fn subcolumns_of_vec() -> &'static [usize] {
        &[1, 3, 4, 5, 6]
    }
    pub fn apply_sort_commands(&mut self) -> Result<()> {
        for c in self.sort.sort_commands.iter() {
            if !self.get_sortable_columns().contains(&c.column) {
                return Err(Error::Other(format!("Unable to sort column {}", c.column,)));
            }
            self.list.sort(
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
        Box::new(self.list.get_list_iter().filter(move |ls| {
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
        self.route = AlbumSongsInputRouting::List;
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
        self.route = AlbumSongsInputRouting::List;
        self.filter.filter_commands.clear();
    }
    fn open_sort(&mut self) {
        self.sort.shown = true;
        self.route = AlbumSongsInputRouting::Sort;
    }
    pub fn toggle_filter(&mut self) {
        let shown = self.filter.shown;
        if !shown {
            // We need to set cur back to 0  and clear text somewhere and I'd prefer to do
            // it at the time of showing, so it cannot be missed.
            self.filter.filter_text.clear();
            self.route = AlbumSongsInputRouting::Filter;
        } else {
            self.route = AlbumSongsInputRouting::List;
        }
        self.filter.shown = !shown;
    }
    pub fn close_sort(&mut self) {
        self.sort.shown = false;
        self.route = AlbumSongsInputRouting::List;
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
    // TODO: Could be under Scrollable trait.
    pub fn handle_sort_up(&mut self) {
        self.sort.cur = self.sort.cur.saturating_sub(1)
    }
    pub fn handle_sort_down(&mut self) {
        self.sort.cur = self
            .sort
            .cur
            .saturating_add(1)
            .min(self.get_sortable_columns().len().saturating_sub(1));
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
    pub fn handle_songs_found(&mut self) {
        self.list.clear();
        // XXX: Consider clearing sort params here, so that we don't need to sort all
        // the incoming songs. Performance seems OK for now. XXX: Consider also
        // clearing filter params here.
        self.cur_selected = 0;
        self.list.state = ListStatus::InProgress;
    }
}

impl SongListComponent for AlbumSongsPanel {
    fn get_song_from_idx(&self, idx: usize) -> Option<&crate::app::structures::ListSong> {
        self.get_filtered_list_iter().nth(idx)
    }
}

impl TextHandler for AlbumSongsPanel {
    fn get_text(&self) -> &str {
        self.filter.get_text()
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        self.filter.replace_text(text)
    }
    fn is_text_handling(&self) -> bool {
        self.route == AlbumSongsInputRouting::Filter
    }
    fn clear_text(&mut self) -> bool {
        self.filter.clear_text()
    }
    fn handle_event_repr(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        self.filter
            .handle_event_repr(event)
            .map(|effect| effect.map(|this: &mut AlbumSongsPanel| &mut this.filter))
    }
}

impl DominantKeyRouter<AppAction> for AlbumSongsPanel {
    fn dominant_keybinds_active(&self) -> bool {
        self.sort.shown || self.filter.shown
    }

    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &'_ KeyCommand<AppAction>> + '_ {
        self.get_active_keybinds()
    }
}

impl KeyRouter<AppAction> for AlbumSongsPanel {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &'_ KeyCommand<AppAction>> + '_ {
        self.keybinds.iter().chain(self.sort.keybinds.iter())
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &'_ KeyCommand<AppAction>> + '_ {
        Box::new(match self.route {
            AlbumSongsInputRouting::List => self.keybinds.iter(),
            AlbumSongsInputRouting::Sort => self.sort.keybinds.iter(),
            AlbumSongsInputRouting::Filter => self.filter.keybinds.iter(),
        }) as DynKeybindsIter<'_, AppAction>
    }
}

// Is this still relevant?
impl Loadable for AlbumSongsPanel {
    fn is_loading(&self) -> bool {
        matches!(self.list.state, crate::app::structures::ListStatus::Loading)
    }
}
impl Scrollable for AlbumSongsPanel {
    fn increment_list(&mut self, amount: isize) {
        // Naive check using iterator - consider using exact size iterator
        self.cur_selected = self
            .cur_selected
            .saturating_add_signed(amount)
            .min(self.get_filtered_items().count().saturating_sub(1))
    }
    fn get_selected_item(&self) -> usize {
        self.cur_selected
    }
}

impl TableView for AlbumSongsPanel {
    fn get_state(&self) -> ratatui::widgets::TableState {
        self.widget_state.clone()
    }
    fn get_title(&self) -> Cow<str> {
        match self.list.state {
            ListStatus::New => "Songs".into(),
            ListStatus::Loading => "Songs - loading".into(),
            ListStatus::InProgress => format!(
                "Songs - {} results - loading",
                self.list.get_list_iter().len()
            )
            .into(),
            ListStatus::Loaded => {
                format!("Songs - {} results", self.list.get_list_iter().len()).into()
            }
            ListStatus::Error => "Songs - Error receieved".into(),
        }
    }
    fn get_layout(&self) -> &[BasicConstraint] {
        &[
            BasicConstraint::Length(4),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Length(10),
            BasicConstraint::Length(5),
        ]
    }

    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = crate::app::view::TableItem> + '_> {
        let b = self.list.get_list_iter().map(|ls| {
            let song_iter = ls.get_fields_iter().enumerate().filter_map(|(i, f)| {
                if Self::subcolumns_of_vec().contains(&i) {
                    Some(f)
                } else {
                    None
                }
            });
            // XXX: Seems to be a double allocation here - may be able to use dereferences
            // to address.
            Box::new(song_iter) as Box<dyn Iterator<Item = Cow<'_, str>>>
        });
        Box::new(b)
    }

    fn get_headings(&self) -> Box<(dyn Iterator<Item = &'static str> + 'static)> {
        Box::new(["#", "Album", "Song", "Duration", "Year"].into_iter())
    }

    fn get_highlighted_row(&self) -> Option<usize> {
        None
    }
}
impl SortableTableView for AlbumSongsPanel {
    fn get_sortable_columns(&self) -> &[usize] {
        &[1, 4]
    }
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()> {
        // TODO: Maintain a view only struct, for easier rendering of this.
        if !self.get_sortable_columns().contains(&sort_command.column) {
            return Err(Error::Other(format!(
                "Unable to sort column {}",
                sort_command.column,
            )));
        }
        // Map the column of ArtistAlbums to a column of List and sort
        self.list.sort(
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
    fn get_sort_commands(&self) -> &[TableSortCommand] {
        &self.sort.sort_commands
    }
    fn get_filtered_items(&self) -> Box<dyn Iterator<Item = crate::app::view::TableItem> + '_> {
        // We are doing a lot here every draw cycle!
        Box::new(self.get_filtered_list_iter().map(|ls| {
            Box::new(ls.get_fields_iter().enumerate().filter_map(|(i, f)| {
                if Self::subcolumns_of_vec().contains(&i) {
                    Some(f)
                } else {
                    None
                }
            })) as Box<dyn Iterator<Item = Cow<str>>>
        }))
    }
    fn get_filterable_columns(&self) -> &[usize] {
        &[1, 2, 4]
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
}

fn sort_keybinds(config: &Config) -> Vec<KeyCommand<AppAction>> {
    config
        .keybinds
        .sort
        .iter()
        .map(|(kb, ke)| match ke {
            KeyEnum::Key(KeyEnumKey {
                action,
                value,
                visibility,
            }) => KeyCommand::new_modified_from_code_with_visibility(
                kb.code,
                kb.modifiers,
                visibility.clone(),
                action.clone(),
            ),
            KeyEnum::Mode(_) => todo!(),
        })
        .collect()
}

fn filter_keybinds(config: &Config) -> Vec<KeyCommand<AppAction>> {
    config
        .keybinds
        .filter
        .iter()
        .map(|(kb, ke)| match ke {
            KeyEnum::Key(KeyEnumKey {
                action,
                value,
                visibility,
            }) => KeyCommand::new_modified_from_code_with_visibility(
                kb.code,
                kb.modifiers,
                visibility.clone(),
                action.clone(),
            ),
            KeyEnum::Mode(_) => todo!(),
        })
        .collect()
}

pub fn songs_keybinds(config: &Config) -> Vec<KeyCommand<AppAction>> {
    config
        .keybinds
        .browser_songs
        .iter()
        .map(|(kb, ke)| match ke {
            KeyEnum::Key(KeyEnumKey {
                action,
                value,
                visibility,
            }) => KeyCommand::new_modified_from_code_with_visibility(
                kb.code,
                kb.modifiers,
                visibility.clone(),
                action.clone(),
            ),
            KeyEnum::Mode(_) => todo!(),
        })
        .collect()
}
