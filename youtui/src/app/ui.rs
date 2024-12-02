use self::{browser::Browser, logger::Logger, playlist::Playlist};
use super::component::actionhandler::{
    count_visible_keybinds, handle_key_stack, Action, ComponentEffect, DominantKeyRouter,
    KeyHandleAction, KeyRouter, Keymap, TextHandler,
};
use super::keycommand::{DisplayableCommand, DisplayableMode};
use super::server::{ArcServer, IncreaseVolume, TaskMetadata};
use super::structures::*;
use super::view::Scrollable;
use super::AppCallback;
use crate::async_rodio_sink::{SeekDirection, VolumeUpdate};
use crate::config::Config;
use action::AppAction;
use async_callback_manager::{AsyncTask, Constraint};
use crossterm::event::{Event, KeyEvent};
use itertools::Either;
use ratatui::widgets::TableState;
use std::time::Duration;
use tokio::sync::mpsc;

pub mod action;
pub mod browser;
pub mod draw;
mod footer;
mod header;
pub mod logger;
pub mod playlist;

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

pub struct YoutuiWindow {
    context: WindowContext,
    prev_context: WindowContext,
    pub playlist: Playlist,
    pub browser: Browser,
    pub logger: Logger,
    pub callback_tx: mpsc::Sender<AppCallback>,
    keybinds: Keymap<AppAction>,
    key_stack: Vec<KeyEvent>,
    pub help: HelpMenu,
}
impl_youtui_component!(YoutuiWindow);

pub struct HelpMenu {
    pub shown: bool,
    cur: usize,
    len: usize,
    keybinds: Keymap<AppAction>,
    pub widget_state: TableState,
}

impl HelpMenu {
    fn new(config: &Config) -> Self {
        HelpMenu {
            shown: Default::default(),
            cur: Default::default(),
            len: Default::default(),
            keybinds: help_keybinds(config),
            widget_state: Default::default(),
        }
    }
}
impl_youtui_component!(HelpMenu);

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

impl DominantKeyRouter<AppAction> for YoutuiWindow {
    fn dominant_keybinds_active(&self) -> bool {
        self.help.shown
            || match self.context {
                WindowContext::Browser => self.browser.dominant_keybinds_active(),
                WindowContext::Playlist => false,
                WindowContext::Logs => false,
            }
    }

    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        if self.help.shown {
            return Either::Right(Either::Right(std::iter::once(&self.help.keybinds)));
        }
        match self.context {
            WindowContext::Browser => {
                Either::Left(Either::Left(self.browser.get_dominant_keybinds()))
            }
            WindowContext::Playlist => {
                Either::Left(Either::Right(self.playlist.get_active_keybinds()))
            }
            WindowContext::Logs => Either::Right(Either::Left(self.logger.get_active_keybinds())),
        }
    }
}

impl KeyRouter<AppAction> for YoutuiWindow {
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        // If Browser has dominant keybinds, self keybinds shouldn't be visible.
        let kb = std::iter::once(&self.keybinds);
        match self.context {
            WindowContext::Browser => {
                Either::Left(Either::Left(kb.chain(self.browser.get_active_keybinds())))
            }
            WindowContext::Playlist => {
                Either::Left(Either::Right(kb.chain(self.playlist.get_active_keybinds())))
            }
            WindowContext::Logs => Either::Right(kb.chain(self.logger.get_active_keybinds())),
        }
    }
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        std::iter::once(&self.keybinds)
            .chain(self.browser.get_all_keybinds())
            .chain(self.playlist.get_all_keybinds())
            .chain(self.logger.get_all_keybinds())
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
    fn handle_text_event_impl(&mut self, event: &Event) -> Option<ComponentEffect<Self>> {
        match self.context {
            WindowContext::Browser => self
                .browser
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.browser)),
            WindowContext::Playlist => self
                .playlist
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.playlist)),
            WindowContext::Logs => self
                .logger
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut YoutuiWindow| &mut this.logger)),
        }
    }
}

impl YoutuiWindow {
    pub fn new(
        callback_tx: mpsc::Sender<AppCallback>,
        config: &Config,
    ) -> (YoutuiWindow, ComponentEffect<YoutuiWindow>) {
        let (playlist, task) = Playlist::new(callback_tx.clone(), config);
        let this = YoutuiWindow {
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist,
            browser: Browser::new(callback_tx.clone(), config),
            logger: Logger::new(callback_tx.clone(), config),
            keybinds: global_keybinds(config),
            key_stack: Vec::new(),
            help: HelpMenu::new(config),
            callback_tx,
        };
        (this, task.map(|this: &mut Self| &mut this.playlist))
    }
    // Splitting out event types removes one layer of indentation.
    pub async fn handle_event(&mut self, event: crossterm::event::Event) -> ComponentEffect<Self> {
        if let Some(effect) = self.try_handle_text(&event) {
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
    pub fn pauseplay(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .pauseplay()
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn handle_next(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .handle_next()
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn handle_prev(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .handle_previous()
            .map(|this: &mut Self| &mut this.playlist)
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
    async fn global_handle_key_stack(&mut self) -> ComponentEffect<Self> {
        match handle_key_stack(self.get_active_keybinds(), &self.key_stack) {
            KeyHandleAction::Action(a) => {
                let effect = a.apply(self).await;
                self.key_stack.clear();
                effect
            }
            KeyHandleAction::Mode { .. } => AsyncTask::new_no_op(),
            KeyHandleAction::NoMap => {
                self.key_stack.clear();
                AsyncTask::new_no_op()
            }
        }
    }
    fn key_pending(&self) -> bool {
        !self.key_stack.is_empty()
    }
    pub fn toggle_help(&mut self) {
        if self.help.shown {
            self.help.shown = false;
        } else {
            self.help.shown = true;
            // Setup Help menu parameters
            self.help.cur = 0;
            // We have to get the keybind length this way as the help menu iterator is not
            // ExactSized
            self.help.len = count_visible_keybinds(self);
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
    fn get_cur_displayable_mode(
        &self,
    ) -> Option<DisplayableMode<'_, impl Iterator<Item = DisplayableCommand<'_>>>> {
        let KeyHandleAction::Mode { name, keys } =
            handle_key_stack(self.get_active_keybinds(), &self.key_stack)
        else {
            return None;
        };
        let displayable_commands = keys
            .iter()
            .map(|(kb, kt)| DisplayableCommand::from_command(kb, kt));
        Some(DisplayableMode {
            displayable_commands,
            description: name.into(),
        })
    }
}

fn global_keybinds(config: &Config) -> Keymap<AppAction> {
    config.keybinds.global.clone()
}
fn help_keybinds(config: &Config) -> Keymap<AppAction> {
    config.keybinds.help.clone()
}
