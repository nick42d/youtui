mod artistalbums;
use crossterm::event::{KeyCode, KeyEvent};

use std::{borrow::Cow, mem};
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::parse::{SearchResultArtist, SongResult};

use crate::{app::ui::actionhandler::Keybind, core::send_or_error};

use self::{
    artistalbums::{AlbumSongsPanel, ArtistAction, ArtistSearchPanel, ArtistSongsAction},
    draw::draw_browser,
};

use super::{
    actionhandler::{
        Action, ActionHandler, EventHandler, KeyHandler, KeyRouter, Suggestable, TextHandler,
    },
    contextpane::ContextPane,
    structures::ListStatus,
    taskregister::TaskID,
    view::{Drawable, Scrollable},
    UIMessage, WindowContext,
};

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserAction {
    ToggleHelp,
    ViewPlaylist,
    Quit,
    ViewLogs,
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
    ui_tx: mpsc::Sender<UIMessage>,
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub artist_list: ArtistSearchPanel,
    pub album_songs_list: AlbumSongsPanel,
    keybinds: Vec<Keybind<BrowserAction>>,
    key_stack: Vec<KeyEvent>,
    help_shown: bool,
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
            Self::Quit => "Quit".into(),
            Self::ViewLogs => "View Logs".into(),
            Self::Left => "Left".into(),
            Self::Right => "Right".into(),
            Self::ViewPlaylist => "View Playlist".into(),
            Self::ToggleHelp => "Help".into(),
            Self::Artist(x) => x.describe(),
            Self::ArtistSongs(x) => x.describe(),
        }
    }
}
impl Suggestable for Browser {
    fn get_search_suggestions(&self) -> &[String] {
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
}

impl ContextPane<BrowserAction> for Browser {
    fn help_shown(&self) -> bool {
        self.help_shown
    }
    fn context_name(&self) -> std::borrow::Cow<'static, str> {
        "Browser".into()
    }
}
impl Drawable for Browser {
    fn draw_chunk<B: ratatui::prelude::Backend>(
        &self,
        f: &mut ratatui::Frame<B>,
        chunk: ratatui::prelude::Rect,
    ) {
        draw_browser(f, self, chunk);
    }
}
impl KeyRouter<BrowserAction> for Browser {
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<BrowserAction>> + 'a> {
        Box::new(
            self.keybinds
                .iter()
                .chain(self.artist_list.get_all_keybinds())
                .chain(self.album_songs_list.get_keybinds()),
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
impl EventHandler<BrowserAction> for Browser {
    fn get_mut_key_stack(&mut self) -> &mut Vec<KeyEvent> {
        &mut self.key_stack
    }
    fn get_key_stack(&self) -> &[KeyEvent] {
        &self.key_stack
    }
    fn get_global_sender(&self) -> &mpsc::Sender<UIMessage> {
        &self.ui_tx
    }
}
impl ActionHandler<ArtistAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistAction) {
        match action {
            ArtistAction::DisplayAlbums => self.get_songs().await,
            ArtistAction::ToggleSearch => self.artist_list.toggle_search(),
            ArtistAction::Search => self.search().await,
            ArtistAction::Up => self.artist_list.increment_list(-1),
            ArtistAction::Down => self.artist_list.increment_list(1),
            ArtistAction::PageUp => self.artist_list.increment_list(-10),
            ArtistAction::PageDown => self.artist_list.increment_list(10),
        }
    }
}
impl ActionHandler<ArtistSongsAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistSongsAction) {
        match action {
            ArtistSongsAction::PlayAlbum => self.play_album().await,
            ArtistSongsAction::AddAlbumToPlaylist => self.add_album_to_playlist().await,
            // XXX: This is incorrect as it actually plays all songs.
            ArtistSongsAction::PlaySong => self.play_songs().await,
            ArtistSongsAction::AddSongToPlaylist => self.add_to_playlist().await,
            ArtistSongsAction::PlaySongs => self.play_songs().await,
            ArtistSongsAction::AddSongsToPlaylist => self.add_all_to_playlist().await,
            ArtistSongsAction::Up => self.album_songs_list.increment_list(-1),
            ArtistSongsAction::Down => self.album_songs_list.increment_list(1),
            ArtistSongsAction::PageUp => self.album_songs_list.increment_list(-10),
            ArtistSongsAction::PageDown => self.album_songs_list.increment_list(10),
        }
    }
}
impl ActionHandler<BrowserAction> for Browser {
    async fn handle_action(&mut self, action: &BrowserAction) {
        match action {
            BrowserAction::ArtistSongs(a) => self.handle_action(a).await,
            BrowserAction::Artist(a) => self.handle_action(a).await,
            BrowserAction::Quit => send_or_error(&self.ui_tx, UIMessage::Quit).await,
            BrowserAction::ViewLogs => {
                send_or_error(&self.ui_tx, UIMessage::ChangeContext(WindowContext::Logs)).await
            }
            BrowserAction::Left => self.left(),
            BrowserAction::Right => self.right(),
            BrowserAction::ViewPlaylist => {
                send_or_error(
                    &self.ui_tx,
                    UIMessage::ChangeContext(WindowContext::Playlist),
                )
                .await
            }
            BrowserAction::ToggleHelp => self.help_shown = !self.help_shown,
        }
    }

