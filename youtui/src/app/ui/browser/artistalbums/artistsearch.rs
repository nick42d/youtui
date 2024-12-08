use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, Component, ComponentEffect, KeyRouter, Suggestable, TextHandler,
        },
        server::{ArcServer, GetSearchSuggestions, TaskMetadata},
        ui::{
            action::{AppAction, ListAction, PAGE_KEY_LINES},
            browser::Browser,
        },
        view::{ListView, Loadable, Scrollable, SortableList},
    },
    config::{keymap::Keymap, Config},
};
use async_callback_manager::{AsyncTask, Constraint};
use rat_text::text_input::{handle_events, TextInputState};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, iter::Iterator};
use tracing::error;
use ytmapi_rs::{common::SearchSuggestion, parse::SearchResultArtist};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ArtistInputRouting {
    Search,
    #[default]
    List,
}

pub struct ArtistSearchPanel {
    pub list: Vec<SearchResultArtist>,
    // Duplicate of search popped?
    // Could be a function instead.
    pub route: ArtistInputRouting,
    selected: usize,
    sort_commands_list: Vec<String>,
    keybinds: Keymap<AppAction>,
    search_keybinds: Keymap<AppAction>,
    pub search_popped: bool,
    pub search: SearchBlock,
    pub widget_state: ListState,
}

pub struct SearchBlock {
    pub search_contents: TextInputState,
    pub search_suggestions: Vec<SearchSuggestion>,
    pub suggestions_cur: Option<usize>,
}
impl_youtui_component!(SearchBlock);

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

impl Action for BrowserArtistsAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            Self::DisplaySelectedArtistAlbums => "Display albums for selected artist",
        }
        .into()
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserSearchAction {
    SearchArtist,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}
