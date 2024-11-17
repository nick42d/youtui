use std::time::Duration;

use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::component::actionhandler::{
    get_key_subset, handle_key_stack, handle_key_stack_and_action, Action, ActionHandler,
    DominantKeyRouter, KeyDisplayer, KeyHandleAction, KeyHandleOutcome, KeyRouter, TextHandler,
};
use super::keycommand::{
    CommandVisibility, DisplayableCommand, DisplayableMode, KeyCommand, Keymap,
};
use super::server::{ArcServer, IncreaseVolume, TaskMetadata};
use super::structures::*;
use super::view::Scrollable;
use super::{AppCallback, ASYNC_CALLBACK_SENDER_CHANNEL_SIZE};
use crate::async_rodio_sink::{SeekDirection, VolumeUpdate};
use crate::core::send_or_error;
use async_callback_manager::{AsyncCallbackSender, Constraint};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use tokio::sync::mpsc;

mod browser;
pub mod draw;
mod footer;
mod header;
mod logger;
mod playlist;

const VOL_TICK: i8 = 5;
const SEEK_AMOUNT: Duration = Duration::from_secs(5);

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
    StepSeekForward,
    StepSeekBack,
    ToggleHelp,
    HelpUp,
    HelpDown,
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
    async_tx: AsyncCallbackSender<ArcServer, Self, TaskMetadata>,
}

pub struct HelpMenu {
    shown: bool,
    cur: usize,
    len: usize,
    keybinds: Vec<KeyCommand<UIAction>>,
    pub widget_state: TableState,
}

impl Default for HelpMenu {
    fn default() -> Self {
        HelpMenu {
            shown: Default::default(),
            cur: Default::default(),
            len: Default::default(),
            keybinds: help_keybinds(),
            widget_state: Default::default(),
        }
    }
}

impl Scrollable for HelpMenu {
    fn increment_list(&mut self, amount: isize) {
        self.cur = self
            .cur
            .saturating_add_signed(amount)
            .min(self.len.saturating_sub(1));
    }

    fn get_selected_item(&self) -> usize {
        self.cur
    }
}

impl DominantKeyRouter for YoutuiWindow {
    fn dominant_keybinds_active(&self) -> bool {
        self.help.shown
            || match self.context {
                WindowContext::Browser => self.browser.dominant_keybinds_active(),
                WindowContext::Playlist => false,
                WindowContext::Logs => false,
            }
    }
}

// We can't implement KeyRouter, as it would require us to have a single Action
// type for the whole application.
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
            .get_this_keybinds()
            .filter(|kc| kc.visibility == CommandVisibility::Global)
            .map(|kb| kb.as_displayable());
        if self.is_dominant_keybinds() {
            return Box::new(kb);
        }
        let cx = match self.context {
            // Consider if double boxing can be removed.
            WindowContext::Browser => Box::new(
                self.browser
                    .get_routed_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Playlist => Box::new(
                self.playlist
                    .get_routed_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            )
                as Box<dyn Iterator<Item = DisplayableCommand>>,
            WindowContext::Logs => Box::new(
                self.logger
                    .get_routed_global_keybinds()
                    .map(|kb| kb.as_displayable()),
            ) as Box<dyn Iterator<Item = DisplayableCommand>>,
        };
        Box::new(kb.chain(cx))
    }

    fn get_all_visible_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand> + 'a> {
        // Self.keybinds is incorrect
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

