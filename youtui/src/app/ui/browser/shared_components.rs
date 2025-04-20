use crate::app::component::actionhandler::Action;
use crate::app::component::actionhandler::{ComponentEffect, TextHandler};
use crate::app::server::{GetSearchSuggestions, HandleApiError};
use crate::app::ui::browser::Browser;
use crate::app::view::{TableFilterCommand, TableSortCommand};
use async_callback_manager::{AsyncTask, Constraint};
use rat_text::text_input::{handle_events, TextInputState};
use serde::{Deserialize, Serialize};
use ytmapi_rs::common::SearchSuggestion;

#[derive(Default)]
pub struct SearchBlock {
    pub search_contents: TextInputState,
    pub search_suggestions: Vec<SearchSuggestion>,
    pub suggestions_cur: Option<usize>,
}
impl_youtui_component!(SearchBlock);

// TODO: refactor
#[derive(Clone, Default)]
pub struct FilterManager {
    pub filter_commands: Vec<TableFilterCommand>,
    pub filter_text: TextInputState,
    pub shown: bool,
}
impl_youtui_component!(FilterManager);

// TODO: refactor
#[derive(Clone, Default)]
pub struct SortManager {
    pub sort_commands: Vec<TableSortCommand>,
    pub shown: bool,
    pub cur: usize,
}
impl_youtui_component!(SortManager);

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterAction {
    Close,
    ClearFilter,
    Apply,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortAction {
    Close,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserSearchAction {
    SearchArtist,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}

impl Action for FilterAction {
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
}

impl Action for SortAction {
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
}

impl Action for BrowserSearchAction {
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

impl SortManager {
    pub fn new() -> Self {
        Self {
            sort_commands: Default::default(),
            shown: Default::default(),
            cur: Default::default(),
        }
    }
}
impl FilterManager {
    pub fn new() -> Self {
        Self {
            filter_text: Default::default(),
            filter_commands: Default::default(),
            shown: Default::default(),
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
    fn handle_text_event_impl(
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
    // Ask the UI for search suggestions for the current query
    fn fetch_search_suggestions(&mut self) -> ComponentEffect<Self> {
        // No need to fetch search suggestions if contents is empty.
        if self.search_contents.is_empty() {
            self.search_suggestions.clear();
            return AsyncTask::new_no_op();
        }
        let handler = |this: &mut Self, results| match results {
            Ok((suggestions, text)) => {
                this.replace_search_suggestions(suggestions, text);
                AsyncTask::new_no_op()
            }
            Err(error) => AsyncTask::new_future(
                HandleApiError {
                    error,
                    // To avoid needing to clone search query to use in the error message, this
                    // error message is minimal.
                    message: "Error recieved getting search suggestions".to_string(),
                },
                |_, _| {},
                None,
            ),
        };
        AsyncTask::new_future_chained(
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
