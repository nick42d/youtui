use crate::app::{
    component::actionhandler::{
        Action, ActionHandler, ActionProcessor, KeyHandler, KeyRouter, Keybind, TextHandler,
    },
    ui::AppCallback,
    view::Drawable,
};
use crate::core::send_or_error;
use crossterm::event::KeyCode;
use draw::draw_logger;
use ratatui::{
    prelude::{Backend, Rect},
    Frame,
};
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
            // TODO: Improve this from Debug implementation
            _ => format!("{:?}", self).into(),
        }
    }
}
pub struct Logger {
    logger_state: tui_logger::TuiWidgetState,
    ui_tx: Sender<AppCallback>,
    keybinds: Vec<Keybind<LoggerAction>>,
}

impl Drawable for Logger {
    fn draw_chunk<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        draw_logger(f, self, chunk)
    }
}

impl KeyHandler<LoggerAction> for Logger {
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<LoggerAction>> + 'a> {
        Box::new(self.keybinds.iter())
    }
}

impl TextHandler for Logger {
    fn push_text(&mut self, _c: char) {}
    fn pop_text(&mut self) {}
    fn is_text_handling(&self) -> bool {
        false
    }
    fn take_text(&mut self) -> String {
        Default::default()
    }
    fn replace_text(&mut self, _text: String) {}
}

impl KeyRouter<LoggerAction> for Logger {
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<LoggerAction>> + 'a> {
        self.get_keybinds()
    }
}

impl ActionProcessor<LoggerAction> for Logger {}

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
        self.logger_state.transition(&TuiWidgetEvent::SpaceKey);
    }
    fn handle_down(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::DownKey);
    }
    fn handle_up(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::UpKey);
    }
    fn handle_pgdown(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::NextPageKey);
    }
    fn handle_pgup(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::PrevPageKey);
    }
    fn handle_reduce_shown(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::LeftKey);
    }
    fn handle_increase_shown(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::RightKey);
    }
    fn handle_exit_page_mode(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::EscapeKey);
    }
    fn handle_increase_captured(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::PlusKey);
    }
    fn handle_reduce_captured(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::MinusKey);
    }
    fn handle_toggle_target_focus(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::FocusKey);
    }
    fn handle_toggle_target_selector(&mut self) {
        self.logger_state.transition(&TuiWidgetEvent::HideKey);
    }
}

fn logger_keybinds() -> Vec<Keybind<LoggerAction>> {
    vec![
        Keybind::new_global_from_code(KeyCode::F(5), LoggerAction::ViewBrowser),
        Keybind::new_from_code(KeyCode::Char('['), LoggerAction::ReduceCaptured),
        Keybind::new_from_code(KeyCode::Char(']'), LoggerAction::IncreaseCaptured),
        Keybind::new_from_code(KeyCode::Left, LoggerAction::ReduceShown),
        Keybind::new_from_code(KeyCode::Right, LoggerAction::IncreaseShown),
        Keybind::new_from_code(KeyCode::Up, LoggerAction::Up),
        Keybind::new_from_code(KeyCode::Down, LoggerAction::Down),
        Keybind::new_from_code(KeyCode::PageUp, LoggerAction::PageUp),
        Keybind::new_from_code(KeyCode::PageDown, LoggerAction::PageDown),
        Keybind::new_from_code(KeyCode::Char(' '), LoggerAction::ToggleHideFiltered),
        Keybind::new_from_code(KeyCode::Esc, LoggerAction::ExitPageMode),
        Keybind::new_from_code(KeyCode::Char('f'), LoggerAction::ToggleTargetFocus),
        Keybind::new_from_code(KeyCode::Char('h'), LoggerAction::ToggleTargetSelector),
    ]
}

pub mod draw {
    use ratatui::{
        prelude::{Backend, Constraint, Direction, Layout, Rect},
        style::{Color, Style},
        Frame,
    };

    use super::Logger;

    pub fn draw_logger<B>(f: &mut Frame<B>, l: &Logger, chunk: Rect)
    where
        B: Backend,
    {
        let log = tui_logger::TuiLoggerSmartWidget::default()
            .border_style(Style::default().fg(Color::Cyan))
            .state(&l.logger_state)
            .output_timestamp(Some("%H:%M:%S:%3f".to_string()));
        f.render_widget(log, chunk);
    }
    /// helper function to create a centered rect using up certain percentage of the available rect `r`
    fn _centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1])[1]
    }
}
