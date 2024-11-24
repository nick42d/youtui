use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::component::actionhandler::{
    get_key_subset, handle_key_stack, handle_key_stack_and_action, Action, ComponentEffect,
    DominantKeyRouter, KeyDisplayer, KeyHandleAction, KeyHandleOutcome, KeyRouter, TextHandler,
};
use super::keycommand::{
    CommandVisibility, DisplayableCommand, DisplayableMode, KeyCommand, Keymap,
};
use super::server::{ArcServer, IncreaseVolume, TaskMetadata};
use super::structures::*;
use super::view::Scrollable;
use super::AppCallback;
use crate::async_rodio_sink::{SeekDirection, VolumeUpdate};
use crate::config::Config;
use crate::core::send_or_error;
use async_callback_manager::{AsyncTask, Constraint};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::Duration;
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
}
impl_youtui_component!(YoutuiWindow);

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
impl Action for UIAction {
    type State = YoutuiWindow;
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
    async fn apply(self, state: &mut Self::State) -> ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            UIAction::Next => {
                return state
                    .playlist
                    .handle_next()
                    .await
                    .map(|this: &mut Self::State| &mut this.playlist)
            }
            UIAction::Prev => {
                return state
                    .playlist
                    .handle_previous()
                    .await
                    .map(|this: &mut Self::State| &mut this.playlist)
            }
            UIAction::Pause => {
                return state
                    .playlist
                    .pauseplay()
                    .await
                    .map(|this: &mut Self::State| &mut this.playlist)
            }
            UIAction::StepVolUp => return state.handle_increase_volume(VOL_TICK).await,
            UIAction::StepVolDown => return state.handle_increase_volume(-VOL_TICK).await,
            UIAction::StepSeekForward => {
                return state.handle_seek(SEEK_AMOUNT, SeekDirection::Forward)
            }
            UIAction::StepSeekBack => return state.handle_seek(SEEK_AMOUNT, SeekDirection::Back),
            UIAction::Quit => send_or_error(&state.callback_tx, AppCallback::Quit).await,
            UIAction::ToggleHelp => state.toggle_help(),
            UIAction::ViewLogs => state.handle_change_context(WindowContext::Logs),
            UIAction::HelpUp => state.help.increment_list(-1),
            UIAction::HelpDown => state.help.increment_list(1),
        }
        AsyncTask::new_no_op()
    }
}

impl TextHandler for YoutuiWindow {
    fn is_text_handling(&self) -> bool {
        match self.context {
            WindowContext::Browser => self.browser.is_text_handling(),
            WindowContext::Playlist => self.playlist.is_text_handling(),
            WindowContext::Logs => self.logger.is_text_handling(),
        }
    }
    fn get_text(&self) -> &str {
        match self.context {
            WindowContext::Browser => self.browser.get_text(),
            WindowContext::Playlist => self.playlist.get_text(),
            WindowContext::Logs => self.logger.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.context {
            WindowContext::Browser => self.browser.replace_text(text),
            WindowContext::Playlist => self.playlist.replace_text(text),
            WindowContext::Logs => self.logger.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.context {
            WindowContext::Browser => self.browser.clear_text(),
            WindowContext::Playlist => self.playlist.clear_text(),
            WindowContext::Logs => self.logger.clear_text(),
        }
    }
    fn handle_event_repr(&mut self, event: &Event) -> Option<ComponentEffect<Self>> {
        match self.context {
            WindowContext::Browser => self
                .browser
                .handle_event_repr(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.browser)),
            WindowContext::Playlist => self
                .playlist
                .handle_event_repr(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.playlist)),
            WindowContext::Logs => self
                .logger
                .handle_event_repr(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.logger)),
        }
    }
}

