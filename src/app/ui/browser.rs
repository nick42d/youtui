use self::{
    artistalbums::{AlbumSongsPanel, ArtistAction, ArtistSearchPanel, ArtistSongsAction},
    draw::draw_browser,
};
use super::{AppCallback, WindowContext, YoutuiMutableState};
use crate::app::{
    component::actionhandler::{
        Action, ActionHandler, ActionProcessor, KeyHandler, KeyRouter, Suggestable, TextHandler,
    },
    structures::ListStatus,
    view::{
        DrawableMut, Scrollable, SortDirection, SortableTableView, TableSortCommand, TableView,
    },
};
use crate::{app::component::actionhandler::Keybind, core::send_or_error};
use crossterm::event::KeyCode;
use std::{borrow::Cow, mem};
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::{
    common::SearchSuggestion,
    parse::{SearchResultArtist, SongResult},
};

const PAGE_KEY_LINES: isize = 10;

mod artistalbums;
mod draw;

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserAction {
    ViewPlaylist,
    ToggleSearch,
    Left,
    Right,
    Artist(ArtistAction),
    ArtistSongs(ArtistSongsAction),
}

#[derive(PartialEq)]
pub enum InputRouting {
    Artist,
    Song,
}

pub struct Browser {
    ui_tx: mpsc::Sender<AppCallback>,
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub artist_list: ArtistSearchPanel,
    pub album_songs_list: AlbumSongsPanel,
    keybinds: Vec<Keybind<BrowserAction>>,
}

impl InputRouting {
    pub fn left(&self) -> Self {
        match self {
            Self::Song => Self::Artist,
            Self::Artist => Self::Artist,
        }
    }
    pub fn right(&self) -> Self {
        match self {
            Self::Artist => Self::Song,
            Self::Song => Self::Song,
        }
    }
}
impl Action for BrowserAction {
    fn context(&self) -> Cow<str> {
        let context = "Browser";
        match self {
            Self::Artist(a) => format!("{context}->{}", a.context()).into(),
            Self::ArtistSongs(a) => format!("{context}->{}", a.context()).into(),
            _ => context.into(),
        }
    }
    fn describe(&self) -> Cow<str> {
        match self {
            Self::Left => "Left".into(),
            Self::Right => "Right".into(),
            Self::ViewPlaylist => "View Playlist".into(),
            Self::ToggleSearch => "Toggle Search".into(),
            Self::Artist(x) => x.describe(),
            Self::ArtistSongs(x) => x.describe(),
        }
    }
}
// Should this really be implemented on the Browser...
impl Suggestable for Browser {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.get_search_suggestions(),
            InputRouting::Song => &[],
        }
    }
    fn has_search_suggestions(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.has_search_suggestions(),
            InputRouting::Song => false,
        }
    }
}
impl TextHandler for Browser {
    fn push_text(&mut self, c: char) {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.push_text(c),
            InputRouting::Song => (),
        }
        self.fetch_search_suggestions();
    }
    fn pop_text(&mut self) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_list.pop_text();
            }
            InputRouting::Song => (),
        }
        self.fetch_search_suggestions();
    }
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.is_text_handling(),
            InputRouting::Song => false,
        }
    }
    fn take_text(&mut self) -> String {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.take_text(),
            InputRouting::Song => Default::default(),
        }
    }
    fn replace_text(&mut self, text: String) {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.replace_text(text),
            InputRouting::Song => (),
        }
    }
}

