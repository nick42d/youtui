use super::get_adjusted_list_column;
use crate::app::component::actionhandler::DominantKeyRouter;
use crate::app::ui::browser::BrowserAction;
use crate::app::view::{SortDirection, SortableTableView, TableSortCommand};
use crate::app::{
    component::actionhandler::{Action, KeyRouter},
    keycommand::KeyCommand,
    structures::{AlbumSongsList, ListStatus, Percentage},
    view::{BasicConstraint, Loadable, Scrollable, TableView},
};
use crate::error::Error;
use crate::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use std::borrow::Cow;
use tracing::warn;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AlbumSongsInputRouting {
    Sort,
    #[default]
    List,
}

#[derive(Clone)]
pub struct AlbumSongsPanel {
    pub list: AlbumSongsList,
    keybinds: Vec<KeyCommand<BrowserAction>>,
    pub route: AlbumSongsInputRouting,
    pub sort: SortManager,
}

#[derive(Clone)]
pub struct SortManager {
    sort_commands: Vec<TableSortCommand>,
    pub shown: bool,
    pub cur: usize,
    keybinds: Vec<KeyCommand<BrowserAction>>,
}

impl Default for SortManager {
    fn default() -> Self {
        Self {
            sort_commands: Default::default(),
            shown: Default::default(),
            cur: Default::default(),
            keybinds: sort_keybinds(),
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
    PopSort,
    SortUp,
    SortDown,
    CloseSort,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}

impl AlbumSongsPanel {
    pub fn new() -> AlbumSongsPanel {
        AlbumSongsPanel {
            keybinds: songs_keybinds(),
            list: Default::default(),
            route: Default::default(),
            sort: Default::default(),
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
    fn open_sort(&mut self) {
        self.sort.shown = true;
        self.route = AlbumSongsInputRouting::Sort;
    }
    pub fn close_sort(&mut self) {
        self.sort.shown = false;
        self.route = AlbumSongsInputRouting::List;
    }
    pub fn handle_pop_sort(&mut self) {
        // If no sortable columns, should we not handle this?
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
            Self::Up | Self::SortUp => "Up",
            Self::Down | Self::SortDown => "Down",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
            Self::PopSort => "Sort",
            Self::CloseSort => "Close sort",
            Self::ClearSort => "Clear sort",
            Self::SortSelectedAsc => "Sort ascending",
            Self::SortSelectedDesc => "Sort descending",
        }
        .into()
    }
}

impl DominantKeyRouter for AlbumSongsPanel {
    fn dominant_keybinds_active(&self) -> bool {
        self.sort.shown
    }
}

impl KeyRouter<BrowserAction> for AlbumSongsPanel {
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        Box::new(self.keybinds.iter().chain(self.sort.keybinds.iter()))
    }
    fn get_routed_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        Box::new(match self.route {
            AlbumSongsInputRouting::List => self.keybinds.iter(),
            AlbumSongsInputRouting::Sort => self.sort.keybinds.iter(),
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
        // Remove commands that already exist for the same column, as this new command will trump the old ones.
        // Slightly naive - loops the whole vec, could short circuit.
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
}

fn sort_keybinds() -> Vec<KeyCommand<BrowserAction>> {
    // Consider a blocking type of keybind for this that stops all other commands being received.
    vec![
        KeyCommand::new_global_from_code(
            KeyCode::F(4),
            BrowserAction::ArtistSongs(ArtistSongsAction::CloseSort),
        ),
        KeyCommand::new_global_from_code(
            KeyCode::Enter,
            BrowserAction::ArtistSongs(ArtistSongsAction::SortSelectedAsc),
        ),
        // Seems to not work on Windows.
        KeyCommand::new_global_modified_from_code(
            KeyCode::Enter,
            KeyModifiers::ALT,
            BrowserAction::ArtistSongs(ArtistSongsAction::SortSelectedDesc),
        ),
        KeyCommand::new_global_from_code(
            KeyCode::Char('C'),
            BrowserAction::ArtistSongs(ArtistSongsAction::ClearSort),
        ),
        KeyCommand::new_hidden_from_code(
            KeyCode::Esc,
            BrowserAction::ArtistSongs(ArtistSongsAction::CloseSort),
        ),
        // XXX: Consider if these type of actions can be for all lists.
        KeyCommand::new_hidden_from_code(
            KeyCode::Down,
            BrowserAction::ArtistSongs(ArtistSongsAction::SortDown),
        ),
        KeyCommand::new_hidden_from_code(
            KeyCode::Up,
            BrowserAction::ArtistSongs(ArtistSongsAction::SortUp),
        ),
    ]
}

pub fn songs_keybinds() -> Vec<KeyCommand<BrowserAction>> {
    vec![
        KeyCommand::new_global_from_code(
            KeyCode::F(4),
            BrowserAction::ArtistSongs(ArtistSongsAction::PopSort),
        ),
        KeyCommand::new_from_code(
            KeyCode::PageUp,
            BrowserAction::ArtistSongs(ArtistSongsAction::PageUp),
        ),
        KeyCommand::new_from_code(
            KeyCode::PageDown,
            BrowserAction::ArtistSongs(ArtistSongsAction::PageDown),
        ),
        KeyCommand::new_hidden_from_code(
            KeyCode::Down,
            BrowserAction::ArtistSongs(ArtistSongsAction::Down),
        ),
        KeyCommand::new_hidden_from_code(
            KeyCode::Up,
            BrowserAction::ArtistSongs(ArtistSongsAction::Up),
        ),
        KeyCommand::new_action_only_mode(
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