impl YoutuiWindow {
    pub fn new(
        callback_tx: mpsc::Sender<AppCallback>,
        config: &Config,
    ) -> (YoutuiWindow, ComponentEffect<YoutuiWindow>) {
        let (playlist, task) = Playlist::new(callback_tx.clone());
        let this = YoutuiWindow {
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist,
            browser: Browser::new(callback_tx.clone()),
            logger: Logger::new(callback_tx.clone()),
            keybinds: global_keybinds(),
            key_stack: Vec::new(),
            help: Default::default(),
            callback_tx,
        };
        (this, task.map(|this: &mut Self| &mut this.playlist))
    }
    // Splitting out event types removes one layer of indentation.
    pub async fn handle_initial_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> ComponentEffect<Self> {
        if let Some(effect) = self.handle_event(&event) {
            return effect;
        };
        match event {
            Event::Key(k) => return self.handle_key_event(k).await,
            Event::Mouse(m) => return self.handle_mouse_event(m),
            other => tracing::warn!("Received unimplemented {:?} event", other),
        }
        AsyncTask::new_no_op()
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
    }
    async fn handle_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> ComponentEffect<Self> {
        self.key_stack.push(key_event);
        self.global_handle_key_stack().await
    }
    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> ComponentEffect<Self> {
        tracing::warn!("Received unimplemented {:?} mouse event", mouse_event);
        AsyncTask::new_no_op()
    }
    pub async fn handle_increase_volume(&mut self, inc: i8) -> ComponentEffect<Self> {
        // Visually update the state first for instant feedback.
        self.increase_volume(inc);
        AsyncTask::new_future(
            IncreaseVolume(inc),
            Self::handle_volume_update,
            Some(Constraint::new_block_same_type()),
        )
    }
    pub fn handle_seek(
        &mut self,
        duration: Duration,
        direction: SeekDirection,
    ) -> ComponentEffect<Self> {
        self.playlist
            .handle_seek(duration, direction)
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn handle_volume_update(&mut self, update: Option<VolumeUpdate>) {
        self.playlist.handle_volume_update(update)
    }
    pub fn handle_add_songs_to_playlist(&mut self, song_list: Vec<ListSong>) {
        let _ = self.playlist.push_song_list(song_list);
    }
    pub fn handle_add_songs_to_playlist_and_play(
        &mut self,
        song_list: Vec<ListSong>,
    ) -> ComponentEffect<Self> {
        let effect = self.playlist.reset();
        let id = self.playlist.push_song_list(song_list);
        effect
            .push(self.playlist.play_song_id(id))
            .map(|this: &mut Self| &mut this.playlist)
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

    async fn global_handle_key_stack(&mut self) -> ComponentEffect<Self> {
        // First handle my own keybinds, otherwise forward if our keybinds are not
        // dominant. TODO: Remove allocation
        match handle_key_stack(self.get_this_keybinds(), self.key_stack.clone()) {
            KeyHandleAction::Action(a) => {
                let effect = a.apply(self).await;
                self.key_stack.clear();
                return effect;
            }
            KeyHandleAction::Mode => {
                return AsyncTask::new_no_op();
            }
            KeyHandleAction::NoMap => {
                if self.is_dominant_keybinds() {
                    self.key_stack.clear();
                    return AsyncTask::new_no_op();
                }
            }
        };
        let subcomponents_outcome = match self.context {
            // TODO: Remove allocation
            WindowContext::Browser => {
                handle_key_stack_and_action(&mut self.browser, self.key_stack.clone())
                    .await
                    .map(|this: &mut Self| &mut this.browser)
            }
            WindowContext::Playlist => {
                handle_key_stack_and_action(&mut self.playlist, self.key_stack.clone())
                    .await
                    .map(|this: &mut Self| &mut this.playlist)
            }
            WindowContext::Logs => {
                handle_key_stack_and_action(&mut self.logger, self.key_stack.clone())
                    .await
                    .map(|this: &mut Self| &mut this.logger)
            }
        };
        let effect = match subcomponents_outcome {
            KeyHandleOutcome::Action(a) => a,
            KeyHandleOutcome::Mode => return AsyncTask::new_no_op(),
            KeyHandleOutcome::NoMap => AsyncTask::new_no_op(),
        };
        self.key_stack.clear();
        effect
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
