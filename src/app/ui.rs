mod browser;
pub mod draw;
mod footer;
mod header;
mod logger;
mod playlist;

use std::borrow::Cow;

use self::browser::BrowserAction;
use self::playlist::PlaylistAction;
use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::taskmanager::{AppRequest, TaskID};

use super::component::actionhandler::{
    Action, ActionHandler, ActionProcessor, DisplayableKeyRouter, KeyHandleOutcome, KeyHandler,
    KeyRouter, Keybind, KeybindVisibility, Keymap, TextHandler,
};

use super::server;
use super::structures::*;
use crate::app::server::downloader::SongProgressUpdateType;
use crossterm::event::{Event, KeyCode, KeyEvent};
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::common::TextRun;
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
    AddSongsToPlaylist(Vec<ListSong>),
    PlaySongs(Vec<ListSong>),
}

// A message from the server to update state.
#[derive(Debug)]
pub enum StateUpdateMessage {
    SetSongProgress(SongProgressUpdateType, ListSongID),
    ReplaceArtistList(Vec<ytmapi_rs::parse::SearchResultArtist>),
    HandleSearchArtistError,
    ReplaceSearchSuggestions(Vec<Vec<TextRun>>, String),
    HandleSongListLoading,
    HandleSongListLoaded,
    HandleNoSongsFound,
    HandleSongsFound,
    AppendSongList {
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
    },
    HandleDonePlaying(ListSongID),
    SetToPaused(ListSongID),
    SetToPlaying(ListSongID),
    SetToStopped,
    SetVolume(Percentage),
}

// An action that can be triggered from a keybind.
#[derive(Clone, Debug, PartialEq)]
pub enum UIAction {
    Quit,
    Next,
    Prev,
    Pause,
    StepVolUp,
    StepVolDown,
    Browser(BrowserAction),
    Playlist(PlaylistAction),
    ToggleHelp,
    ViewLogs,
}

pub struct YoutuiWindow {
    status: AppStatus,
    context: WindowContext,
    prev_context: WindowContext,
    playlist: Playlist,
    browser: Browser,
    logger: Logger,
    _ui_tx: mpsc::Sender<UIMessage>,
    ui_rx: mpsc::Receiver<UIMessage>,
    task_manager_request_tx: mpsc::Sender<AppRequest>,
    keybinds: Vec<Keybind<UIAction>>,
    key_stack: Vec<KeyEvent>,
    help_shown: bool,
}

impl DisplayableKeyRouter for YoutuiWindow {
    // XXX: Can turn these boxed iterators into types.
    fn get_all_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a> {
        let kb = self.keybinds.iter().map(|kb| kb.as_readable());
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => {
                Box::new(self.browser.get_all_keybinds().map(|kb| kb.as_readable()))
                    as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>
            }
            WindowContext::Playlist => {
                Box::new(self.playlist.get_all_keybinds().map(|kb| kb.as_readable()))
                    as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>
            }
            WindowContext::Logs => {
                Box::new(self.logger.get_all_keybinds().map(|kb| kb.as_readable()))
                    as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>
            }
        };
        Box::new(kb.chain(cx))
    }

    fn get_all_global_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a> {
        let kb = self
            .keybinds
            .iter()
            .filter(|kb| kb.visibility == KeybindVisibility::Global)
            .map(|kb| kb.as_readable());
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => Box::new(
                self.browser
                    .get_all_keybinds()
                    .filter(|kb| kb.visibility == KeybindVisibility::Global)
                    .map(|kb| kb.as_readable()),
            )
                as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>,
            WindowContext::Playlist => Box::new(
                self.playlist
                    .get_all_keybinds()
                    .filter(|kb| kb.visibility == KeybindVisibility::Global)
                    .map(|kb| kb.as_readable()),
            )
                as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>,
            WindowContext::Logs => Box::new(
                self.logger
                    .get_all_keybinds()
                    .filter(|kb| kb.visibility == KeybindVisibility::Global)
                    .map(|kb| kb.as_readable()),
            )
                as Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)>>,
        };
        Box::new(kb.chain(cx))
    }
}

impl KeyHandler<UIAction> for YoutuiWindow {
    // XXX: Need to determine how this should really be implemented.
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<UIAction>> + 'a> {
        Box::new(self.keybinds.iter())
    }
}

