mod actionhandler;
mod browser;
mod contextpane;
mod footer;
mod header;
mod help;
mod logger;
mod messagehandler;
mod playlist;
pub mod structures;
mod view;
// Public due to task register
pub mod taskregister;

use std::rc::Rc;

use crate::core::send_or_error;

use self::actionhandler::{
    Action, ActionHandler, KeyHandleOutcome, KeyHandler, Keybind, Keymap, TextHandler,
};
use self::browser::BrowserAction;
use self::contextpane::ContextPane;
use self::playlist::PlaylistAction;
use self::{
    actionhandler::ActionProcessor,
    browser::Browser,
    logger::Logger,
    playlist::Playlist,
    taskregister::{AppRequest, TaskID},
};

use super::server::{self, SongProgressUpdateType};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Row, Table};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    terminal::Frame,
};
use structures::*;
use taskregister::TaskRegister;
use tokio::sync::mpsc::{self, Sender};
use tracing::error;
use ytmapi_rs::{
    parse::{SearchResultArtist, SongResult},
    ChannelID, VideoID,
};

const PAGE_KEY_SCROLL_AMOUNT: isize = 10;
const CHANNEL_SIZE: usize = 256;

#[deprecated]
pub struct BasicCommand {
    key: KeyCode,
    name: String,
}
#[derive(PartialEq)]
pub enum AppStatus {
    Running,
    Exiting,
}

// Which app level keyboard shortcuts function.
// What is displayed in header
// The main pane of the application
// XXX: This is a bit like a route.
pub enum WindowContext {
    Browser,
    Playlist,
    Logs,
}

// A callback from one of the application components to the top level.
pub enum UIMessage {
    DownloadSong(VideoID<'static>, ListSongID),
    Quit,
    ChangeContext(WindowContext),
    Next,
    Prev,
    StepVolUp,
    StepVolDown,
    SearchArtist(String),
    GetSearchSuggestions(String),
    GetArtistSongs(ChannelID<'static>),
    KillPendingSearchTasks,
    KillPendingGetTasks,
    AddSongsToPlaylist(Vec<ListSong>),
    PlaySongs(Vec<ListSong>),
}
#[derive(Clone, Debug, PartialEq)]
pub enum UIAction {
    Quit,
    Next,
    Prev,
    StepVolUp,
    StepVolDown,
    Browser(BrowserAction),
    Playlist(PlaylistAction),
}

pub struct YoutuiWindow {
    pub status: AppStatus,
    context: WindowContext,
    prev_context: WindowContext,
    playlist: Playlist,
    browser: Browser,
    tasks: TaskRegister,
    logger: Logger,
    _ui_tx: mpsc::Sender<UIMessage>,
    ui_rx: mpsc::Receiver<UIMessage>,
    keybinds: Vec<Keybind<UIAction>>,
    key_stack: Vec<KeyEvent>,
    help_shown: bool,
}

impl KeyHandler<UIAction> for YoutuiWindow {
    // XXX: Need to determine how this should really be implemented.
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<UIAction>> + 'a> {
        Box::new(self.keybinds.iter())
    }
}

impl Action for Box<(dyn Action + 'static)> {
    fn context(&self) -> std::borrow::Cow<str> {
        todo!()
    }

    fn describe(&self) -> std::borrow::Cow<str> {
        todo!()
    }
}
impl YoutuiWindow {
    fn get_cur_mode<'a>(&'a self) -> Option<Box<dyn Iterator<Item = (String, String)> + 'a>> {
        if let Some(map) = self.get_key_subset(&self.key_stack) {
            if let Keymap::Mode(mode) = map {
                return Some(Box::new(
                    mode.key_binds
                        .iter()
                        // TODO: Remove allocation
                        .map(|bind| (bind.to_string(), bind.describe().to_string())),
                ));
            }
        }
        match self.context {
            WindowContext::Browser => {
                if let Some(map) = self.browser.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(Box::new(
                            mode.key_binds
                                .iter()
                                // TODO: Remove allocation
                                .map(|bind| (bind.to_string(), bind.describe().to_string())),
                        ));
                    }
                }
            }
            WindowContext::Playlist => todo!(),
            WindowContext::Logs => todo!(),
        }

        None
    }
}

