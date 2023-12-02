use crate::app::ui::browser::BrowserAction;
use crate::app::view::{SortDirection, SortableTableView, TableSortCommand};
use crate::app::{
    component::actionhandler::{Action, KeyHandler, KeyRouter, Keybind, Suggestable, TextHandler},
    structures::{AlbumSongsList, ListStatus, Percentage},
    view::{BasicConstraint, ListView, Loadable, Scrollable, SortableList, TableView},
};
use crate::error::Error;
use crate::Result;
use crossterm::event::KeyCode;
use std::borrow::Cow;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::parse::SearchResultArtist;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ArtistInputRouting {
    Search,
    #[default]
    List,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AlbumSongsInputRouting {
    Sort,
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
    keybinds: Vec<Keybind<BrowserAction>>,
    search_keybinds: Vec<Keybind<BrowserAction>>,
    pub search_popped: bool,
    pub search: SearchBlock,
}

#[derive(Default, Clone)]
pub struct SearchBlock {
    pub search_contents: String,
    pub search_suggestions: Vec<SearchSuggestion>,
    pub text_cur: usize,
    pub suggestions_cur: Option<usize>,
}

#[derive(Default, Clone)]
pub struct AlbumSongsPanel {
    pub list: AlbumSongsList,
    keybinds: Vec<Keybind<BrowserAction>>,
    sort_keybinds: Vec<Keybind<BrowserAction>>,
    sort_commands: Vec<TableSortCommand>,
    pub route: AlbumSongsInputRouting,
    pub sort_popped: bool,
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
    PopSort,
    CloseSort,
    ClearSort,
    Sort(usize, SortDirection),
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

impl AlbumSongsPanel {
    pub fn new() -> AlbumSongsPanel {
        AlbumSongsPanel {
            keybinds: songs_keybinds(),
            sort_keybinds: sort_keybinds(),
            ..Default::default()
        }
    }
    pub fn subcolumns_of_vec() -> &'static [usize] {
        &[1, 3, 4, 5, 6]
    }
    pub fn open_sort(&mut self) {
        self.sort_popped = true;
        self.route = AlbumSongsInputRouting::Sort;
    }
    pub fn close_sort(&mut self) {
        self.sort_popped = false;
        self.route = AlbumSongsInputRouting::List;
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
    fn push_text(&mut self, c: char) {
        self.search_contents.push(c);
        self.text_cur += 1;
    }
    fn pop_text(&mut self) {
        self.search_contents.pop();
        self.text_cur = self.text_cur.saturating_sub(1);
    }
    fn is_text_handling(&self) -> bool {
        true
    }
    fn take_text(&mut self) -> String {
        self.text_cur = 0;
        self.search_suggestions.clear();
        std::mem::take(&mut self.search_contents)
    }
    fn replace_text(&mut self, text: String) {
        self.search_contents = text;
        self.move_cursor_to_end();
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
            self.search_contents = self.search_suggestions
                [self.suggestions_cur.expect("Set to non-None value above")]
            .get_text();
            self.move_cursor_to_end();
        }
    }
    fn move_cursor_to_end(&mut self) {
        self.text_cur = self.search_contents.len();
    }
}

impl TextHandler for ArtistSearchPanel {
    fn push_text(&mut self, c: char) {
        self.search.push_text(c);
    }
    fn pop_text(&mut self) {
        self.search.pop_text();
    }
    fn is_text_handling(&self) -> bool {
        self.route == ArtistInputRouting::Search
    }
    fn take_text(&mut self) -> String {
        self.search.take_text()
    }
    fn replace_text(&mut self, text: String) {
        self.search.replace_text(text)
    }
}

impl Suggestable for ArtistSearchPanel {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        self.search.search_suggestions.as_slice()
    }
    fn has_search_suggestions(&self) -> bool {
        self.search.search_suggestions.len() > 0
    }
}