impl YoutuiWindow {
    // TODO: also return Mode description.
    // The downside of this approach is that if draw_popup is calling this function,
    // it is gettign called every tick.
    // Consider a way to set this in the in state memory.
    fn get_cur_mode<'a>(&'a self) -> Option<Box<dyn Iterator<Item = (Cow<str>, Cow<str>)> + 'a>> {
        if let Some(map) = self.get_key_subset(&self.key_stack) {
            if let Keymap::Mode(mode) = map {
                return Some(mode.as_readable_short_iter());
            }
        }
        match self.context {
            WindowContext::Browser => {
                if let Some(map) = self.browser.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_readable_short_iter());
                    }
                }
            }
            WindowContext::Playlist => {
                if let Some(map) = self.logger.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_readable_short_iter());
                    }
                }
            }
            WindowContext::Logs => {
                if let Some(map) = self.logger.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_readable_short_iter());
                    }
                }
            }
        }
        None
    }
}

impl ActionProcessor<UIAction> for YoutuiWindow {}

impl ActionHandler<UIAction> for YoutuiWindow {
    async fn handle_action(&mut self, action: &UIAction) {
        match action {
            UIAction::Next => self.playlist.handle_next().await,
            UIAction::Prev => self.playlist.handle_previous().await,
            UIAction::Pause => self.playlist.pauseplay().await,
            UIAction::StepVolUp => self.playlist.handle_increase_volume().await,
            UIAction::StepVolDown => self.playlist.handle_decrease_volume().await,
            UIAction::Browser(b) => self.browser.handle_action(b).await,
            UIAction::Playlist(b) => self.playlist.handle_action(b).await,
            UIAction::Quit => self.quit(),
            UIAction::ToggleHelp => self.help_shown = !self.help_shown,
            UIAction::ViewLogs => self.change_context(WindowContext::Logs),
        }
    }
}

impl Action for UIAction {
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            UIAction::Next | UIAction::Prev | UIAction::StepVolUp | UIAction::StepVolDown => {
                "Global".into()
            }
            UIAction::Browser(a) => a.context(),
            UIAction::Playlist(a) => a.context(),
            UIAction::Quit => "Global".into(),
            UIAction::ToggleHelp => "Global".into(),
            UIAction::ViewLogs => "Global".into(),
            UIAction::Pause => "Global".into(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            UIAction::Quit => "Quit".into(),
            UIAction::Prev => "Prev Song".into(),
            UIAction::Next => "Next Song".into(),
            UIAction::Pause => "Pause".into(),
            UIAction::StepVolUp => "Vol Up".into(),
            UIAction::StepVolDown => "Vol Down".into(),
            UIAction::ToggleHelp => "Toggle Help".into(),
            UIAction::ViewLogs => "View Logs".into(),
            UIAction::Browser(a) => a.describe(),
            UIAction::Playlist(a) => a.describe(),
        }
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
    pub fn new(task_manager_request_tx: mpsc::Sender<AppRequest>) -> YoutuiWindow {
        // TODO: derive default
        let (ui_tx, ui_rx) = mpsc::channel(CHANNEL_SIZE);
        YoutuiWindow {
            status: AppStatus::Running,
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist: Playlist::new(ui_tx.clone()),
            browser: Browser::new(ui_tx.clone()),
            logger: Logger::new(ui_tx.clone()),
            _ui_tx: ui_tx,
            ui_rx,
            keybinds: global_keybinds(),
            key_stack: Vec::new(),
            help_shown: false,
            task_manager_request_tx,
        }
    }
    pub fn get_status(&self) -> AppStatus {
        self.status
    }
    pub fn set_status(&mut self, new_status: AppStatus) {
        self.status = new_status;
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
        self.process_ui_messages().await;
    }
    pub fn quit(&mut self) {
        crossterm::terminal::disable_raw_mode().unwrap();
        super::destruct_terminal();
        self.status = super::ui::AppStatus::Exiting;
    }
    pub async fn process_ui_messages(&mut self) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UIMessage::DownloadSong(video_id, playlist_id) => {
                    self.task_manager_request_tx
                        .send(AppRequest::Download(video_id, playlist_id))
                        .await
                        .unwrap_or_else(|_| error!("Error sending Download Songs task"));
                }
                UIMessage::Quit => self.quit(),

