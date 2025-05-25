use self::browser::Browser;
use self::logger::Logger;
use self::playlist::Playlist;
use super::component::actionhandler::{
    apply_action_mapped, get_visible_keybinds_as_readable_iter, handle_key_stack, ActionHandler,
    ComponentEffect, DominantKeyRouter, KeyHandleAction, KeyRouter, Scrollable, TextHandler,
    YoutuiEffect,
};
use super::server::{IncreaseVolume, SetVolume};
use super::structures::*;
use super::AppCallback;
use crate::async_rodio_sink::{SeekDirection, VolumeUpdate};
use crate::config::keymap::Keymap;
use crate::config::Config;
use crate::keyaction::{DisplayableKeyAction, DisplayableMode};
use action::{AppAction, ListAction, TextEntryAction, PAGE_KEY_LINES, SEEK_AMOUNT};
use async_callback_manager::{AsyncTask, Constraint};
use crossterm::event::{Event, KeyEvent};
use itertools::Either;
use ratatui::widgets::TableState;
use std::time::Duration;

pub mod action;
pub mod browser;
pub mod draw;
pub mod draw_media_controls;
mod footer;
mod header;
pub mod logger;
pub mod playlist;

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
    playlist: Playlist,
    browser: Browser,
    logger: Logger,
    config: Config,
    key_stack: Vec<KeyEvent>,
    help: HelpMenu,
}
impl_youtui_component!(YoutuiWindow);

pub struct HelpMenu {
    pub shown: bool,
    cur: usize,
    len: usize,
    pub widget_state: TableState,
}

impl HelpMenu {
    fn new() -> Self {
        HelpMenu {
            shown: Default::default(),
            cur: Default::default(),
            len: Default::default(),
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
    fn is_scrollable(&self) -> bool {
        true
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

    fn get_dominant_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        if self.help.shown {
            return Either::Right(Either::Right(
                [&config.keybinds.help, &config.keybinds.list].into_iter(),
            ));
        }
        match self.context {
            WindowContext::Browser => {
                Either::Left(Either::Left(self.browser.get_dominant_keybinds(config)))
            }
            WindowContext::Playlist => {
                Either::Left(Either::Right(self.playlist.get_active_keybinds(config)))
            }
            WindowContext::Logs => {
                Either::Right(Either::Left(self.logger.get_active_keybinds(config)))
            }
        }
    }
}

impl Scrollable for YoutuiWindow {
    fn increment_list(&mut self, amount: isize) {
        if self.help.shown {
            return self.help.increment_list(amount);
        }
        match self.context {
            WindowContext::Browser => self.browser.increment_list(amount),
            WindowContext::Playlist => self.playlist.increment_list(amount),
            WindowContext::Logs => (),
        }
    }
    fn is_scrollable(&self) -> bool {
        self.help.shown
            || match self.context {
                WindowContext::Browser => self.browser.is_scrollable(),
                WindowContext::Playlist => self.playlist.is_scrollable(),
                WindowContext::Logs => false,
            }
    }
}

impl KeyRouter<AppAction> for YoutuiWindow {
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        let kb = if self.is_scrollable() {
            Either::Left(std::iter::once(&config.keybinds.list))
        } else {
            Either::Right(std::iter::empty())
        };
        if self.dominant_keybinds_active() {
            return Either::Right(Either::Right(self.get_dominant_keybinds(config).chain(kb)));
        }
        let kb = kb.chain(std::iter::once(&config.keybinds.global));
        let kb = if self.is_text_handling() {
            Either::Left(kb.chain(std::iter::once(&config.keybinds.text_entry)))
        } else {
            Either::Right(kb)
        };
        match self.context {
            WindowContext::Browser => Either::Left(Either::Left(
                kb.chain(self.browser.get_active_keybinds(config)),
            )),
            WindowContext::Playlist => Either::Left(Either::Right(
                kb.chain(self.playlist.get_active_keybinds(config)),
            )),
            WindowContext::Logs => Either::Right(Either::Left(
                kb.chain(self.logger.get_active_keybinds(config)),
            )),
        }
    }
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        std::iter::once(&config.keybinds.global)
            .chain(self.browser.get_all_keybinds(config))
            .chain(self.playlist.get_all_keybinds(config))
            .chain(self.logger.get_all_keybinds(config))
    }
}