impl Action for ArtistSongsAction {
    fn context(&self) -> Cow<str> {
        "Artist Songs Panel".into()
    }
    fn describe(&self) -> Cow<str> {
        match &self {
            Self::PlaySong => "Play song",
            Self::PlaySongs => "Play songs",
            Self::PlayAlbum => "Play album",
            Self::AddSongToPlaylist => "Add song to playlist",
            Self::AddSongsToPlaylist => "Add songs to playlist",
            Self::AddAlbumToPlaylist => "Add album to playlist",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
            Self::PopSort => "Sort",
            Self::CloseSort => "Close Sort",
            Self::ClearSort => "Clear Sort",
            // TODO: Improve message
            Self::Sort(_, _) => "Sort [col, dir]",
        }
        .into()
    }
}
impl KeyRouter<BrowserAction> for ArtistSearchPanel {
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(self.keybinds.iter().chain(self.search_keybinds.iter()))
    }
}

impl KeyHandler<BrowserAction> for ArtistSearchPanel {
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
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

impl KeyRouter<BrowserAction> for AlbumSongsPanel {
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(self.keybinds.iter().chain(self.sort_keybinds.iter()))
    }
}

impl KeyHandler<BrowserAction> for AlbumSongsPanel {
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(match self.route {
            AlbumSongsInputRouting::List => self.keybinds.iter(),
            AlbumSongsInputRouting::Sort => self.sort_keybinds.iter(),
        })
    }
}

// Is this still relevant?
impl Loadable for AlbumSongsPanel {
    fn is_loading(&self) -> bool {
        match self.list.state {
            crate::app::structures::ListStatus::Loading => true,
            _ => false,
        }
    }
}
impl Scrollable for AlbumSongsPanel {
    fn increment_list(&mut self, amount: isize) {
        self.list.increment_list(amount)
    }
    fn get_selected_item(&self) -> usize {
        self.list.get_selected_item()
    }
}

impl TableView for AlbumSongsPanel {
    fn get_title(&self) -> Cow<str> {
        match self.list.state {
            ListStatus::New => "Songs".into(),
            ListStatus::Loading => "Songs - loading".into(),
            ListStatus::InProgress => {
                format!("Songs - {} results - loading", self.list.list.len()).into()
            }
            ListStatus::Loaded => format!("Songs - {} results", self.list.list.len()).into(),
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
        let b = self.list.list.iter().map(|ls| {
            let song_iter = ls.get_fields_iter().enumerate().filter_map(|(i, f)| {
                if Self::subcolumns_of_vec().contains(&i) {
                    Some(f)
                } else {
                    None
                }
            });
            // XXX: Seems to be a double allocation here - may be able to use dereferences to address.
            Box::new(song_iter) as Box<dyn Iterator<Item = Cow<'_, str>>>
        });
        Box::new(b)
    }

    fn get_headings(&self) -> Box<(dyn Iterator<Item = &'static str> + 'static)> {
        Box::new(["#", "Album", "Song", "Duration", "Year"].into_iter())
    }
}
impl SortableTableView for AlbumSongsPanel {
    fn get_sortable_columns(&self) -> &[usize] {
        // Not quite what we're expecting here.
        &[1, 4]
    }
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()> {
        if !self.get_sortable_columns().contains(&sort_command.column) {
            return Err(Error::Other(format!(
                "Unable to sort column {}",
                sort_command.column,
            )));
        }
        // Map the column of ArtistAlbums to a column of List
        let Some(column_adj) = Self::subcolumns_of_vec().get(sort_command.column) else {
            return Err(Error::Other(format!(
                "Unable to sort column, doesn't match up with underlying list. {}",
                sort_command.column,
            )));
        };
        self.list.sort(*column_adj, sort_command.direction);
        // Naive as doesn't remove duplicates.
        self.sort_commands.push(sort_command);
        Ok(())
    }

