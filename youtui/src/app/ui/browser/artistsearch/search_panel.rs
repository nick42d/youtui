use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, Component, ComponentEffect, KeyRouter, Scrollable, Suggestable,
            TextHandler, YoutuiEffect,
        },
        server::{ArcServer, GetSearchSuggestions, HandleApiError, TaskMetadata},
        ui::{
            action::AppAction,
            browser::{
                shared_components::{BrowserSearchAction, SearchBlock},
                Browser,
            },
        },
        view::{ListView, Loadable, SortableList},
    },
    config::{keymap::Keymap, Config},
};
use async_callback_manager::{AsyncTask, Constraint};
use rat_text::text_input::{handle_events, TextInputState};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, iter::Iterator};
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

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

impl Action for BrowserArtistsAction {
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
            search: SearchBlock::default(),
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
}
impl Component for ArtistSearchPanel {
    type Bkend = ArcServer;
    type Md = TaskMetadata;
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
        self.route == ArtistInputRouting::List
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