impl ActionHandler<UIAction> for YoutuiWindow {
    async fn handle_action(&mut self, action: &UIAction) {
        match action {
            UIAction::Next => self.playlist.handle_next().await,
            UIAction::Prev => self.playlist.handle_previous().await,
            UIAction::Pause => self.playlist.pauseplay().await,
            UIAction::StepVolUp => self.handle_increase_volume(VOL_TICK).await,
            UIAction::StepVolDown => self.handle_increase_volume(-VOL_TICK).await,
            UIAction::StepSeekForward => self.handle_seek(SEEK_AMOUNT, SeekDirection::Forward),
            UIAction::StepSeekBack => self.handle_seek(SEEK_AMOUNT, SeekDirection::Back),
            UIAction::Quit => send_or_error(&self.callback_tx, AppCallback::Quit).await,
            UIAction::ToggleHelp => self.toggle_help(),
            UIAction::ViewLogs => self.handle_change_context(WindowContext::Logs),
            UIAction::HelpUp => self.help.increment_list(-1),
            UIAction::HelpDown => self.help.increment_list(1),
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
            UIAction::HelpUp => "Help".into(),
            UIAction::HelpDown => "Help".into(),
            UIAction::StepSeekForward => "Global".into(),
            UIAction::StepSeekBack => "Global".into(),
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
            UIAction::HelpUp => "Help".into(),
            UIAction::HelpDown => "Help".into(),
            UIAction::StepSeekForward => format!("Seek Forward {}s", SEEK_AMOUNT.as_secs()).into(),
            UIAction::StepSeekBack => format!("Seek Back {}s", SEEK_AMOUNT.as_secs()).into(),
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
    pub async fn new(
        callback_tx: mpsc::Sender<AppCallback>,
        callback_manager: &mut async_callback_manager::AsyncCallbackManager<
            ArcServer,
            TaskMetadata,
        >,
    ) -> YoutuiWindow {
        YoutuiWindow {
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist: Playlist::new(callback_manager, callback_tx.clone()).await,
            browser: Browser::new(callback_manager, callback_tx.clone()),
            logger: Logger::new(callback_tx.clone()),
            keybinds: global_keybinds(),
            key_stack: Vec::new(),
            help: Default::default(),
            callback_tx,
            async_tx: callback_manager.new_sender(ASYNC_CALLBACK_SENDER_CHANNEL_SIZE),
        }
    }
    pub async fn async_update(&mut self) {
        tokio::select! {
            _ = self.browser.async_update() => (),
            _ = self.playlist.async_update() => (),
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
    pub async fn handle_increase_volume(&mut self, inc: i8) {
        // Visually update the state first for instant feedback.
        self.increase_volume(inc);
        self.async_tx.add_callback(
            IncreaseVolume(inc),
            Self::handle_volume_update,
            Some(Constraint::new_block_same_type()),
        );
    }
    pub fn handle_seek(&mut self, duration: Duration, direction: SeekDirection) {
        self.playlist.handle_seek(duration, direction);
    }
    pub fn handle_volume_update(&mut self, update: Option<VolumeUpdate>) {
        self.playlist.handle_volume_update(update)
    }
    pub fn handle_add_songs_to_playlist(&mut self, song_list: Vec<ListSong>) {
        let _ = self.playlist.push_song_list(song_list);
    }
    pub fn handle_add_songs_to_playlist_and_play(&mut self, song_list: Vec<ListSong>) {
        self.playlist.reset();
        let id = self.playlist.push_song_list(song_list);
        self.playlist.play_song_id(id);
    }
    fn is_dominant_keybinds(&self) -> bool {
        self.help.shown
    }
    fn get_this_keybinds(&self) -> Box<dyn Iterator<Item = &KeyCommand<UIAction>> + '_> {
        Box::new(if self.help.shown {
            Box::new(self.help.keybinds.iter()) as Box<dyn Iterator<Item = &KeyCommand<UIAction>>>
        } else if self.dominant_keybinds_active() {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = &KeyCommand<UIAction>>>
        } else {
            Box::new(self.keybinds.iter()) as Box<dyn Iterator<Item = &KeyCommand<UIAction>>>
        })
    }

    async fn global_handle_key_stack(&mut self) {
        // First handle my own keybinds, otherwise forward if our keybinds are not
        // dominant. TODO: Remove allocation
        match handle_key_stack(self.get_this_keybinds(), self.key_stack.clone()) {
            KeyHandleAction::Action(a) => {
                self.handle_action(&a).await;
                self.key_stack.clear();
                return;
            }
            KeyHandleAction::Mode => {
                return;
            }
            KeyHandleAction::NoMap => {
                if self.is_dominant_keybinds() {
                    self.key_stack.clear();
                    return;
                }
            }
        };
        if let KeyHandleOutcome::Mode = match self.context {
            // TODO: Remove allocation
            WindowContext::Browser => {
                handle_key_stack_and_action(&mut self.browser, self.key_stack.clone()).await
            }
            WindowContext::Playlist => {
                handle_key_stack_and_action(&mut self.playlist, self.key_stack.clone()).await
            }
            WindowContext::Logs => {
                handle_key_stack_and_action(&mut self.logger, self.key_stack.clone()).await
            }
        } {
            return;
        }
        self.key_stack.clear()
    }
    fn key_pending(&self) -> bool {
        !self.key_stack.is_empty()
    }
    fn toggle_help(&mut self) {
        if self.help.shown {
            self.help.shown = false;
        } else {
            self.help.shown = true;
            // Setup Help menu parameters
            self.help.cur = 0;
            // We have to get the keybind length this way as the help menu iterator is not
            // ExactSized
            self.help.len = self.get_all_visible_keybinds_as_readable_iter().count();
        }
    }
    /// Visually increment the volume, note, does not actually change the
    /// volume.
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
    // The downside of this approach is that if draw_popup is calling this function,
    // it is gettign called every tick.
    // Consider a way to set this in the in state memory.
    fn get_cur_displayable_mode(&self) -> Option<DisplayableMode<'_>> {
        if let Some(Keymap::Mode(mode)) = get_key_subset(self.get_this_keybinds(), &self.key_stack)
        {
            return Some(DisplayableMode {
                displayable_commands: mode.as_displayable_iter(),
                description: mode.describe(),
            });
        }
        match self.context {
            WindowContext::Browser => {
                if let Some(Keymap::Mode(mode)) =
                    get_key_subset(self.browser.get_routed_keybinds(), &self.key_stack)
                {
                    return Some(DisplayableMode {
                        displayable_commands: mode.as_displayable_iter(),
                        description: mode.describe(),
                    });
                }
            }
            WindowContext::Playlist => {
                if let Some(Keymap::Mode(mode)) =
                    get_key_subset(self.playlist.get_routed_keybinds(), &self.key_stack)
                {
                    return Some(DisplayableMode {
                        displayable_commands: mode.as_displayable_iter(),
                        description: mode.describe(),
                    });
                }
            }
            WindowContext::Logs => {
                if let Some(Keymap::Mode(mode)) =
                    get_key_subset(self.logger.get_routed_keybinds(), &self.key_stack)
                {
                    return Some(DisplayableMode {
                        displayable_commands: mode.as_displayable_iter(),
                        description: mode.describe(),
                    });
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
        KeyCommand::new_from_code(KeyCode::Char('['), UIAction::StepSeekBack),
        KeyCommand::new_from_code(KeyCode::Char(']'), UIAction::StepSeekForward),
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
fn help_keybinds() -> Vec<KeyCommand<UIAction>> {
    vec![
        KeyCommand::new_hidden_from_code(KeyCode::Down, UIAction::HelpDown),
        KeyCommand::new_hidden_from_code(KeyCode::Up, UIAction::HelpUp),
        KeyCommand::new_hidden_from_code(KeyCode::Esc, UIAction::ToggleHelp),
        KeyCommand::new_global_from_code(KeyCode::F(1), UIAction::ToggleHelp),
    ]
}