impl Action for BrowserSearchAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            BrowserSearchAction::SearchArtist => "Search",
            BrowserSearchAction::PrevSearchSuggestion => "Prev Search Suggestion",
            BrowserSearchAction::NextSearchSuggestion => "Next Search Suggestion",
        }
        .into()
    }
}
impl ActionHandler<BrowserArtistsAction> for Browser {
    async fn apply_action(
        &mut self,
        action: BrowserArtistsAction,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self> {
        match action {
            BrowserArtistsAction::DisplaySelectedArtistAlbums => self.get_songs(),
        }
    }
}
impl ActionHandler<BrowserSearchAction> for Browser {
    async fn apply_action(
        &mut self,
        action: BrowserSearchAction,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self> {
        match action {
            BrowserSearchAction::SearchArtist => return self.search(),
            BrowserSearchAction::PrevSearchSuggestion => self.artist_list.search.increment_list(-1),
            BrowserSearchAction::NextSearchSuggestion => self.artist_list.search.increment_list(1),
        }
        AsyncTask::new_no_op()
    }
}
impl ArtistSearchPanel {
    pub fn new(config: &Config) -> Self {
        Self {
            keybinds: browser_artist_search_keybinds(config),
            search_keybinds: search_keybinds(config),
            list: Default::default(),
            route: Default::default(),
            selected: Default::default(),
            sort_commands_list: Default::default(),
            search_popped: Default::default(),
            search: SearchBlock::new(),
            widget_state: Default::default(),
        }
    }
    pub fn open_search(&mut self) {
        self.search_popped = true;
        self.route = ArtistInputRouting::Search;
    }
    pub fn close_search(&mut self) {
        self.search_popped = false;
        self.route = ArtistInputRouting::List;
    }
    pub fn handle_list_action(&mut self, action: ListAction) -> ComponentEffect<Self> {
        if self.route != ArtistInputRouting::List {
            return AsyncTask::new_no_op();
        }
        match action {
            ListAction::Up => self.increment_list(-1),
            ListAction::Down => self.increment_list(1),
            ListAction::PageUp => self.increment_list(-PAGE_KEY_LINES),
            ListAction::PageDown => self.increment_list(PAGE_KEY_LINES),
        }
        AsyncTask::new_no_op()
    }
}
impl Component for ArtistSearchPanel {
    type Bkend = ArcServer;
    type Md = TaskMetadata;
}

impl TextHandler for SearchBlock {
    fn is_text_handling(&self) -> bool {
        true
    }
    fn get_text(&self) -> &str {
        self.search_contents.text()
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        self.search_contents.set_text(text);
        self.search_contents.move_to_line_end(false);
    }
    fn clear_text(&mut self) -> bool {
        self.search_suggestions.clear();
        self.search_contents.clear()
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match handle_events(&mut self.search_contents, true, event) {
            rat_text::event::TextOutcome::Continue => None,
            rat_text::event::TextOutcome::Unchanged => Some(AsyncTask::new_no_op()),
            rat_text::event::TextOutcome::Changed => Some(AsyncTask::new_no_op()),
            rat_text::event::TextOutcome::TextChanged => Some(self.fetch_search_suggestions()),
        }
    }
}

impl SearchBlock {
    pub fn new() -> Self {
        Self {
            search_contents: Default::default(),
            search_suggestions: Default::default(),
            suggestions_cur: Default::default(),
        }
    }
    // Ask the UI for search suggestions for the current query
    fn fetch_search_suggestions(&mut self) -> AsyncTask<Self, ArcServer, TaskMetadata> {
        // No need to fetch search suggestions if contents is empty.
        if self.search_contents.is_empty() {
            self.search_suggestions.clear();
            return AsyncTask::new_no_op();
        }
        let handler = |this: &mut Self, results| match results {
            Ok((suggestions, text)) => {
                this.replace_search_suggestions(suggestions, text);
            }
            Err(e) => {
                error!("Error <{e}> recieved getting search suggestions");
            }
        };
        AsyncTask::new_future(
            GetSearchSuggestions(self.get_text().to_string()),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    fn replace_search_suggestions(
        &mut self,
        search_suggestions: Vec<SearchSuggestion>,
        search: String,
    ) {
        if self.get_text() == search {
            self.search_suggestions = search_suggestions;
            self.suggestions_cur = None;
        }
    }
    pub fn increment_list(&mut self, amount: isize) {
        if !self.search_suggestions.is_empty() {
            self.suggestions_cur = Some(
                self.suggestions_cur
                    .map(|cur| {
                        cur.saturating_add_signed(amount)
                            .min(self.search_suggestions.len() - 1)
                    })
                    .unwrap_or_default(),
            );
            // Safe - clamped above
            // Clone is ok here as we want to duplicate the search suggestion.
            self.replace_text(
                self.search_suggestions[self.suggestions_cur.expect("Set to non-None value above")]
                    .get_text(),
            );
        }
    }
}

impl TextHandler for ArtistSearchPanel {
    fn is_text_handling(&self) -> bool {
        self.route == ArtistInputRouting::Search
    }
    fn get_text(&self) -> &str {
        self.search.get_text()
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        self.search.replace_text(text)
    }
    fn clear_text(&mut self) -> bool {
        self.search.clear_text()
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        self.search
            .handle_text_event_impl(event)
            .map(|effect| effect.map(|this: &mut Self| &mut this.search))
    }
}

impl Suggestable for ArtistSearchPanel {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        self.search.search_suggestions.as_slice()
    }
    fn has_search_suggestions(&self) -> bool {
        !self.search.search_suggestions.is_empty()
    }
}

impl KeyRouter<AppAction> for ArtistSearchPanel {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        [&self.keybinds, &self.search_keybinds].into_iter()
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        match self.route {
            ArtistInputRouting::List => std::iter::once(&self.keybinds),
            ArtistInputRouting::Search => std::iter::once(&self.search_keybinds),
        }
    }
}

impl Scrollable for ArtistSearchPanel {
    fn increment_list(&mut self, amount: isize) {
        self.selected = self
            .selected
            .checked_add_signed(amount)
            .unwrap_or(0)
            .min(self.len().checked_add_signed(-1).unwrap_or(0));
    }
    fn is_scrollable(&self) -> bool {
        todo!()
    }
}

impl SortableList for ArtistSearchPanel {
    // Could instead be lazy
    fn push_sort_command(&mut self, _list_sort_command: String) {
        self.list.sort_by(|a, b| a.artist.cmp(&b.artist));
    }
    fn clear_sort_commands(&mut self) {
        self.sort_commands_list.clear();
    }
}
impl Loadable for ArtistSearchPanel {
    fn is_loading(&self) -> bool {
        // This is just a basic list without a loading function.
        false
    }
}
impl ListView for ArtistSearchPanel {
    fn get_selected_item(&self) -> usize {
        self.selected
    }
    type DisplayItem = String;
    fn get_state(&self) -> ratatui::widgets::ListState {
        self.widget_state.clone()
    }
    fn get_items_display(&self) -> Vec<&Self::DisplayItem> {
        self.list
            .iter()
            .map(|search_result| &search_result.artist)
            .collect()
    }
    fn get_title(&self) -> Cow<str> {
        "Artists".into()
    }
}
fn search_keybinds(config: &Config) -> Keymap<AppAction> {
    config.keybinds.browser_search.clone()
}
fn browser_artist_search_keybinds(config: &Config) -> Keymap<AppAction> {
    config.keybinds.browser_artists.clone()
}