impl ActionProcessor<UIAction> for YoutuiWindow {}

fn global_keybinds() -> Vec<Keybind<UIAction>> {
    vec![
        Keybind::new_from_code(KeyCode::Char('+'), UIAction::StepVolUp),
        Keybind::new_from_code(KeyCode::Char('-'), UIAction::StepVolDown),
        Keybind::new_from_code(KeyCode::Char('<'), UIAction::Prev),
        Keybind::new_from_code(KeyCode::Char('>'), UIAction::Next),
        Keybind::new_global_from_code(KeyCode::F(10), UIAction::Quit),
    ]
}

impl ActionHandler<UIAction> for YoutuiWindow {
    async fn handle_action(&mut self, action: &UIAction) {
        match action {
            UIAction::Next => todo!(),
            UIAction::Prev => todo!(),
            UIAction::StepVolUp => todo!(),
            UIAction::StepVolDown => todo!(),
            UIAction::Browser(b) => self.browser.handle_action(b).await,
            UIAction::Playlist(b) => self.playlist.handle_action(b).await,
            UIAction::Quit => todo!(),
        }
    }
}

impl Action for UIAction {
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            UIAction::Next | UIAction::Prev | UIAction::StepVolUp | UIAction::StepVolDown => {
                "".into()
            }
            UIAction::Browser(a) => a.context(),
            UIAction::Playlist(a) => a.context(),
            UIAction::Quit => "".into(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        format!("{:?}", self).into()
    }
}

impl TextHandler for YoutuiWindow {
    fn push_text(&mut self, c: char) {
        match self.context {
            WindowContext::Browser => self.browser.push_text(c),
            WindowContext::Playlist => self.playlist.push_text(c),
            WindowContext::Logs => self.logger.push_text(c),
        }
    }
    fn pop_text(&mut self) {
        match self.context {
            WindowContext::Browser => self.browser.pop_text(),
            WindowContext::Playlist => self.playlist.pop_text(),
            WindowContext::Logs => self.logger.pop_text(),
        }
    }
    fn is_text_handling(&self) -> bool {
        match self.context {
            WindowContext::Browser => self.browser.is_text_handling(),
            WindowContext::Playlist => self.playlist.is_text_handling(),
            WindowContext::Logs => self.logger.is_text_handling(),
        }
    }
    fn take_text(&mut self) -> String {
        match self.context {
            WindowContext::Browser => self.browser.take_text(),
            WindowContext::Playlist => self.playlist.take_text(),
            WindowContext::Logs => self.logger.take_text(),
        }
    }
    fn replace_text(&mut self, text: String) {
        match self.context {
            WindowContext::Browser => self.browser.replace_text(text),
            WindowContext::Playlist => self.playlist.replace_text(text),
            WindowContext::Logs => self.logger.replace_text(text),
        }
    }
}

