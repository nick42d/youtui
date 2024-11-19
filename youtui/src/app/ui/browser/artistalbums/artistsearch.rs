use std::borrow::Cow;

use crossterm::event::KeyCode;
use rat_text::text_input::{handle_events, TextInputState};
use ratatui::widgets::ListState;
use ytmapi_rs::{common::SearchSuggestion, parse::SearchResultArtist};

use crate::app::{
    component::actionhandler::{Action, KeyRouter, Suggestable, TextHandler},
    keycommand::KeyCommand,
    ui::browser::BrowserAction,
    view::{ListView, Loadable, Scrollable, SortableList},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ArtistInputRouting {
    Search,
    #[default]
    List,
}

#[derive(Default, Clone)]
pub struct ArtistSearchPanel {
    pub list: Vec<SearchResultArtist>,
    // Duplicate of search popped?
    // Could be a function instead.
    pub route: ArtistInputRouting,
    selected: usize,
    sort_commands_list: Vec<String>,
    keybinds: Vec<KeyCommand<BrowserAction>>,
    search_keybinds: Vec<KeyCommand<BrowserAction>>,
    pub search_popped: bool,
    pub search: SearchBlock,
    pub widget_state: ListState,
}

#[derive(Default, Clone)]
pub struct SearchBlock {
    pub search_contents: TextInputState,
    pub search_suggestions: Vec<SearchSuggestion>,
    pub suggestions_cur: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArtistAction {
    DisplayAlbums,
    // XXX: This could be a subset - eg ListAction
    Up,
    Down,
    PageUp,
    PageDown,
    // XXX: Could be a subset just for search
    Search,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}

impl ArtistSearchPanel {
    pub fn new() -> Self {
        Self {
            keybinds: browser_artist_search_keybinds(),
            search_keybinds: search_keybinds(),
            ..Default::default()
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
impl Action for ArtistAction {
    fn context(&self) -> Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> Cow<str> {
        match &self {
            Self::Search => "Search",
            Self::DisplayAlbums => "Display albums for selected artist",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
            ArtistAction::PrevSearchSuggestion => "Next Search Suggestion",
            ArtistAction::NextSearchSuggestion => "Prev Search Suggestion",
        }
        .into()
    }
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
    }
    fn clear_text(&mut self) -> bool {
        self.search_contents.clear()
    }
    fn handle_event_repr(&mut self, event: &crossterm::event::Event) -> bool {
        match handle_events(&mut self.search_contents, true, event) {
            rat_text::event::TextOutcome::Continue => false,
            rat_text::event::TextOutcome::Unchanged => true,
            rat_text::event::TextOutcome::Changed => true,
            rat_text::event::TextOutcome::TextChanged => true,
        }
    }
}

impl SearchBlock {
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
            self.search_contents.set_text(
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
    fn handle_event_repr(&mut self, event: &crossterm::event::Event) -> bool {
        self.search.handle_event_repr(&event)
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

impl KeyRouter<BrowserAction> for ArtistSearchPanel {
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        Box::new(self.keybinds.iter().chain(self.search_keybinds.iter()))
    }
    fn get_routed_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        Box::new(match self.route {
            ArtistInputRouting::List => self.keybinds.iter(),
            ArtistInputRouting::Search => self.search_keybinds.iter(),
        })
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
    fn get_selected_item(&self) -> usize {
        self.selected
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
fn search_keybinds() -> Vec<KeyCommand<BrowserAction>> {
    vec![
        KeyCommand::new_from_code(KeyCode::Enter, BrowserAction::Artist(ArtistAction::Search)),
        KeyCommand::new_from_code(
            KeyCode::Down,
            BrowserAction::Artist(ArtistAction::NextSearchSuggestion),
        ),
        KeyCommand::new_from_code(
            KeyCode::Up,
            BrowserAction::Artist(ArtistAction::PrevSearchSuggestion),
        ),
    ]
}
fn browser_artist_search_keybinds() -> Vec<KeyCommand<BrowserAction>> {
    vec![
        KeyCommand::new_from_code(
            KeyCode::Enter,
            BrowserAction::Artist(ArtistAction::DisplayAlbums),
        ),
        // XXX: Consider if these type of actions can be for all lists.
        KeyCommand::new_hidden_from_code(KeyCode::Down, BrowserAction::Artist(ArtistAction::Down)),
        KeyCommand::new_hidden_from_code(KeyCode::Up, BrowserAction::Artist(ArtistAction::Up)),
        KeyCommand::new_from_code(KeyCode::PageUp, BrowserAction::Artist(ArtistAction::PageUp)),
        KeyCommand::new_from_code(
            KeyCode::PageDown,
            BrowserAction::Artist(ArtistAction::PageDown),
        ),
    ]
}
