use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::component::actionhandler::{
    Action, ActionHandler, ActionProcessor, KeyDisplayer, KeyHandleOutcome, KeyHandler, KeyRouter,
    TextHandler,
};
use super::keycommand::{CommandVisibility, DisplayableCommand, KeyCommand, Keymap};
use super::structures::*;
use super::AppCallback;
use crate::app::server::downloader::DownloadProgressUpdateType;
use crate::core::send_or_error;
use crate::error::Error;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::{ListState, TableState};
use std::borrow::Cow;
use tokio::sync::mpsc;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::parse::{SearchResultArtist, SongResult};

mod browser;
pub mod draw;
mod footer;
mod header;
mod logger;
mod playlist;

const VOL_TICK: i8 = 5;

// Which app level keyboard shortcuts function.
// What is displayed in header
// The main pane of the application
// XXX: This is a bit like a route.
#[derive(Debug)]
pub enum WindowContext {
    Browser,
    Playlist,
    Logs,
}

// An Action that can be triggered from a keybind.
#[derive(Clone, Debug, PartialEq)]
pub enum UIAction {
    Quit,
    Next,
    Prev,
    Pause,
    StepVolUp,
    StepVolDown,
    ToggleHelp,
    ViewLogs,
}

pub struct YoutuiWindow {
    context: WindowContext,
    prev_context: WindowContext,
    playlist: Playlist,
    browser: Browser,
    logger: Logger,
    callback_tx: mpsc::Sender<AppCallback>,
    keybinds: Vec<KeyCommand<UIAction>>,
    key_stack: Vec<KeyEvent>,
    help: HelpMenu,
    mutable_state: YoutuiMutableState,
}

#[derive(Default)]
pub struct HelpMenu {
    shown: bool,
    cur: usize,
}

// Mutable state for scrollable widgets.
// This needs to be stored seperately so that we don't have concurrent mutable access.
#[derive(Default)]
pub struct YoutuiMutableState {
    pub filter_state: ListState,
    pub help_state: TableState,
    pub browser_album_songs_state: TableState,
    pub browser_artists_state: ListState,
    pub playlist_state: TableState,
}

// We can't implemnent KeyRouter, as it would require us to have a single Action type for the whole application.
impl KeyDisplayer for YoutuiWindow {
    // XXX: Can turn these boxed iterators into types.
    fn get_all_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<(dyn Iterator<Item = DisplayableCommand<'a>> + 'a)> {
        let kb = self.keybinds.iter().map(|kb| kb.as_displayable());
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => Box::new(
                self.browser
                    .get_all_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Playlist => Box::new(
                self.playlist
                    .get_all_keybinds()
                    .map(|kb| kb.as_displayable()),
            )
                as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Logs => {
                Box::new(self.logger.get_all_keybinds().map(|kb| kb.as_displayable()))
                    as Box<dyn Iterator<Item = DisplayableCommand>>
            }
        };
        Box::new(kb.chain(cx))
    }

    fn get_context_global_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand> + 'a> {
        let kb = self
            .keybinds
            .iter()
            .filter(|kb| kb.visibility == CommandVisibility::Global)
            .map(|kb| kb.as_displayable());
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => Box::new(
                self.browser
                    .get_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Playlist => Box::new(
                self.playlist
                    .get_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            )
                as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Logs => Box::new(
                self.logger
                    .get_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
        };
        Box::new(kb.chain(cx))
    }

    fn get_all_visible_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand> + 'a> {
        let kb = self
            .keybinds
            .iter()
            .filter(|kb| kb.visibility != CommandVisibility::Hidden)
            .map(|kb| kb.as_displayable());
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => Box::new(
                self.browser
                    .get_all_visible_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Playlist => Box::new(
                self.playlist
                    .get_all_visible_keybinds()
                    .map(|kb| kb.as_displayable()),
            )
                as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Logs => Box::new(
                self.logger
                    .get_all_visible_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
        };
        Box::new(kb.chain(cx))
    }
}

impl KeyHandler<UIAction> for YoutuiWindow {
    // XXX: Need to determine how this should really be implemented.
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a KeyCommand<UIAction>> + 'a> {
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
            UIAction::Quit => send_or_error(&self.callback_tx, AppCallback::Quit).await,
            UIAction::ToggleHelp => self.help.shown = !self.help.shown,
            UIAction::ViewLogs => self.handle_change_context(WindowContext::Logs),
        }
    }
}

