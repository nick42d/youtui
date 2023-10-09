use std::borrow::Cow;

use crossterm::event::KeyCode;
use ratatui::prelude::Constraint;
use ytmapi_rs::parse::SearchResultArtist;

use crate::app::ui::{
    actionhandler::{Action, KeyHandler, KeyRouter, Keybind, TextHandler},
    browser::BrowserAction,
    structures::{AlbumSongsList, ListSong, ListStatus},
    view::{Drawable, ListView, Loadable, Scrollable, SortableList, TableView},
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
    pub route: ArtistInputRouting,
    pub selected: usize,
    pub sort_commands_list: Vec<String>,
    keybinds: Vec<Keybind<BrowserAction>>,
    search_keybinds: Vec<Keybind<BrowserAction>>,
    pub search_popped: bool,
    pub search_contents: String,
}

#[derive(Default, Clone)]
pub struct AlbumSongsPanel {
    pub list: AlbumSongsList,
    keybinds: Vec<Keybind<BrowserAction>>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ArtistAction {
    DisplayAlbums,
    ToggleSearch,
    Search,
    // XXX: This could be a subset - eg ListAction
    Up,
    Down,
    PageUp,
    PageDown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArtistSongsAction {
    PlaySong,
    PlayAlbum,
    AddSongToPlaylist,
    AddAlbumToPlaylist,
    Up,
    Down,
    PageUp,
    PageDown,
}

impl ArtistSearchPanel {
    pub fn new() -> Self {
        Self {
            keybinds: browser_artist_search_keybinds(),
            search_keybinds: search_keybinds(),
            ..Default::default()
        }
    }
    // Workaround as removed Selectable trait.
    // XXX: What actually is a panel here? I can't select ArtistSearchPanel as it contains multiple components
    fn is_selected(&self) -> bool {
        true
    }
    pub fn toggle_search(&mut self) {
        if !self.search_popped {
            self.open_search()
        } else {
            self.close_search()
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
            ..Default::default()
        }
    }
}

impl Action for ArtistAction {
    fn context(&self) -> Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> Cow<str> {
        match &self {
            Self::Search => "Search",
            Self::ToggleSearch => "Toggle search",
            Self::DisplayAlbums => "Display albums for selected artist",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
        }
        .into()
    }
}

impl TextHandler for ArtistSearchPanel {
    fn push_text(&mut self, c: char) {
        self.search_contents.push(c);
    }
    fn pop_text(&mut self) {
        self.search_contents.pop();
    }
    fn is_text_handling(&self) -> bool {
        self.route == ArtistInputRouting::Search
    }
}

impl Action for ArtistSongsAction {
    fn context(&self) -> Cow<str> {
        "Artist Songs Panel".into()
    }
    fn describe(&self) -> Cow<str> {
        match &self {
            Self::PlaySong => "Play song",
            Self::PlayAlbum => "Play album",
            Self::AddSongToPlaylist => "Add song to playlist",
            Self::AddAlbumToPlaylist => "Add album to playlist",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
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
    fn get_selected_item(&self) -> usize {
        self.selected
    }
    fn increment_list(&mut self, amount: isize) {
        self.selected = self
            .selected
            .checked_add_signed(amount)
            .unwrap_or(0)
            .min(self.len().checked_add_signed(-1).unwrap_or(0));
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

impl KeyHandler<BrowserAction> for AlbumSongsPanel {
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(self.keybinds.iter())
    }
}

impl Loadable for AlbumSongsPanel {
    fn is_loading(&self) -> bool {
        match self.list.state {
            crate::app::ui::structures::ListStatus::Loading => true,
            _ => false,
        }
    }
}
impl Scrollable for AlbumSongsPanel {
    fn get_selected_item(&self) -> usize {
        self.list.get_selected_item()
    }
    fn increment_list(&mut self, amount: isize) {
        self.list.increment_list(amount)
    }
}

// XXX: This is an argument for not making a TableView drawable
// - as struct could contain multiple "drawable" panes, but then only have one draw_chunk method.
impl Drawable for AlbumSongsPanel {
    fn draw_chunk<B: ratatui::prelude::Backend>(
        &self,
        _f: &mut ratatui::Frame<B>,
        _chunk: ratatui::prelude::Rect,
    ) {
        todo!()
    }
}

impl TableView for AlbumSongsPanel {
    type Item = ListSong;
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
    fn get_layout(&self) -> Vec<ratatui::prelude::Constraint> {
        vec![
            Constraint::Min(6),
            Constraint::Min(3),
            Constraint::Max(30),
            Constraint::Max(30),
            Constraint::Min(9),
            Constraint::Min(4),
        ]
    }
    fn get_items(&self) -> Vec<&Self::Item> {
        self.list.list.iter().collect()
    }
    fn get_headings(&self) -> Vec<&'static str> {
        vec!["", "#", "Album", "Song", "Duration", "Year"]
    }
}

fn search_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![Keybind::new_from_code(
        KeyCode::Enter,
        BrowserAction::Artist(ArtistAction::Search),
    )]
}

fn browser_artist_search_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_global_from_code(
            KeyCode::F(2),
            BrowserAction::Artist(ArtistAction::ToggleSearch),
        ),
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
                    KeyCode::Char('p'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::PlaySong),
                ),
                (
                    KeyCode::Char('a'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::PlayAlbum),
                ),
                (
                    KeyCode::Char('P'),
                    BrowserAction::ArtistSongs(ArtistSongsAction::AddSongToPlaylist),
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
