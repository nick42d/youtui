//! Example of using async-callback-manager in a ratatui app.
#![allow(clippy::unwrap_used)]

use async_callback_manager::{
    AsyncCallbackManager, AsyncTask, BackendStreamingTask, BackendTask, TaskHandler, TaskOutcome,
};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use futures::{FutureExt, stream};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Paragraph};
use std::future::Future;
use std::time::Duration;
use tokio_stream::StreamExt;

#[derive(Default, Debug)]
enum Mode {
    BlockPreviousTasks,
    KillPreviousTasks,
    #[default]
    Unhandled,
}
impl Mode {
    fn toggle(&self) -> Self {
        match self {
            Mode::BlockPreviousTasks => Mode::KillPreviousTasks,
            Mode::KillPreviousTasks => Mode::Unhandled,
            Mode::Unhandled => Mode::BlockPreviousTasks,
        }
    }
}
impl<T> From<&Mode> for Option<async_callback_manager::Constraint<T>> {
    fn from(value: &Mode) -> Self {
        match value {
            Mode::BlockPreviousTasks => {
                Some(async_callback_manager::Constraint::new_block_same_type())
            }
            Mode::KillPreviousTasks => {
                Some(async_callback_manager::Constraint::new_kill_same_type())
            }
            Mode::Unhandled => None,
        }
    }
}
struct State {
    word: String,
    number: String,
    mode: Mode,
}
impl State {
    fn draw(&self, f: &mut Frame) {
        let greeting = Paragraph::new(
            format!("Hello World! (press 'q' to quit, 'j' to get a random word, 'k' to count from 1 to 10)\n
            Race condition handling mode is {:?}, press 't' to toggle.",
            self.mode)
        )
        .block(Block::bordered());
        let word = Paragraph::new(format!("Word: {}", self.word)).block(Block::bordered());
        let number = Paragraph::new(format!("Number: {}", self.number)).block(Block::bordered());
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(f.area());
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(bottom);
        f.render_widget(greeting, top);
        f.render_widget(word, left);
        f.render_widget(number, right);
    }
    fn handle_toggle_mode(&mut self) {
        self.mode = self.mode.toggle()
    }
    async fn handle_get_word(&mut self) -> AsyncTask<Self, reqwest::Client, ()> {
        self.word = "Loading".to_string();
        #[derive(Debug, PartialEq)]
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        struct Handler;
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        impl TaskHandler<String, State, reqwest::Client, ()> for Handler {
            fn handle(
                self,
                input: String,
            ) -> impl async_callback_manager::FrontendEffect<State, reqwest::Client, ()>
            {
                |state: &mut State| state.word = input
            }
        }
        #[cfg(not(any(feature = "task-equality", feature = "task-debug")))]
        let handler = |state: &mut Self, word| state.word = word;
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        let handler = Handler;

        AsyncTask::new_future(GetWordRequest, handler, (&self.mode).into())
    }
    async fn handle_start_counter(&mut self) -> AsyncTask<Self, reqwest::Client, ()> {
        self.number = "Loading".to_string();
        #[derive(Debug, PartialEq, Clone)]
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        struct Handler;
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        impl TaskHandler<String, State, reqwest::Client, ()> for Handler {
            fn handle(
                self,
                input: String,
            ) -> impl async_callback_manager::FrontendEffect<State, reqwest::Client, ()>
            {
                |state: &mut State| state.number = input
            }
        }
        #[cfg(not(any(feature = "task-equality", feature = "task-debug")))]
        let handler = |state: &mut Self, num| state.number = num;
        #[cfg(any(feature = "task-equality", feature = "task-debug"))]
        let handler = Handler;

        AsyncTask::new_stream(CounterStream, handler, (&self.mode).into())
    }
}

#[tokio::main]
async fn main() {
    let mut terminal = ratatui::init();
    let backend = reqwest::Client::new();
    let mut events = EventStream::new().filter_map(event_to_action);
    let mut manager = AsyncCallbackManager::new();
    let mut state = State {
        word: String::new(),
        number: String::new(),
        mode: Default::default(),
    };
    loop {
        terminal.draw(|f| state.draw(f)).unwrap();
        tokio::select! {
            Some(action) = events.next() => match action {
                Action::Quit => break,
                Action::GetWord => {
                    manager.spawn_task(&backend,
                    state.handle_get_word().await)
                },
                Action::StartCounter => {
                    manager.spawn_task(&backend,
                    state.handle_start_counter().await)
                },
                Action::ToggleMode => state.handle_toggle_mode(),
            },
            Some(outcome) = manager.get_next_response() => match outcome {
                TaskOutcome::StreamFinished {..} => continue,
                TaskOutcome::TaskPanicked {error,..}|TaskOutcome::StreamPanicked { error, ..} => std::panic::resume_unwind(error.into_panic()),
                TaskOutcome::MutationReceived { mutation, ..} =>
                    manager.spawn_task(&backend, mutation(&mut state)),
            },
        };
    }
    ratatui::restore();
}

#[derive(Debug, PartialEq)]
struct GetWordRequest;
impl BackendTask<reqwest::Client> for GetWordRequest {
    type MetadataType = ();
    type Output = String;
    fn into_future(
        self,
        backend: &reqwest::Client,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move {
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
}

#[derive(Debug, PartialEq)]
struct CounterStream;
impl<T> BackendStreamingTask<T> for CounterStream {
    type Output = String;
    type MetadataType = ();
    fn into_stream(
        self,
        _: &T,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        stream::iter(1..11).map(|x| x.to_string()).then(|x| {
            tokio::time::sleep(Duration::from_millis(500))
                .map(|_| x)
                .boxed()
        })
    }
}

enum Action {
    Quit,
    GetWord,
    StartCounter,
    ToggleMode,
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
            }) => return Some(Action::GetWord),
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                kind: KeyEventKind::Press,
                ..
            }) => return Some(Action::StartCounter),
            Event::Key(KeyEvent {
                code: KeyCode::Char('t'),
                kind: KeyEventKind::Press,
                ..
            }) => return Some(Action::ToggleMode),
            _ => (),
        }
    }
    None
}
