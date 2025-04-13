use crate::app::component::actionhandler::{ActionHandler, YoutuiEffect};
use crate::config::keymap::Keymap;
use crate::core::send_or_error;
use crate::{
    app::{
        component::actionhandler::{Action, ComponentEffect, KeyRouter, TextHandler},
        server::{ArcServer, TaskMetadata},
        ui::AppCallback,
        view::Drawable,
    },
    config::Config,
};
use async_callback_manager::AsyncTask;
use draw::draw_logger;
use ratatui::{prelude::Rect, Frame};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tokio::sync::mpsc::Sender;
use tui_logger::TuiWidgetEvent;

use super::action::AppAction;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggerAction {
    ToggleTargetSelector,
    ToggleTargetFocus,
    ToggleHideFiltered,
    Up,
    Down,
    PageUp,
    PageDown,
    ReduceShown,
    IncreaseShown,
    ReduceCaptured,
    IncreaseCaptured,
    ExitPageMode,
    ViewBrowser,
}
impl Action for LoggerAction {
    type State = Logger;
    fn context(&self) -> Cow<str> {
        "Logger".into()
    }
    fn describe(&self) -> Cow<str> {
        match self {
            LoggerAction::ViewBrowser => "View Browser".into(),
            LoggerAction::ToggleTargetSelector => "Toggle Target Selector Widget".into(),
            LoggerAction::ToggleTargetFocus => "Toggle Focus Selected Target".into(),
            LoggerAction::ToggleHideFiltered => "Toggle Hide Filtered Targets".into(),
            LoggerAction::Up => "Up - Selector".into(),
            LoggerAction::Down => "Down - Selector".into(),
            LoggerAction::PageUp => "Enter Page Mode, Scroll History Up".into(),
            LoggerAction::PageDown => "In Page Mode: Scroll History Down".into(),
            LoggerAction::ReduceShown => "Reduce SHOWN (!) Messages".into(),
            LoggerAction::IncreaseShown => "Increase SHOWN (!) Messages".into(),
            LoggerAction::ReduceCaptured => "Reduce CAPTURED (!) Messages".into(),
            LoggerAction::IncreaseCaptured => "Increase CAPTURED (!) Messages".into(),
            LoggerAction::ExitPageMode => "Exit Page Mode".into(),
        }
    }
}
pub struct Logger {
    logger_state: tui_logger::TuiWidgetState,
    ui_tx: Sender<AppCallback>,
    keybinds: Keymap<AppAction>,
}
impl_youtui_component!(Logger);

impl ActionHandler<LoggerAction> for Logger {
    async fn apply_action(&mut self, action: LoggerAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            LoggerAction::ToggleTargetSelector => self.handle_toggle_target_selector(),
            LoggerAction::ToggleTargetFocus => self.handle_toggle_target_focus(),
            LoggerAction::ToggleHideFiltered => self.handle_toggle_hide_filtered(),
            LoggerAction::Up => self.handle_up(),
            LoggerAction::Down => self.handle_down(),
            LoggerAction::PageUp => self.handle_pgup(),
            LoggerAction::PageDown => self.handle_pgdown(),
            LoggerAction::ReduceShown => self.handle_reduce_shown(),
            LoggerAction::IncreaseShown => self.handle_increase_shown(),
            LoggerAction::ReduceCaptured => self.handle_reduce_captured(),
            LoggerAction::IncreaseCaptured => self.handle_increase_captured(),
            LoggerAction::ExitPageMode => self.handle_exit_page_mode(),
            LoggerAction::ViewBrowser => self.handle_view_browser().await,
        }
        AsyncTask::new_no_op()
    }
}
impl Drawable for Logger {
    fn draw_chunk(&self, f: &mut Frame, chunk: Rect, selected: bool) {
        draw_logger(f, self, chunk, selected)
    }
}

impl KeyRouter<AppAction> for Logger {
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        std::iter::once(&self.keybinds)
    }
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        self.get_active_keybinds()
    }
}

impl TextHandler for Logger {
    fn is_text_handling(&self) -> bool {
        false
    }
    fn get_text(&self) -> &str {
        Default::default()
    }
    fn replace_text(&mut self, _text: impl Into<String>) {}
    fn clear_text(&mut self) -> bool {
        false
    }
    fn handle_text_event_impl(
        &mut self,
        _event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        None
    }
}

impl Logger {
    pub fn new(ui_tx: Sender<AppCallback>, config: &Config) -> Self {
        Self {
            ui_tx,
            logger_state: tui_logger::TuiWidgetState::default(),
            keybinds: logger_keybinds(config),
        }
    }
    async fn handle_view_browser(&mut self) {
        send_or_error(
            &self.ui_tx,
            AppCallback::ChangeContext(super::WindowContext::Browser),
        )
        .await;
    }
    fn handle_toggle_hide_filtered(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::SpaceKey);
    }
    fn handle_down(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::DownKey);
    }
    fn handle_up(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::UpKey);
    }
    fn handle_pgdown(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::NextPageKey);
    }
    fn handle_pgup(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::PrevPageKey);
    }
    fn handle_reduce_shown(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::LeftKey);
    }
    fn handle_increase_shown(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::RightKey);
    }
    fn handle_exit_page_mode(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::EscapeKey);
    }
    fn handle_increase_captured(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::PlusKey);
    }
    fn handle_reduce_captured(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::MinusKey);
    }
    fn handle_toggle_target_focus(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::FocusKey);
    }
    fn handle_toggle_target_selector(&mut self) {
        self.logger_state.transition(TuiWidgetEvent::HideKey);
    }
}

fn logger_keybinds(config: &Config) -> Keymap<AppAction> {
    config.keybinds.log.clone()
}

pub mod draw {
    use super::Logger;
    use crate::drawutils::{DESELECTED_BORDER_COLOUR, SELECTED_BORDER_COLOUR};
    use ratatui::{
        prelude::Rect,
        style::{Color, Style},
        Frame,
    };

    pub fn draw_logger(f: &mut Frame, l: &Logger, chunk: Rect, selected: bool) {
        let border_colour = if selected {
            SELECTED_BORDER_COLOUR
        } else {
            DESELECTED_BORDER_COLOUR
        };
        let log = tui_logger::TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .border_style(Style::default().fg(border_colour))
            .state(&l.logger_state)
            .output_timestamp(Some("%H:%M:%S:%3f".to_string()));
        f.render_widget(log, chunk);
    }
}
