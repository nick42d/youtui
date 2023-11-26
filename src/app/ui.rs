mod browser;
pub mod draw;
mod footer;
mod header;
mod logger;
mod playlist;

use self::browser::BrowserAction;
use self::playlist::PlaylistAction;
use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::component::actionhandler::{
    Action, ActionHandler, ActionProcessor, DisplayableKeyRouter, KeyHandleOutcome, KeyHandler,
    KeyRouter, Keybind, KeybindVisibility, Keymap, TextHandler,
};
use super::server;
use super::structures::*;
use super::taskmanager::{AppRequest, TaskID};
use crate::app::server::downloader::DownloadProgressUpdateType;
use crate::core::send_or_error;
use crate::error::Error;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::common::{SearchSuggestion, TextRun};
use ytmapi_rs::{
    parse::{SearchResultArtist, SongResult},
    ChannelID, VideoID,
};

const PAGE_KEY_SCROLL_AMOUNT: isize = 10;
const CHANNEL_SIZE: usize = 256;
const VOL_TICK: i8 = 5;

#[deprecated]
pub struct BasicCommand {
    key: KeyCode,
    name: String,
}
#[derive(PartialEq)]
pub enum AppStatus {
    Running,
    // Cow: Message
    Exiting(Cow<'static, str>),
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
// TODO: Shift these up to App. Then our UI want need to hold as many channels.
pub enum UIMessage {
    DownloadSong(VideoID<'static>, ListSongID),
    GetVolume,
    GetProgress(ListSongID),
    Quit,
    ChangeContext(WindowContext),
    Next,
    Prev,
    IncreaseVolume(i8),
    SearchArtist(String),
    GetSearchSuggestions(String),
    GetArtistSongs(ChannelID<'static>),
    AddSongsToPlaylist(Vec<ListSong>),
    AddSongsToPlaylistAndPlay(Vec<ListSong>),
    PlaySong(Arc<Vec<u8>>, ListSongID),
    PausePlay(ListSongID),
    Stop(ListSongID),
    StopAll,
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

impl ActionProcessor<UIAction> for YoutuiWindow {}

impl ActionHandler<UIAction> for YoutuiWindow {
    async fn handle_action(&mut self, action: &UIAction) {
        match action {
            UIAction::Next => self.playlist.handle_next().await,
            UIAction::Prev => self.playlist.handle_previous().await,
            UIAction::Pause => self.playlist.pauseplay().await,
            UIAction::StepVolUp => self.handle_increase_volume(VOL_TICK).await,
            UIAction::StepVolDown => self.handle_increase_volume(-VOL_TICK).await,
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
    pub fn get_status(&self) -> &AppStatus {
        &self.status
    }
    pub fn set_status(&mut self, new_status: AppStatus) {
        self.status = new_status;
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
        self.process_ui_messages().await;
    }
    pub fn quit(&mut self) {
        self.status = super::ui::AppStatus::Exiting("Quitting".into());
    }
    pub async fn process_ui_messages(&mut self) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UIMessage::DownloadSong(video_id, playlist_id) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::Download(video_id, playlist_id),
                    )
                    .await;
                }
                UIMessage::Quit => self.quit(),

                UIMessage::ChangeContext(context) => self.change_context(context),
                UIMessage::Next => self.playlist.handle_next().await,
                UIMessage::Prev => self.playlist.handle_previous().await,
                UIMessage::IncreaseVolume(i) => {
                    self.handle_increase_volume(i).await;
                }
                UIMessage::GetSearchSuggestions(text) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::GetSearchSuggestions(text),
                    )
                    .await;
                }
                UIMessage::SearchArtist(artist) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::SearchArtists(artist),
                    )
                    .await;
                }
                UIMessage::GetArtistSongs(id) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::GetArtistSongs(id),
                    )
                    .await;
                }
                UIMessage::AddSongsToPlaylist(song_list) => {
                    self.playlist.push_song_list(song_list);
                }
                UIMessage::AddSongsToPlaylistAndPlay(song_list) => {
                    self.playlist.reset().await;
                    let id = self.playlist.push_song_list(song_list);
                    self.playlist.play_song_id(id).await;
                }
                UIMessage::PlaySong(song, id) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::PlaySong(song, id),
                    )
                    .await;
                }

                UIMessage::PausePlay(id) => {
                    send_or_error(&self.task_manager_request_tx, AppRequest::PausePlay(id)).await;
                }
                UIMessage::Stop(id) => {
                    send_or_error(&self.task_manager_request_tx, AppRequest::Stop(id)).await;
                }
                UIMessage::StopAll => {
                    send_or_error(&self.task_manager_request_tx, AppRequest::StopAll).await;
                }
                UIMessage::GetVolume => {
                    send_or_error(&self.task_manager_request_tx, AppRequest::GetVolume).await;
                }
                UIMessage::GetProgress(id) => {
                    send_or_error(
                        &self.task_manager_request_tx,
                        AppRequest::GetPlayProgress(id),
                    )
                    .await;
                }
            }
        }
    }
    async fn handle_increase_volume(&mut self, inc: i8) {
        // Visually update the state first for instant feedback.
        self.playlist.increase_volume(inc);
        send_or_error(
            &self.task_manager_request_tx,
            AppRequest::IncreaseVolume(inc),
        )
        .await;
    }
    pub async fn handle_done_playing(&mut self, id: ListSongID) {
        self.playlist.handle_done_playing(id).await
    }
    pub async fn handle_set_to_paused(&mut self, id: ListSongID) {
        self.playlist.handle_set_to_paused(id).await
    }
    pub async fn handle_set_to_playing(&mut self, id: ListSongID) {
        self.playlist.handle_set_to_playing(id)
    }
    pub async fn handle_set_to_stopped(&mut self, id: ListSongID) {
        self.playlist.handle_set_to_stopped(id)
    }
    pub async fn handle_set_all_to_stopped(&mut self) {
        self.playlist.handle_set_all_to_stopped()
    }
    pub fn handle_set_volume(&mut self, p: Percentage) {
        self.playlist.handle_set_volume(p)
    }
    pub fn handle_api_error(&mut self, e: Error) {
        self.set_status(AppStatus::Exiting(e.to_string().into()));
    }
    pub fn handle_set_song_play_progress(&mut self, f: f64, id: ListSongID) {
        self.playlist.handle_set_song_play_progress(f, id);
    }
    pub async fn handle_set_song_download_progress(
        &mut self,
        update: DownloadProgressUpdateType,
        playlist_id: ListSongID,
    ) {
        self.playlist
            .handle_song_progress_update(update, playlist_id)
            .await
    }
    pub async fn handle_replace_search_suggestions(
        &mut self,
        x: Vec<SearchSuggestion>,
        search: String,
    ) {
        self.browser.handle_replace_search_suggestions(x, search);
    }
    pub async fn handle_replace_artist_list(&mut self, x: Vec<SearchResultArtist>) {
        self.browser.handle_replace_artist_list(x).await;
    }
    pub fn handle_song_list_loaded(&mut self) {
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
    pub fn handle_search_artist_error(&mut self) {
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
                if let Some(map) = self.playlist.get_key_subset(&self.key_stack) {
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
    // TODO: also return Mode description.
    // The downside of this approach is that if draw_popup is calling this function,
    // it is gettign called every tick.
    // Consider a way to set this in the in state memory.
    fn get_cur_mode_description(&self) -> Option<Cow<str>> {
        if let Some(map) = self.get_key_subset(&self.key_stack) {
            if let Keymap::Mode(mode) = map {
                return Some(mode.describe());
            }
        }
        match self.context {
            WindowContext::Browser => {
                if let Some(map) = self.browser.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.describe());
                    }
                }
            }
            WindowContext::Playlist => {
                if let Some(map) = self.playlist.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.describe());
                    }
                }
            }
            WindowContext::Logs => {
                if let Some(map) = self.logger.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.describe());
                    }
                }
            }
        }
        None
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
        Keybind::new_modified_from_code(KeyCode::Char('c'), KeyModifiers::CONTROL, UIAction::Quit),
    ]
}