impl DrawableMut for Browser {
    fn draw_mut_chunk(
        &self,
        f: &mut ratatui::Frame,
        chunk: ratatui::prelude::Rect,
        mutable_state: &mut YoutuiMutableState,
    ) {
        draw_browser(
            f,
            self,
            chunk,
            &mut mutable_state.browser_artists,
            &mut mutable_state.browser_album_songs,
        );
    }
}
impl KeyRouter<BrowserAction> for Browser {
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(
            self.keybinds
                .iter()
                .chain(self.artist_list.get_all_keybinds())
                .chain(self.album_songs_list.get_all_keybinds()),
        )
    }
}
impl KeyHandler<BrowserAction> for Browser {
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        let additional_binds = match self.input_routing {
            InputRouting::Song => Some(self.album_songs_list.get_keybinds()),
            InputRouting::Artist => Some(self.artist_list.get_keybinds()),
        }
        .into_iter()
        .flatten();
        Box::new(self.keybinds.iter().chain(additional_binds))
    }
}
impl ActionProcessor<BrowserAction> for Browser {}
impl ActionHandler<ArtistAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistAction) {
        match action {
            ArtistAction::DisplayAlbums => self.get_songs().await,
            ArtistAction::Search => self.search().await,
            ArtistAction::Up => self.artist_list.increment_list(-1),
            ArtistAction::Down => self.artist_list.increment_list(1),
            ArtistAction::PageUp => self.artist_list.increment_list(-10),
            ArtistAction::PageDown => self.artist_list.increment_list(10),
            ArtistAction::PrevSearchSuggestion => self.artist_list.search.increment_list(-1),
            ArtistAction::NextSearchSuggestion => self.artist_list.search.increment_list(1),
        }
    }
}
impl ActionHandler<ArtistSongsAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistSongsAction) {
        match action {
            ArtistSongsAction::PlayAlbum => self.play_album().await,
            ArtistSongsAction::PlaySong => self.play_song().await,
            ArtistSongsAction::PlaySongs => self.play_songs().await,
            ArtistSongsAction::AddAlbumToPlaylist => self.add_album_to_playlist().await,
            ArtistSongsAction::AddSongToPlaylist => self.add_song_to_playlist().await,
            ArtistSongsAction::AddSongsToPlaylist => self.add_songs_to_playlist().await,
            ArtistSongsAction::Up => self.album_songs_list.increment_list(-1),
            ArtistSongsAction::Down => self.album_songs_list.increment_list(1),
            ArtistSongsAction::PageUp => self.album_songs_list.increment_list(-PAGE_KEY_LINES),
            ArtistSongsAction::PageDown => self.album_songs_list.increment_list(PAGE_KEY_LINES),
            ArtistSongsAction::PopSort => self.album_songs_list.open_sort(),
            ArtistSongsAction::CloseSort => self.album_songs_list.close_sort(),
            ArtistSongsAction::ClearSort => {
                self.album_songs_list.close_sort();
                self.album_songs_list.clear_sort_commands();
            }
            ArtistSongsAction::Sort(column, direction) => {
                // TODO: Error handling
                let _ = self.album_songs_list.push_sort_command(TableSortCommand {
                    column: *column,
                    direction: *direction,
                });
                self.album_songs_list.close_sort();
            }
        }
    }
}
impl ActionHandler<BrowserAction> for Browser {
    async fn handle_action(&mut self, action: &BrowserAction) {
        match action {
            BrowserAction::ArtistSongs(a) => self.handle_action(a).await,
            BrowserAction::Artist(a) => self.handle_action(a).await,
            BrowserAction::Left => self.left(),
            BrowserAction::Right => self.right(),
            BrowserAction::ViewPlaylist => {
                send_or_error(
                    &self.ui_tx,
                    AppCallback::ChangeContext(WindowContext::Playlist),
                )
                .await
            }
            // TODO: fix routing changes etc
            BrowserAction::ToggleSearch => self.handle_toggle_search(),
        }
    }
    // KeyCode::F(3) => self.artist_list.push_sort_command("test".to_owned()),
}
impl Browser {
    pub fn new(ui_tx: mpsc::Sender<AppCallback>) -> Self {
        Self {
            ui_tx,
            artist_list: ArtistSearchPanel::new(),
            album_songs_list: AlbumSongsPanel::new(),
            input_routing: InputRouting::Artist,
            prev_input_routing: InputRouting::Artist,
            keybinds: browser_keybinds(),
        }
    }
    fn left(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.left();
    }
    fn right(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.right();
    }
    fn handle_toggle_search(&mut self) {
        if self.artist_list.search_popped {
            self.artist_list.close_search();
            self.revert_routing();
        } else {
            self.artist_list.open_search();
            self.change_routing(InputRouting::Artist);
        }
    }
    // Ask the UI for search suggestions for the current query
    // XXX: Currently has race conditions - if list is cleared response will arrive afterwards.
    // Proposal: When recieving a message from the app validate against query string.
    fn fetch_search_suggestions(&mut self) {
        // No need to fetch search suggestions if contents is empty.
        if self.artist_list.search.search_contents.is_empty() {
            self.artist_list.search.search_suggestions.clear();
            return;
        }
        if let Err(e) = self.ui_tx.try_send(AppCallback::GetSearchSuggestions(
            self.artist_list.search.search_contents.clone(),
        )) {
            error!("Error <{e}> recieved sending message")
        };
    }
    async fn play_song(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_song_idx) = self.album_songs_list.list.cur_selected else {
            return;
        };
        if let Some(cur_song) = self.album_songs_list.list.list.get(cur_song_idx) {
            send_or_error(
                &self.ui_tx,
                AppCallback::AddSongsToPlaylistAndPlay(vec![cur_song.clone()]),
            )
            .await;
        }
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn play_songs(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_song) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            .list
            .iter()
            .skip(cur_song)
            .cloned()
            .collect();
        send_or_error(
            &self.ui_tx,
            AppCallback::AddSongsToPlaylistAndPlay(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_songs_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_song) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            .list
            .iter()
            .skip(cur_song)
            .cloned()
            .collect();
        send_or_error(&self.ui_tx, AppCallback::AddSongsToPlaylist(song_list)).await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_song_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_song_idx) = self.album_songs_list.list.cur_selected else {
            return;
        };
        if let Some(cur_song) = self.album_songs_list.list.list.get(cur_song_idx) {
            send_or_error(
                &self.ui_tx,
                AppCallback::AddSongsToPlaylist(vec![cur_song.clone()]),
            )
            .await;
        }
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_album_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_index) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let Some(cur_song) = self.album_songs_list.list.list.get(cur_index) else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            .list
            .iter()
            .filter(|song| song.get_album() == cur_song.get_album())
            .cloned()
            .collect();
        send_or_error(&self.ui_tx, AppCallback::AddSongsToPlaylist(song_list)).await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn play_album(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let Some(cur_index) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let Some(cur_song) = self.album_songs_list.list.list.get(cur_index) else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            .list
            .iter()
            .filter(|song| song.get_album() == cur_song.get_album())
            // XXX: Could instead be inside an Rc.
            .cloned()
            .collect();
        send_or_error(
            &self.ui_tx,
            AppCallback::AddSongsToPlaylistAndPlay(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn get_songs(&mut self) {
        let selected = self.artist_list.get_selected_item();
        self.change_routing(InputRouting::Song);
        self.album_songs_list.list.list.clear();

        let Some(cur_artist_id) = self
            .artist_list
            .list
            .get(selected)
            .cloned()
            .and_then(|a| a.browse_id)
        else {
            error!("Tried to get item from list with index out of range");
            return;
        };
        send_or_error(&self.ui_tx, AppCallback::GetArtistSongs(cur_artist_id)).await;
        tracing::info!("Sent request to UI to get songs");
    }
    async fn search(&mut self) {
        self.artist_list.close_search();
        let search_query = self.artist_list.search.take_text();
        send_or_error(&self.ui_tx, AppCallback::SearchArtist(search_query)).await;
        tracing::info!("Sent request to UI to search");
    }
    pub fn handle_search_artist_error(&mut self) {
        self.album_songs_list.list.state = ListStatus::Error;
    }
    pub fn handle_song_list_loaded(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loaded;
    }
    pub fn handle_song_list_loading(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loading;
    }
    pub async fn handle_replace_artist_list(&mut self, artist_list: Vec<SearchResultArtist>) {
        self.artist_list.list = artist_list;
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0).await;
    }
    pub fn handle_replace_search_suggestions(
        &mut self,
        search_suggestions: Vec<SearchSuggestion>,
        search: String,
    ) {
        if self.artist_list.search.search_contents == search {
            self.artist_list.search.search_suggestions = search_suggestions;
            self.artist_list.search.suggestions_cur = None;
        }
    }
    pub fn handle_no_songs_found(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loaded;
        self.album_songs_list.list.list.clear()
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
    ) {
        self.album_songs_list
            .list
            .append_raw_songs(song_list, album, year, artist);
        self.album_songs_list.list.state = ListStatus::InProgress;
    }
    pub fn handle_songs_found(&mut self) {
        self.album_songs_list.list.list.clear();
        self.album_songs_list.list.cur_selected = Some(0);
        self.album_songs_list.list.state = ListStatus::InProgress;
    }
    pub async fn increment_cur_list(&mut self, increment: isize) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_list.increment_list(increment);
            }
            InputRouting::Song => {
                self.album_songs_list.increment_list(increment);
            }
        };
    }
    #[deprecated]
    pub fn revert_routing(&mut self) {
        mem::swap(&mut self.input_routing, &mut self.prev_input_routing);
    }
    // Could be in trait.
    #[deprecated = "Should be in a trait"]
    pub fn change_routing(&mut self, input_routing: InputRouting) {
        self.prev_input_routing = mem::replace(&mut self.input_routing, input_routing);
    }
}

fn browser_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_global_from_code(KeyCode::F(5), BrowserAction::ViewPlaylist),
        Keybind::new_global_from_code(KeyCode::F(2), BrowserAction::ToggleSearch),
        Keybind::new_from_code(KeyCode::Left, BrowserAction::Left),
        Keybind::new_from_code(KeyCode::Right, BrowserAction::Right),
    ]
}
