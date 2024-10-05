//! Example of using async-callback-manager in a ratatui app.

use anyhow::Result;
use async_callback_manager::{
    AsyncCallbackManager, BackendStreamingTask, BackendTask, AsyncCallbackSender,
};
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{stream, FutureExt};
use ratatui::{
    layout::{self, Constraint, Layout},
    prelude::CrosstermBackend,
    widgets::{Block, Paragraph},
    Terminal,
};
use std::{io, time::Duration};
use tokio_stream::StreamExt;

struct State {
    word: String,
    number: String,
    callback_handle: AsyncCallbackSender<reqwest::Client, Self>,
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let backend = reqwest::Client::new();
    let mut events = EventStream::new().filter_map(event_to_action);
    let mut manager = AsyncCallbackManager::new(50);
    let mut state = State {
        word: String::new(),
        number: String::new(),
        callback_handle: manager.new_sender(50),
    };
    loop {
        terminal.draw(|f| {
            let greeting = Paragraph::new("Hello World! (press 'q' to quit, 'j' to get a random word, 'k' to count from 1 to 10)").block(Block::bordered());
            let word = Paragraph::new(format!("Word: {}", state.word)).block(Block::bordered());
            let number = Paragraph::new(format!("Number: {}", state.number)).block(Block::bordered());
            let [top, bottom] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(f.area());
            let [left, right] = 
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(bottom);
            f.render_widget(greeting, top);
            f.render_widget(word, left);
            f.render_widget(number, right);
        })?;
        tokio::select! {
            Some(action) = events.next() => match action {
                Action::Quit => break,
                Action::SetLeft => {
                    state.word = "Loading".to_string();
                    state.callback_handle.add_callback(
                        GetBirdRequest,
                        |state, bird| state.word = bird,
                        Some(async_callback_manager::Constraint::new_block_same_type()),
                    ).await.unwrap()
                },
                Action::SetRight => {
                    state.number = "Loading".to_string();
                    state.callback_handle.add_stream_callback(
                        CounterStream,
                        |state, num| state.number = num,
                        Some(async_callback_manager::Constraint::new_block_same_type()),
                    ).await.unwrap()
                },
            },
            _ = manager.manage_next_event(backend.clone()) => (),
            mutations = state.callback_handle.get_next_mutations(10) => mutations.apply(&mut state),
        }
    }
    destruct_terminal()?;
    Ok(())
}

struct GetBirdRequest;
impl BackendTask<reqwest::Client> for GetBirdRequest {
    type Output = String;
    async fn into_future(self, backend: reqwest::Client) -> Self::Output {
        backend
            .get("https://random-word-api.herokuapp.com/word")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }
}

struct CounterStream;
impl<T> BackendStreamingTask<T> for CounterStream {
    type Output = String;
    fn into_stream(self, backend: T) -> impl futures::Stream<Item = Self::Output> + Send + Unpin {
        stream::iter(1..11).map(|x| x.to_string()).then(|x| {
            tokio::time::sleep(Duration::from_millis(500))
                .map(|_| x)
                .boxed()
        })
    }
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
    SetLeft,
    SetRight,
}

fn event_to_action(event: Result<Event, std::io::Error>) -> Option<Action> {
    if let Ok(event) = event {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                kind: KeyEventKind::Press,
                ..
            }) => return Some(Action::Quit),
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                kind: KeyEventKind::Press,
                ..
            }) => return Some(Action::SetLeft),
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                kind: KeyEventKind::Press,
                ..
            }) => return Some(Action::SetRight),
            _ => (),
        }
    }
    None
}
