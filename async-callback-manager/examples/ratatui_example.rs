//! Example of using async-callback-manager in a ratatui app.

use anyhow::Result;
use async_callback_manager::{AsyncCallbackManager, CallbackSender};
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{prelude::CrosstermBackend, widgets::Paragraph, Terminal};
use std::{io, time::Duration};

struct State {
    left_value: String,
    right_value: String,
    callback_handle: CallbackSender<Backend, Self>,
}

#[derive(Clone)]
struct Backend {
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut events = EventStream::new().map(event_to_action);
    let mut manager = AsyncCallbackManager::new(50);
    let mut state = State {
        left_value: String::new(),
        right_value: String::new(),
        callback_handle: manager.new_sender(50),
    };
    loop {
        tokio::select! {
            Some(Some(action)) = events.next() => match action {
                Action::Quit => break,
                Action::SetLeftA => (),
                Action::SetLeftB => (),
                Action::SetRight => (),
            },
            else => {}
        }
        terminal.draw(|f| {
            let greeting = Paragraph::new("Hello World! (press 'q' to quit)");
            f.render_widget(greeting, f.area());
        })?;
    }
    destruct_terminal()?;
    Ok(())
}

/// Cleanly exit the tui
fn destruct_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(io::stdout(), crossterm::cursor::Show)?;
    Ok(())
}

enum Action {
    Quit,
    SetLeftA,
    SetLeftB,
    SetRight,
}

fn event_to_action(event: Result<Event, std::io::Error>) -> Option<Action> {
    if let Ok(event) = event {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                kind: KeyEventKind::Release,
                ..
            }) => return Some(Action::Quit),
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                kind: KeyEventKind::Release,
                ..
            }) => return Some(Action::SetLeftA),
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                kind: KeyEventKind::Release,
                ..
            }) => return Some(Action::SetLeftB),
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                kind: KeyEventKind::Release,
                ..
            }) => return Some(Action::SetRight),
            _ => (),
        }
    }
    None
}