    fn clear_sort_commands(&mut self) {
        self.sort_commands.clear();
    }
    fn get_sort_commands(&self) -> &[TableSortCommand] {
        &self.sort_commands
    }
}

fn search_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_from_code(KeyCode::Enter, BrowserAction::Artist(ArtistAction::Search)),
        Keybind::new_from_code(
            KeyCode::Down,
            BrowserAction::Artist(ArtistAction::NextSearchSuggestion),
        ),
        Keybind::new_from_code(
            KeyCode::Up,
            BrowserAction::Artist(ArtistAction::PrevSearchSuggestion),
        ),
    ]
}

fn sort_keybinds() -> Vec<Keybind<BrowserAction>> {
    // Consider a blocking type of keybind for this that stops all other commands being received.
    vec![
        Keybind::new_from_code(
            KeyCode::Char('1'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(0, SortDirection::Asc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('2'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(1, SortDirection::Asc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('3'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(2, SortDirection::Asc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('4'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(3, SortDirection::Asc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('5'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(4, SortDirection::Asc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('!'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(0, SortDirection::Desc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('@'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(1, SortDirection::Desc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('#'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(2, SortDirection::Desc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('$'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(3, SortDirection::Desc)),
        ),
        Keybind::new_from_code(
            KeyCode::Char('%'),
            BrowserAction::ArtistSongs(ArtistSongsAction::Sort(4, SortDirection::Desc)),
        ),
        Keybind::new_global_from_code(
            KeyCode::F(4),
            BrowserAction::ArtistSongs(ArtistSongsAction::CloseSort),
        ),
        Keybind::new_from_code(
            KeyCode::Char('C'),
            BrowserAction::ArtistSongs(ArtistSongsAction::ClearSort),
        ),
    ]
}

fn browser_artist_search_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_from_code(
            KeyCode::Enter,
            BrowserAction::Artist(ArtistAction::DisplayAlbums),
        ),
        // XXX: Consider if these type of actions can be for all lists.
        Keybind::new_from_code(KeyCode::Down, BrowserAction::Artist(ArtistAction::Down)),
        Keybind::new_from_code(KeyCode::Up, BrowserAction::Artist(ArtistAction::Up)),
        Keybind::new_from_code(KeyCode::PageUp, BrowserAction::Artist(ArtistAction::PageUp)),
        Keybind::new_from_code(
            KeyCode::PageDown,
            BrowserAction::Artist(ArtistAction::PageDown),
        ),
    ]
}

pub fn songs_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_global_from_code(
            KeyCode::F(4),
            BrowserAction::ArtistSongs(ArtistSongsAction::PopSort),
        ),
        Keybind::new_from_code(
            KeyCode::PageUp,
            BrowserAction::ArtistSongs(ArtistSongsAction::PageUp),
        ),
        Keybind::new_from_code(
            KeyCode::PageDown,
            BrowserAction::ArtistSongs(ArtistSongsAction::PageDown),
        ),
        Keybind::new_from_code(
            KeyCode::Down,
            BrowserAction::ArtistSongs(ArtistSongsAction::Down),
        ),
        Keybind::new_from_code(
            KeyCode::Up,
            BrowserAction::ArtistSongs(ArtistSongsAction::Up),
        ),
        Keybind::new_action_only_mode(
            vec![
                (
                    KeyCode::Enter,
                    BrowserAction::ArtistSongs(ArtistSongsAction::PlaySong),
                ),
                (
                    KeyCode::Char('p'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::PlaySongs),
                ),
                (
                    KeyCode::Char('a'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::PlayAlbum),
                ),
                (
                    KeyCode::Char(' '),
                    BrowserAction::ArtistSongs(ArtistSongsAction::AddSongToPlaylist),
                ),
                (
                    KeyCode::Char('P'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::AddSongsToPlaylist),
                ),
                (
                    KeyCode::Char('A'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::AddAlbumToPlaylist),
                ),
            ],
            KeyCode::Enter,
            "Play",
        ),
    ]
}