impl Action for UIAction {
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            UIAction::Next | UIAction::Prev | UIAction::StepVolUp | UIAction::StepVolDown => {
                "Global".into()
            }
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
    pub fn new(callback_tx: mpsc::Sender<AppCallback>) -> YoutuiWindow {
        // TODO: derive default
        YoutuiWindow {
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist: Playlist::new(callback_tx.clone()),
            browser: Browser::new(callback_tx.clone()),
            logger: Logger::new(callback_tx.clone()),
            keybinds: global_keybinds(),
            key_stack: Vec::new(),
            help: Default::default(),
            mutable_state: Default::default(),
            callback_tx,
        }
    }
    // Splitting out event types removes one layer of indentation.
    pub async fn handle_event(&mut self, event: crossterm::event::Event) {
        match event {
            Event::Key(k) => self.handle_key_event(k).await,
            Event::Mouse(m) => self.handle_mouse_event(m),
            other => tracing::warn!("Received unimplemented {:?} event", other),
        }
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
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
    // XXX: Should not be here, but required for now due to callback routing.
    pub async fn handle_api_error(&mut self, e: Error) {
        send_or_error(&self.callback_tx, AppCallback::HandleApiError(e)).await;
    }

    pub async fn handle_increase_volume(&mut self, inc: i8) {
        // Visually update the state first for instant feedback.
        self.increase_volume(inc);
        send_or_error(&self.callback_tx, AppCallback::IncreaseVolume(inc)).await;
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
    pub fn handle_set_volume(&mut self, p: Percentage) {
        self.playlist.handle_set_volume(p)
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
    pub fn handle_add_songs_to_playlist(&mut self, song_list: Vec<ListSong>) {
        let _ = self.playlist.push_song_list(song_list);
    }
    pub async fn handle_add_songs_to_playlist_and_play(&mut self, song_list: Vec<ListSong>) {
        self.playlist.reset().await;
        let id = self.playlist.push_song_list(song_list);
        self.playlist.play_song_id(id).await;
    }
    pub fn handle_songs_found(&mut self) {
        self.browser.handle_songs_found();
    }
    pub fn handle_search_artist_error(&mut self) {
        self.browser.handle_search_artist_error();
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

    /// Visually increment the volume, note, does not actually change the volume.
    fn increase_volume(&mut self, inc: i8) {
        self.playlist.increase_volume(inc);
    }
    pub fn handle_change_context(&mut self, new_context: WindowContext) {
        std::mem::swap(&mut self.context, &mut self.prev_context);
        self.context = new_context;
    }
    fn _revert_context(&mut self) {
        std::mem::swap(&mut self.context, &mut self.prev_context);
    }
    // TODO: also return Mode description.
    // The downside of this approach is that if draw_popup is calling this function,
    // it is gettign called every tick.
    // Consider a way to set this in the in state memory.
    fn get_cur_mode<'a>(&'a self) -> Option<Box<dyn Iterator<Item = DisplayableCommand> + 'a>> {
        if let Some(map) = self.get_key_subset(&self.key_stack) {
            if let Keymap::Mode(mode) = map {
                return Some(mode.as_displayable_iter());
            }
        }
        match self.context {
            WindowContext::Browser => {
                if let Some(map) = self.browser.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_displayable_iter());
                    }
                }
            }
            WindowContext::Playlist => {
                if let Some(map) = self.playlist.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_displayable_iter());
                    }
                }
            }
            WindowContext::Logs => {
                if let Some(map) = self.logger.get_key_subset(&self.key_stack) {
                    if let Keymap::Mode(mode) = map {
                        return Some(mode.as_displayable_iter());
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

fn global_keybinds() -> Vec<KeyCommand<UIAction>> {
    vec![
        KeyCommand::new_from_code(KeyCode::Char('+'), UIAction::StepVolUp),
        KeyCommand::new_from_code(KeyCode::Char('-'), UIAction::StepVolDown),
        KeyCommand::new_from_code(KeyCode::Char('<'), UIAction::Prev),
        KeyCommand::new_from_code(KeyCode::Char('>'), UIAction::Next),
        KeyCommand::new_global_from_code(KeyCode::F(1), UIAction::ToggleHelp),
        KeyCommand::new_global_from_code(KeyCode::F(10), UIAction::Quit),
        KeyCommand::new_global_from_code(KeyCode::F(12), UIAction::ViewLogs),
        KeyCommand::new_global_from_code(KeyCode::Char(' '), UIAction::Pause),
        KeyCommand::new_modified_from_code(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            UIAction::Quit,
        ),
    ]
}