                UIMessage::ChangeContext(context) => self.change_context(context),
                UIMessage::Next => self.playlist.handle_next().await,
                UIMessage::Prev => self.playlist.handle_previous().await,
                UIMessage::StepVolUp => self.playlist.handle_increase_volume().await,
                UIMessage::StepVolDown => self.playlist.handle_decrease_volume().await,
                UIMessage::GetSearchSuggestions(text) => {
                    self.task_manager_request_tx
                        .send(AppRequest::GetSearchSuggestions(text))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
                UIMessage::SearchArtist(artist) => {
                    self.task_manager_request_tx
                        .send(AppRequest::SearchArtists(artist))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
                UIMessage::GetArtistSongs(id) => {
                    self.task_manager_request_tx
                        .send(AppRequest::GetArtistSongs(id))
                        .await
                        .unwrap_or_else(|e| error!("Error <{e}> sending request"));
                }
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
    pub async fn process_state_updates(&mut self, state_updates: Vec<StateUpdateMessage>) {
        // Process all messages in queue from API on each tick.
        for msg in state_updates {
            tracing::debug!("Processing {:?}", msg);
            match msg {
                StateUpdateMessage::SetSongProgress(update, id) => {
                    self.handle_song_progress_update(update, id).await
                }
                StateUpdateMessage::ReplaceArtistList(l) => {
                    self.handle_replace_artist_list(l).await
                }
                StateUpdateMessage::HandleSearchArtistError => self.handle_search_artist_error(),
                StateUpdateMessage::ReplaceSearchSuggestions(runs, query) => {
                    self.handle_replace_search_suggestions(runs, query).await
                }
                StateUpdateMessage::HandleSongListLoading => self.handle_song_list_loading(),
                StateUpdateMessage::HandleSongListLoaded => self.handle_song_list_loaded(),
                StateUpdateMessage::HandleNoSongsFound => self.handle_no_songs_found(),
                StateUpdateMessage::HandleSongsFound => self.handle_songs_found(),
                StateUpdateMessage::AppendSongList {
                    song_list,
                    album,
                    year,
                    artist,
                } => self.handle_append_song_list(song_list, album, year, artist),
                StateUpdateMessage::HandleDonePlaying(id) => self.handle_done_playing(id).await,
                StateUpdateMessage::SetToPaused(id) => self.handle_set_to_paused(id).await,
                StateUpdateMessage::SetToPlaying(id) => self.handle_set_to_playing(id).await,
                StateUpdateMessage::SetToStopped => self.handle_set_to_stopped().await,
                StateUpdateMessage::SetVolume(p) => self.handle_set_volume(p),
            }
        }
    }
    async fn handle_done_playing(&mut self, id: ListSongID) {
        self.playlist.handle_done_playing(id).await
    }
    async fn handle_set_to_paused(&mut self, id: ListSongID) {
        self.playlist.handle_set_to_paused(id).await
    }
    async fn handle_set_to_playing(&mut self, id: ListSongID) {
        self.playlist.handle_set_to_playing(id).await
    }
    async fn handle_set_to_stopped(&mut self) {
        self.playlist.handle_set_to_stopped().await
    }
    fn handle_set_volume(&mut self, p: Percentage) {
        self.playlist.handle_set_volume(p)
    }
    async fn handle_song_progress_update(
        &mut self,
        update: SongProgressUpdateType,
        playlist_id: ListSongID,
    ) {
        self.playlist
            .handle_song_progress_update(update, playlist_id)
            .await
    }
    async fn handle_replace_search_suggestions(&mut self, x: Vec<Vec<TextRun>>, search: String) {
        self.browser.handle_replace_search_suggestions(x, search);
    }
    async fn handle_replace_artist_list(&mut self, x: Vec<SearchResultArtist>) {
        self.browser.handle_replace_artist_list(x).await;
    }
    fn handle_song_list_loaded(&mut self) {
        self.browser.handle_song_list_loaded();
    }
    pub fn handle_song_list_loading(&mut self) {
        self.browser.handle_song_list_loading();
    }
    pub fn handle_no_songs_found(&mut self) {
        self.browser.handle_no_songs_found();
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
    ) {
        self.browser
            .handle_append_song_list(song_list, album, year, artist)
    }
    pub fn handle_songs_found(&mut self) {
        self.browser.handle_songs_found();
    }
    fn handle_search_artist_error(&mut self) {
        self.browser.handle_search_artist_error();
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
            WindowContext::Playlist => self.playlist.handle_key_stack(self.key_stack.clone()).await,
            WindowContext::Logs => self.logger.handle_key_stack(self.key_stack.clone()).await,
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

fn global_keybinds() -> Vec<Keybind<UIAction>> {
    vec![
        Keybind::new_from_code(KeyCode::Char('+'), UIAction::StepVolUp),
        Keybind::new_from_code(KeyCode::Char('-'), UIAction::StepVolDown),
        Keybind::new_from_code(KeyCode::Char('<'), UIAction::Prev),
        Keybind::new_from_code(KeyCode::Char('>'), UIAction::Next),
        Keybind::new_global_from_code(KeyCode::F(1), UIAction::ToggleHelp),
        Keybind::new_global_from_code(KeyCode::F(10), UIAction::Quit),
        Keybind::new_global_from_code(KeyCode::F(12), UIAction::ViewLogs),
        Keybind::new_global_from_code(KeyCode::Char(' '), UIAction::Pause),
    ]
}
