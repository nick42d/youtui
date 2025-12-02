use crate::app::component::actionhandler::{
    Action, Component, ComponentEffect, KeyRouter, Scrollable, Suggestable, TextHandler,
};
use crate::app::server::{ArcServer, TaskMetadata};
use crate::app::ui::action::AppAction;
use crate::app::ui::browser::shared_components::SearchBlock;
use crate::app::view::{ListView, Loadable};
use crate::config::Config;
use crate::config::keymap::Keymap;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::iter::Iterator;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::parse::SearchResultPlaylist;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PlaylistInputRouting {
    #[default]
    Search,
    List,
}

pub struct PlaylistSearchPanel {
    pub list: Vec<SearchResultPlaylist>,
    // Duplicate of search popped?
    // Could be a function instead.
    pub route: PlaylistInputRouting,
    selected: usize,
    pub search_popped: bool,
    pub search: SearchBlock,
    pub widget_state: ListState,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserPlaylistsAction {
    DisplaySelectedPlaylist,
}

impl Action for BrowserPlaylistsAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        "Playlist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::DisplaySelectedPlaylist => "Display selected playlist",
        }
        .into()
    }
}

impl PlaylistSearchPanel {
    pub fn new() -> Self {
        Self {
            list: Default::default(),
            route: Default::default(),
            selected: Default::default(),
            search_popped: true,
            search: SearchBlock::default(),
            widget_state: Default::default(),
        }
    }
    pub fn open_search(&mut self) {
        self.search_popped = true;
        self.route = PlaylistInputRouting::Search;
    }
    pub fn close_search(&mut self) {
        self.search_popped = false;
        self.route = PlaylistInputRouting::List;
    }
}
impl Component for PlaylistSearchPanel {
    type Bkend = ArcServer;
    type Md = TaskMetadata;
}

impl TextHandler for PlaylistSearchPanel {
    fn is_text_handling(&self) -> bool {
        self.route == PlaylistInputRouting::Search
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

impl Suggestable for PlaylistSearchPanel {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        self.search.get_search_suggestions()
    }
    fn has_search_suggestions(&self) -> bool {
        self.search.has_search_suggestions()
    }
}

impl KeyRouter<AppAction> for PlaylistSearchPanel {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        [
            &config.keybinds.browser_artists,
            &config.keybinds.browser_search,
        ]
        .into_iter()
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.route {
            PlaylistInputRouting::List => std::iter::once(&config.keybinds.browser_artists),
            PlaylistInputRouting::Search => std::iter::once(&config.keybinds.browser_search),
        }
    }
}

impl Scrollable for PlaylistSearchPanel {
    fn increment_list(&mut self, amount: isize) {
        self.selected = self
            .selected
            .checked_add_signed(amount)
            .unwrap_or(0)
            .min(self.len().checked_add_signed(-1).unwrap_or(0));
    }
    fn is_scrollable(&self) -> bool {
        self.route == PlaylistInputRouting::List
    }
}
impl Loadable for PlaylistSearchPanel {
    fn is_loading(&self) -> bool {
        // This is just a basic list without a loading function.
        false
    }
}
impl ListView for PlaylistSearchPanel {
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
            .map(|search_result| match search_result {
                SearchResultPlaylist::Featured(r) => &r.title,
                SearchResultPlaylist::Community(r) => &r.title,
                SearchResultPlaylist::Podcast(r) => &r.title,
                _ => todo!(),
            })
            .collect()
    }
    fn get_title(&self) -> Cow<'_, str> {
        "Playlists".into()
    }
}
