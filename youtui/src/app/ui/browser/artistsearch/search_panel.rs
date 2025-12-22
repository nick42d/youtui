use crate::app::component::actionhandler::{
    Action, Component, ComponentEffect, KeyRouter, Scrollable, Suggestable, TextHandler,
};
use crate::app::server::{ArcServer, TaskMetadata};
use crate::app::ui::action::AppAction;
use crate::app::ui::browser::shared_components::SearchBlock;
use crate::app::view::{HasTitle, ListView};
use crate::config::Config;
use crate::config::keymap::Keymap;
use crate::widgets::ScrollingListState;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::iter::Iterator;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::parse::SearchResultArtist;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ArtistInputRouting {
    #[default]
    Search,
    List,
}

pub struct ArtistSearchPanel {
    pub list: Vec<SearchResultArtist>,
    // Duplicate of search popped?
    // Could be a function instead.
    pub route: ArtistInputRouting,
    selected: usize,
    pub search_popped: bool,
    pub search: SearchBlock,
    pub widget_state: ScrollingListState,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

impl Action for BrowserArtistsAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::DisplaySelectedArtistAlbums => "Display albums for selected artist",
        }
        .into()
    }
}

impl ArtistSearchPanel {
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
    fn get_text(&self) -> std::option::Option<&str> {
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
        self.search.get_search_suggestions()
    }
    fn has_search_suggestions(&self) -> bool {
        self.search.has_search_suggestions()
    }
}

impl KeyRouter<AppAction> for ArtistSearchPanel {
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
            ArtistInputRouting::List => std::iter::once(&config.keybinds.browser_artists),
            ArtistInputRouting::Search => std::iter::once(&config.keybinds.browser_search),
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
impl ListView for ArtistSearchPanel {
    fn get_selected_item(&self) -> usize {
        self.selected
    }
    fn get_state(&self) -> &ScrollingListState {
        &self.widget_state
    }
    fn get_mut_state(&mut self) -> &mut ScrollingListState {
        &mut self.widget_state
    }
    fn get_items(&self) -> impl ExactSizeIterator<Item = Cow<'_, str>> + '_ {
        self.list
            .iter()
            .map(|search_result| (&search_result.artist).into())
    }
}
impl HasTitle for ArtistSearchPanel {
    fn get_title(&self) -> Cow<'_, str> {
        "Artists".into()
    }
}