impl TextHandler for YoutuiWindow {
    fn is_text_handling(&self) -> bool {
        if self.help.shown {
            return false;
        }
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

impl ActionHandler<AppAction> for YoutuiWindow {
    fn apply_action(&mut self, action: AppAction) -> impl Into<YoutuiEffect<Self>> {
        // NOTE: This is the place to check if we _should_ be handling an action.
        // For example if a user has set custom 'playlist' keybinds that trigger
        // 'browser' actions, but browser is not shown currently, this could be filtered
        // out here.
        match action {
            AppAction::VolUp => {
                return Into::<YoutuiEffect<Self>>::into(self.handle_increase_volume(5))
            }
            AppAction::VolDown => return self.handle_increase_volume(-5).into(),
            AppAction::NextSong => return self.handle_next().into(),
            AppAction::PrevSong => return self.handle_prev().into(),
            AppAction::SeekForward => {
                return self.handle_seek(SEEK_AMOUNT, SeekDirection::Forward).into()
            }
            AppAction::SeekBack => {
                return self.handle_seek(SEEK_AMOUNT, SeekDirection::Back).into()
            }
            AppAction::ToggleHelp => self.toggle_help(),
            AppAction::Quit => return (AsyncTask::new_no_op(), Some(AppCallback::Quit)).into(),
            AppAction::ViewLogs => self.handle_change_context(WindowContext::Logs),
            AppAction::PlayPause => return self.pauseplay().into(),
            AppAction::Log(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.logger)
            }
            AppAction::Playlist(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.playlist)
            }
            AppAction::Browser(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::Filter(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::Sort(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::Help(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.help)
            }
            AppAction::BrowserArtists(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::BrowserSearch(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::BrowserArtistSongs(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::BrowserSongs(a) => {
                return apply_action_mapped(self, a, |this: &mut Self| &mut this.browser)
            }
            AppAction::TextEntry(a) => return self.handle_text_entry_action(a).into(),
            AppAction::List(a) => return self.handle_list_action(a).into(),
            AppAction::NoOp => (),
        };
        AsyncTask::new_no_op().into()
    }
}

impl YoutuiWindow {
    pub fn new(config: Config) -> (YoutuiWindow, ComponentEffect<YoutuiWindow>) {
        let (playlist, task) = Playlist::new();
        let this = YoutuiWindow {
            context: WindowContext::Browser,
            prev_context: WindowContext::Browser,
            playlist,
            config,
            browser: Browser::new(),
            logger: Logger::new(),
            key_stack: Vec::new(),
            help: HelpMenu::new(),
        };
        (this, task.map(|this: &mut Self| &mut this.playlist))
    }
    pub fn get_help_list_items(&self) -> impl Iterator<Item = DisplayableKeyAction<'_>> {
        match self.context {
            WindowContext::Browser => Either::Left(Either::Right(
                get_visible_keybinds_as_readable_iter(self.browser.get_all_keybinds(&self.config)),
            )),
            WindowContext::Playlist => Either::Right(get_visible_keybinds_as_readable_iter(
                self.playlist.get_all_keybinds(&self.config),
            )),
            WindowContext::Logs => Either::Left(Either::Left(
                get_visible_keybinds_as_readable_iter(self.logger.get_all_keybinds(&self.config)),
            )),
        }
        .chain(get_visible_keybinds_as_readable_iter(
            std::iter::once(&self.config.keybinds.global)
                .chain(std::iter::once(&self.config.keybinds.list))
                .chain(std::iter::once(&self.config.keybinds.text_entry)),
        ))
    }
    pub async fn handle_crossterm_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> YoutuiEffect<Self> {
        // TODO: This should be intercepted and keycodes mapped by us instead of going
        // direct to rat-text.
        if let Some(effect) = self.try_handle_text(&event) {
            return effect.into();
        };
        // Splitting out event types removes one layer of indentation.
        match event {
            Event::Key(k) => return self.handle_key_event(k),
            Event::Mouse(m) => return self.handle_mouse_event(m).into(),
            other => tracing::warn!("Received unimplemented {:?} event", other),
        }
        AsyncTask::new_no_op().into()
    }
    pub async fn handle_media_controls_event(
        &mut self,
        event: souvlaki::MediaControlEvent,
    ) -> YoutuiEffect<Self> {
        // This conversion function is written here as this is expected to be the only
        // location it is used.
        let convert_dir = |dir| match dir {
            souvlaki::SeekDirection::Forward => SeekDirection::Forward,
            souvlaki::SeekDirection::Backward => SeekDirection::Back,
        };
        match event {
            souvlaki::MediaControlEvent::Play => return self.resume().into(),
            souvlaki::MediaControlEvent::Pause => return self.pause().into(),
            souvlaki::MediaControlEvent::Toggle => return self.pauseplay().into(),
            souvlaki::MediaControlEvent::Next => return self.handle_next().into(),
            souvlaki::MediaControlEvent::Previous => return self.handle_prev().into(),
            souvlaki::MediaControlEvent::Stop => return self.stop().into(),
            souvlaki::MediaControlEvent::Seek(seek_direction) => {
                return self
                    .handle_seek(SEEK_AMOUNT, convert_dir(seek_direction))
                    .into()
            }
            souvlaki::MediaControlEvent::SeekBy(seek_direction, duration) => {
                return self
                    .handle_seek(duration, convert_dir(seek_direction))
                    .into()
            }
            souvlaki::MediaControlEvent::SetPosition(media_position) => {
                return self.handle_seek_to(media_position.0).into()
            }
            souvlaki::MediaControlEvent::SetVolume(v) => {
                return self.handle_set_volume((v * 100.0) as u8).into()
            }
            souvlaki::MediaControlEvent::Quit => {
                return (AsyncTask::new_no_op(), Some(AppCallback::Quit)).into()
            }
            souvlaki::MediaControlEvent::OpenUri(_) => {
                tracing::info!("Received intentionally unhandled event {:?}", event)
            }
            souvlaki::MediaControlEvent::Raise => {
                tracing::info!("Received intentionally unhandled event {:?}", event)
            }
        }
        AsyncTask::new_no_op().into()
    }
    pub async fn handle_tick(&mut self) {
        self.playlist.handle_tick().await;
    }
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> YoutuiEffect<Self> {
        self.key_stack.push(key_event);
        self.global_handle_key_stack()
    }
    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> ComponentEffect<Self> {
        tracing::warn!("Received unimplemented {:?} mouse event", mouse_event);
        AsyncTask::new_no_op()
    }
    pub fn handle_list_action(&mut self, action: ListAction) -> ComponentEffect<Self> {
        if self.is_scrollable() {
            match action {
                ListAction::Up => self.increment_list(-1),
                ListAction::Down => self.increment_list(1),
                ListAction::PageUp => self.increment_list(-PAGE_KEY_LINES),
                ListAction::PageDown => self.increment_list(PAGE_KEY_LINES),
            }
        }
        AsyncTask::new_no_op()
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        if !self.is_text_handling() {
            return AsyncTask::new_no_op();
        }
        match self.context {
            WindowContext::Browser => self
                .browser
                .handle_text_entry_action(action)
                .map(|this: &mut Self| &mut this.browser),
            WindowContext::Playlist => AsyncTask::new_no_op(),
            WindowContext::Logs => AsyncTask::new_no_op(),
        }
    }
    pub fn pauseplay(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .pauseplay()
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn resume(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .resume()
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn pause(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .pause()
            .map(|this: &mut Self| &mut this.playlist)
    }
    pub fn stop(&mut self) -> ComponentEffect<Self> {
        self.playlist
            .stop()
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
    pub fn handle_increase_volume(&mut self, inc: i8) -> ComponentEffect<Self> {
        // Visually update the state first for instant feedback.
        self.increase_volume(inc);
        AsyncTask::new_future(
            IncreaseVolume(inc),
            Self::handle_volume_update,
            Some(Constraint::new_block_same_type()),
        )
    }
    pub fn handle_set_volume(&mut self, new_vol: u8) -> ComponentEffect<Self> {
        // Visually update the state first for instant feedback.
        self.set_volume(new_vol);
        AsyncTask::new_future(
            SetVolume(new_vol),
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
    pub fn handle_seek_to(&mut self, position: Duration) -> ComponentEffect<Self> {
        self.playlist
            .handle_seek_to(position)
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
    fn global_handle_key_stack(&mut self) -> YoutuiEffect<Self> {
        match handle_key_stack(self.get_active_keybinds(&self.config), &self.key_stack) {
            KeyHandleAction::Action(a) => {
                let effect = self.apply_action(a).into();
                self.key_stack.clear();
                effect
            }
            KeyHandleAction::Mode { .. } => AsyncTask::new_no_op().into(),
            KeyHandleAction::NoMap => {
                self.key_stack.clear();
                AsyncTask::new_no_op().into()
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
            self.help.len = self.get_help_list_items().count();
        }
    }
    /// Visually increment the volume, note, does not actually change the
    /// volume.
    fn increase_volume(&mut self, inc: i8) {
        self.playlist.increase_volume(inc);
    }
    /// Visually set the volume, note, does not actually change the volume.
    fn set_volume(&mut self, new_vol: u8) {
        self.playlist.set_volume(new_vol);
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
    ) -> Option<DisplayableMode<'_, impl Iterator<Item = DisplayableKeyAction<'_>>>> {
        let KeyHandleAction::Mode { name, keys } =
            handle_key_stack(self.get_active_keybinds(&self.config), &self.key_stack)
        else {
            return None;
        };
        let displayable_commands = keys
            .iter()
            .map(|(kb, kt)| DisplayableKeyAction::from_keybind_and_action_tree(kb, kt));
        Some(DisplayableMode {
            displayable_commands,
            description: name.into(),
        })
    }
}