impl YoutuiWindow {
    pub fn new(
        player_request_tx: mpsc::Sender<super::player::Request>,
        player_response_rx: mpsc::Receiver<super::player::Response>,
    ) -> YoutuiWindow {
        // TODO: derive default
        let (ui_tx, ui_rx) = mpsc::channel(CHANNEL_SIZE);
        YoutuiWindow {
            status: AppStatus::Running,
            tasks: TaskRegister::new(),
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist: Playlist::new(player_request_tx, player_response_rx, ui_tx.clone()),
            browser: Browser::new(ui_tx.clone()),
            logger: Logger::new(ui_tx.clone()),
            _ui_tx: ui_tx,
            ui_rx,
            keybinds: global_keybinds(),
            key_stack: Vec::new(),
            help_shown: false,
        }
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
        self.process_messages().await;
        self.process_ui_messages().await;
    }
    pub async fn process_ui_messages(&mut self) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UIMessage::DownloadSong(video_id, playlist_id) => {
                    self.tasks
                        .send_request(AppRequest::Download(video_id, playlist_id))
                        .await
                        .unwrap_or_else(|_| error!("Error sending Download Songs task"));
                }
                UIMessage::Quit => {
                    crossterm::terminal::disable_raw_mode().unwrap();
                    super::destruct_terminal();
                    self.status = super::ui::AppStatus::Exiting;
                }
                UIMessage::ChangeContext(context) => self.change_context(context),
                UIMessage::Next => self.playlist.handle_next().await,
                UIMessage::Prev => self.playlist.handle_previous().await,
                UIMessage::StepVolUp => self.playlist.handle_increase_volume().await,
                UIMessage::StepVolDown => self.playlist.handle_decrease_volume().await,
                UIMessage::GetSearchSuggestions(text) => {
                    self.tasks
                        .send_request(AppRequest::GetSearchSuggestions(text))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
                UIMessage::SearchArtist(artist) => {
                    self.tasks
                        .send_request(AppRequest::SearchArtists(artist))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
                UIMessage::GetArtistSongs(id) => {
                    self.tasks
                        .send_request(AppRequest::GetArtistSongs(id))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
                // XXX: We could potentially have a race condition here if this message arrives after
                // we receive a message from server to add songs.
                UIMessage::KillPendingSearchTasks => self
                    .tasks
                    .kill_all_task_type(taskregister::RequestCategory::Search),
                UIMessage::KillPendingGetTasks => self
                    .tasks
                    .kill_all_task_type(taskregister::RequestCategory::Get),
                UIMessage::AddSongsToPlaylist(song_list) => {
                    self.playlist.push_song_list(song_list);
                }
                UIMessage::PlaySongs(song_list) => {
                    self.playlist
                        .reset()
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> resetting playlist"));
                    let id = self.playlist.push_song_list(song_list);
                    self.playlist.play_song_id(id).await;
                }
            }
        }
    }
    pub async fn process_messages(&mut self) {
        // Process all messages in queue from API on each tick.
        while let Ok(msg) = self.tasks.try_recv() {
            match msg {
                server::Response::SongProgressUpdate(update, playlist_id, id) => {
                    self.handle_song_progress_update(update, playlist_id, id)
                        .await
                }
                server::Response::ReplaceArtistList(x, id) => {
                    self.handle_replace_artist_list(x, id).await
                }
                server::Response::SongsFound(id) => self.handle_songs_found(id),
                server::Response::AppendSongList(song_list, album, year, id) => {
                    self.handle_append_song_list(song_list, album, year, id)
                }
                server::Response::NoSongsFound(id) => self.handle_no_songs_found(id),
                server::Response::SongListLoading(id) => self.handle_song_list_loading(id),
                server::Response::SongListLoaded(id) => self.handle_song_list_loaded(id),
                server::Response::SearchArtistError(id) => self.handle_search_artist_error(id),
                server::Response::ReplaceSearchSuggestions(suggestions, id) => {
                    self.handle_replace_search_suggestions(suggestions, id)
                        .await
                }
            }
        }
    }
    async fn handle_song_progress_update(
        &mut self,
        update: SongProgressUpdateType,
        playlist_id: ListSongID,
        id: TaskID,
    ) {
        self.playlist
            .handle_song_progress_update(update, playlist_id, id)
            .await
    }
    async fn handle_replace_search_suggestions(&mut self, x: Vec<String>, id: TaskID) {
        tracing::info!(
            "Received request to replace search suggestions - ID {:?}",
            id
        );
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_replace_search_suggestions(x, id).await;
    }
    async fn handle_replace_artist_list(&mut self, x: Vec<SearchResultArtist>, id: TaskID) {
        tracing::info!("Received request to replace artists list - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_replace_artist_list(x, id).await;
    }
    fn handle_song_list_loaded(&mut self, id: TaskID) {
        tracing::info!("Received message that song list loaded - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_song_list_loaded(id);
    }
    pub fn handle_song_list_loading(&mut self, id: TaskID) {
        tracing::info!("Received message that song list loading - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_song_list_loading(id);
    }
    pub fn handle_no_songs_found(&mut self, id: TaskID) {
        tracing::info!("Received message that no songs found - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_no_songs_found(id)
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        id: TaskID,
    ) {
        tracing::info!("Received request to append song list - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser
            .handle_append_song_list(song_list, album, year, id)
    }
    pub fn handle_songs_found(&mut self, id: TaskID) {
        tracing::info!("Received response that songs found - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_songs_found(id);
    }
    fn handle_search_artist_error(&mut self, id: TaskID) {
        tracing::warn!("Received message that song list errored - ID {:?}", id);
        if !self.tasks.is_task_valid(id) {
            return;
        }
        self.browser.handle_search_artist_error(id)
    }
    // Splitting out event types removes one layer of indentation.
    pub async fn handle_event(&mut self, event: crossterm::event::Event) {
        match event {
            Event::Key(k) => self.handle_key_event(k).await,
            Event::Mouse(m) => self.handle_mouse_event(m),
            other => tracing::warn!("Received unimplemented {:?} event", other),
        }
    }
    async fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
        if self.handle_text_entry(key_event) {
            return;
        }
        self.key_stack.push(key_event);
        self.global_handle_key_stack().await;
    }
    fn handle_mouse_event(&mut self, mouse_event: crossterm::event::MouseEvent) {
        tracing::warn!("Received unimplemented {:?} mouse event", mouse_event);
    }
    async fn global_handle_key_stack(&mut self) {
        // First handle my own keybinds, otherwise forward.
        if let KeyHandleOutcome::ActionHandled =
            // TODO: Remove allocation
            self.handle_key_stack(self.key_stack.clone()).await
        {
            self.key_stack.clear()
        } else if let KeyHandleOutcome::Mode = match self.context {
            // TODO: Remove allocation
            WindowContext::Browser => self.browser.handle_key_stack(self.key_stack.clone()).await,
            WindowContext::Playlist => todo!(),
            WindowContext::Logs => todo!(),
        } {
        } else {
            self.key_stack.clear()
        }
    }
    fn key_pending(&self) -> bool {
        !self.key_stack.is_empty()
    }
    fn change_context(&mut self, new_context: WindowContext) {
        std::mem::swap(&mut self.context, &mut self.prev_context);
        self.context = new_context;
    }
    fn revert_context(&mut self) {
        std::mem::swap(&mut self.context, &mut self.prev_context);
    }
}

pub fn draw_app<B>(f: &mut Frame<B>, w: &YoutuiWindow)
where
    B: Backend,
{
    let base_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.size());
    header::draw_header(f, w, base_layout[0]);
    match w.context {
        WindowContext::Browser => w.browser.draw_context_chunk(f, base_layout[1]),
        WindowContext::Logs => w.logger.draw_context_chunk(f, base_layout[1]),
        WindowContext::Playlist => w.playlist.draw_context_chunk(f, base_layout[1]),
    }
    if w.key_pending() {
        draw_popup(f, w, base_layout[1]);
    }
    footer::draw_footer(f, w, base_layout[2]);
}
fn draw_popup<B: Backend>(f: &mut Frame<B>, w: &YoutuiWindow, chunk: Rect) {
    let title = "test";
    let commands = w.get_cur_mode();
    // TODO: Remove unwrap, although we shouldn't be drawing popup if no Map.
    let shortcuts_descriptions = commands.unwrap().collect::<Vec<_>>();
    // Cloning here only clones iterators, so it's low-cost.
    // let shortcut_len = shortcuts.clone().map(|s| s.len()).max().unwrap_or_default();
    // let description_len = descriptions
    //     .clone()
    //     .map(|d| d.len())
    //     .max()
    //     .unwrap_or_default();
    // XXX: temporary
    let shortcut_len = 10;
    let description_len = 5;
    let width = shortcut_len + description_len + 3;
    // let height = commands.len() + 2;
    // XXX: temporary
    let height = 10;
    let mut commands_vec = Vec::new();
    for (s, d) in shortcuts_descriptions {
        commands_vec.push(
            Row::new(vec![format!("{}", s), format!("{}", d)]).style(Style::new().fg(Color::White)),
        );
    }
    let table_constraints = [
        Constraint::Min(shortcut_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(description_len.try_into().unwrap_or(u16::MAX)),
    ];
    // let table_constraints = [
    //     Constraint::Length(shortcut_width + 10),
    //     Constraint::Length(description_width),
    // ];
    let block = Table::new(commands_vec)
        .style(Style::new().fg(Color::White))
        .block(
            Block::default()
                .title(title.as_ref())
                .borders(Borders::ALL)
                .style(Style::new().fg(Color::Cyan)),
        )
        .widths(&table_constraints);
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}
/// Helper function to create a popup at bottom corner of chunk.
pub fn left_bottom_corner_rect(height: u16, width: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(height)].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(width)].as_ref())
        .split(popup_layout[1])[1]
}