    // KeyCode::PageUp => self.handle_pgup_pressed().await,
    // KeyCode::PageDown => self.handle_pgdown_pressed().await,
    // KeyCode::F(3) => self.artist_list.push_sort_command("test".to_owned()),
    // KeyCode::F(5) => self
    //     .ui_tx
    //     .send(UIMessage::ChangeContext(WindowContext::Playlist))
    //     .await
    //     .unwrap_or_else(|e| error!("Error {e} sending message.")),
}
impl Browser {
    pub fn new(ui_tx: mpsc::Sender<UIMessage>) -> Self {
        Self {
            ui_tx,
            artist_list: ArtistSearchPanel::new(),
            album_songs_list: AlbumSongsPanel::new(),
            input_routing: InputRouting::Artist,
            prev_input_routing: InputRouting::Artist,
            keybinds: browser_keybinds(),
            key_stack: Vec::new(),
            help_shown: false,
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
    // Ask the UI for search suggestions for the current query
    // XXX: Currently has race conditions - if list is cleared response will arrive afterwards.
    // Proposal: When recieving a message from the app validate against query string.
    fn fetch_search_suggestions(&mut self) {
        // No need to fetch search suggestions
        if self.artist_list.search_contents.is_empty() {
            self.artist_list.search_suggestions.clear();
            return;
        }
        if let Err(e) = self.ui_tx.try_send(UIMessage::GetSearchSuggestions(
            self.artist_list.search_contents.clone(),
        )) {
            error!("Error <{e}> recieved sending message")
        };
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
        send_or_error(&self.ui_tx, UIMessage::PlaySongs(song_list)).await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_all_to_playlist(&mut self) {
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
        send_or_error(&self.ui_tx, UIMessage::AddSongsToPlaylist(song_list)).await;
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
        send_or_error(&self.ui_tx, UIMessage::AddSongsToPlaylist(song_list)).await;
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
        send_or_error(&self.ui_tx, UIMessage::PlaySongs(song_list)).await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_to_playlist(&mut self) {
        let Some(cur_index) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let Some(cur_song) = self.album_songs_list.list.list.get(cur_index) else {
            error!("Tried to get item from list with index out of range");
            return;
        };
        send_or_error(
            &self.ui_tx,
            UIMessage::AddSongsToPlaylist(vec![cur_song.clone()]),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn add_to_playlist_and_play(&mut self) {
        let Some(cur_index) = self.album_songs_list.list.cur_selected else {
            return;
        };
        let Some(cur_song) = self.album_songs_list.list.list.get(cur_index) else {
            error!("Tried to get item from list with index out of range");
            return;
        };
        send_or_error(&self.ui_tx, UIMessage::PlaySongs(vec![cur_song.clone()])).await;
        // XXX: Do we want to indicate that song has been added to playlist?
        // let id = self.playlist.push_clone_listsong(&clone_song);
        // self.playlist.play_song_id(id).await;
    }
    async fn get_songs(&mut self) {
        let Some(selected) = Some(self.artist_list.get_selected_item()) else {
            return;
        };
        self.change_routing(InputRouting::Song);
        send_or_error(&self.ui_tx, UIMessage::KillPendingGetTasks).await;
        tracing::info!("Sent request to UI to kill pending get tasks");
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
        send_or_error(&self.ui_tx, UIMessage::GetArtistSongs(cur_artist_id)).await;
        tracing::info!("Sent request to UI to get songs");
    }
    async fn search(&mut self) {
        self.artist_list.close_search();
        send_or_error(&self.ui_tx, UIMessage::KillPendingSearchTasks).await;
        tracing::info!("Sent request to UI to kill pending search tasks");
        let search_query = std::mem::take(&mut self.artist_list.search_contents);
        send_or_error(&self.ui_tx, UIMessage::SearchArtist(search_query)).await;
        tracing::info!("Sent request to UI to search");
    }
    pub fn handle_search_artist_error(&mut self, _id: TaskID) {
        self.album_songs_list.list.state = ListStatus::Error;
    }
    pub fn handle_song_list_loaded(&mut self, _id: TaskID) {
        self.album_songs_list.list.state = ListStatus::Loaded;
    }
    pub fn handle_song_list_loading(&mut self, _id: TaskID) {
        self.album_songs_list.list.state = ListStatus::Loading;
    }
    pub async fn handle_replace_artist_list(
        &mut self,
        artist_list: Vec<SearchResultArtist>,
        _id: TaskID,
    ) {
        self.artist_list.list = artist_list;
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0).await;
    }
    pub async fn handle_replace_search_suggestions(
        &mut self,
        search_suggestions: Vec<String>,
        _id: TaskID,
    ) {
        self.artist_list.search_suggestions = search_suggestions;
    }
    pub fn handle_no_songs_found(&mut self, _id: TaskID) {
        self.album_songs_list.list.state = ListStatus::Loaded;
        self.album_songs_list.list.list.clear()
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        _id: TaskID,
    ) {
        self.album_songs_list
            .list
            .append_raw_songs(song_list, album, year);
        self.album_songs_list.list.state = ListStatus::InProgress;
    }
    pub fn handle_songs_found(&mut self, _id: TaskID) {
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
    #[deprecated]
    pub fn change_routing(&mut self, input_routing: InputRouting) {
        self.prev_input_routing = mem::replace(&mut self.input_routing, input_routing);
    }
}

fn browser_keybinds() -> Vec<Keybind<BrowserAction>> {
    vec![
        Keybind::new_global_from_code(KeyCode::F(1), BrowserAction::ToggleHelp),
        Keybind::new_global_from_code(KeyCode::F(5), BrowserAction::ViewPlaylist),
        Keybind::new_global_from_code(KeyCode::F(10), BrowserAction::Quit),
        Keybind::new_global_from_code(KeyCode::F(12), BrowserAction::ViewLogs),
        Keybind::new_from_code(KeyCode::Left, BrowserAction::Left),
        Keybind::new_from_code(KeyCode::Right, BrowserAction::Right),
    ]
}

pub mod draw {
    use ratatui::{
        prelude::{Backend, Constraint, Direction, Layout, Rect},
        style::{Color, Style},
        symbols::block,
        widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
        Frame,
    };

    use crate::app::ui::{
        actionhandler::Suggestable,
        view::draw::{draw_list, draw_table},
    };

    use super::{artistalbums::ArtistInputRouting, Browser, InputRouting};

    pub fn draw_browser<B>(f: &mut Frame<B>, browser: &Browser, chunk: Rect)
    where
        B: Backend,
    {
        let layout = Layout::new()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .direction(ratatui::prelude::Direction::Horizontal)
            .split(chunk);
        // XXX: Naive implementation.
        let _albumsongsselected = browser.input_routing == InputRouting::Song;
        let _artistselected =
            !_albumsongsselected && browser.artist_list.route == ArtistInputRouting::List;

        if !browser.artist_list.search_popped {
            draw_list(f, &browser.artist_list, layout[0], _artistselected);
        } else {
            let s = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(layout[0]);
            let search_widget = Paragraph::new(browser.artist_list.search_contents.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title("Search"),
            );
            f.render_widget(search_widget, s[0]);
            draw_list(f, &browser.artist_list, s[1], _artistselected);
            if browser.has_search_suggestions() {
                let suggestions = browser.get_search_suggestions();
                let height = suggestions.len() + 1;
                let width = (suggestions.iter().fold(0, |acc, s| s.len().max(acc)) + 2)
                    .min(s[0].width as usize);
                let area = below_left_rect(
                    height.try_into().unwrap_or(u16::MAX),
                    width.try_into().unwrap_or(u16::MAX),
                    s[0],
                );
                let list: Vec<_> = suggestions
                    .into_iter()
                    .map(|s| ListItem::new(s.as_str()))
                    .collect();
                let block = List::new(list).style(Style::new().fg(Color::White)).block(
                    Block::default()
                        .borders(Borders::all().difference(Borders::TOP))
                        .style(Style::new().fg(Color::Cyan)),
                );
                f.render_widget(Clear, area);
                f.render_widget(block, area);
            }
        }
        draw_table(f, &browser.album_songs_list, layout[1], _albumsongsselected);
    }
    /// Helper function to create a popup below a chunk.
    pub fn below_left_rect(height: u16, width: u16, r: Rect) -> Rect {
        Rect {
            x: r.x,
            y: r.y + r.height,
            width,
            height,
        }
    }
}
