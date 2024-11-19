use crate::app::{
    component::actionhandler::{Action, ActionHandler, KeyRouter, TextHandler},
    keycommand::KeyCommand,
    ui::AppCallback,
    view::Drawable,
};
use crate::core::send_or_error;
use crossterm::event::KeyCode;
use draw::draw_logger;
use ratatui::{prelude::Rect, Frame};
use std::borrow::Cow;
use tokio::sync::mpsc::Sender;
use tui_logger::TuiWidgetEvent;

#[derive(Clone, Debug, PartialEq)]
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
    keybinds: Vec<KeyCommand<LoggerAction>>,
}

impl Drawable for Logger {
    fn draw_chunk(&self, f: &mut Frame, chunk: Rect, selected: bool) {
        draw_logger(f, self, chunk, selected)
    }
}

impl KeyRouter<LoggerAction> for Logger {
    // XXX: Duplication of effort here due to trait structure - not the worst.
    fn get_routed_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<LoggerAction>> + 'a> {
        Box::new(self.keybinds.iter())
    }
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<LoggerAction>> + 'a> {
        self.get_routed_keybinds()
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
    fn handle_event_repr(&mut self, event: &crossterm::event::Event) -> bool {
        false
    }
}

impl ActionHandler<LoggerAction> for Logger {
    async fn handle_action(&mut self, action: &LoggerAction) {
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
    }
}

impl Logger {
    pub fn new(ui_tx: Sender<AppCallback>) -> Self {
        Self {
            ui_tx,
            logger_state: tui_logger::TuiWidgetState::default(),
            keybinds: logger_keybinds(),
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

fn logger_keybinds() -> Vec<KeyCommand<LoggerAction>> {
    vec![
        KeyCommand::new_global_from_code(KeyCode::F(5), LoggerAction::ViewBrowser),
        KeyCommand::new_from_code(KeyCode::Char('['), LoggerAction::ReduceCaptured),
        KeyCommand::new_from_code(KeyCode::Char(']'), LoggerAction::IncreaseCaptured),
        KeyCommand::new_from_code(KeyCode::Left, LoggerAction::ReduceShown),
        KeyCommand::new_from_code(KeyCode::Right, LoggerAction::IncreaseShown),
        KeyCommand::new_from_code(KeyCode::Up, LoggerAction::Up),
        KeyCommand::new_from_code(KeyCode::Down, LoggerAction::Down),
        KeyCommand::new_from_code(KeyCode::PageUp, LoggerAction::PageUp),
        KeyCommand::new_from_code(KeyCode::PageDown, LoggerAction::PageDown),
        KeyCommand::new_from_code(KeyCode::Char(' '), LoggerAction::ToggleHideFiltered),
        KeyCommand::new_from_code(KeyCode::Esc, LoggerAction::ExitPageMode),
        KeyCommand::new_from_code(KeyCode::Char('f'), LoggerAction::ToggleTargetFocus),
        KeyCommand::new_from_code(KeyCode::Char('h'), LoggerAction::ToggleTargetSelector),
    ]
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
