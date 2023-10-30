use crate::get_data_dir;

use super::appevent::{AppEvent, EventHandler};
use super::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc};
use tracing::info;
use tracing_subscriber::prelude::*;
use ui::YoutuiWindow;

mod player;
mod server;
mod ui;

const EVENT_CHANNEL_SIZE: usize = 256;
const PLAYER_CHANNEL_SIZE: usize = 256;
const LOG_FILE_NAME: &str = "debug.log";

pub struct Youtui {
    event_handler: EventHandler,
    window_state: YoutuiWindow,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    player: player::PlayerManager,
}

fn destruct_terminal() {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    execute!(io::stdout(), crossterm::cursor::Show).unwrap();
}

impl Youtui {
    pub fn new() -> Result<Youtui> {
        // TODO: Handle errors
        // Setup tracing and link to tui_logger.
        let tui_logger_layer = tui_logger::tracing_subscriber_layer();
        let log_file = std::fs::File::create(get_data_dir()?.join(LOG_FILE_NAME))?;
        let log_file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(log_file));
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing::Level::INFO);
        tracing_subscriber::registry()
            .with(tui_logger_layer.and_then(log_file_layer))
            .with(context_layer)
            .init();
        info!("Starting");
        // Setup terminal
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        // Ensure clean return to shell if panic.
        std::panic::set_hook(Box::new(|panic_info| {
            destruct_terminal();
            println!("{}", panic_info);
        }));
        // First cut at setting up Player
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(PLAYER_CHANNEL_SIZE);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(PLAYER_CHANNEL_SIZE);
        let player = player::PlayerManager::new(response_tx, request_rx).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).unwrap();
        let event_handler = EventHandler::new(EVENT_CHANNEL_SIZE)?;
        // With current setup for Player YoutuiWindow needs the sender/reciever channels to
        // give to Playlist.
        let window_state = YoutuiWindow::new(request_tx, response_rx);
        Ok(Youtui {
            terminal,
            event_handler,
            window_state,
            player,
        })
    }
    pub async fn run(&mut self) {
        while self.window_state.status == ui::AppStatus::Running {
            let msg = self.event_handler.next().await;
            self.process_message(msg).await;
            // Write to terminal, using UI state as the input
            // We draw after handling the event, as the event could be a keypress we want to instantly react to.
            // TODO: Error handling
            self.terminal
                .draw(|f| {
                    ui::draw::draw_app(f, &self.window_state);
                })
                .unwrap();
        }
    }
    async fn process_message(&mut self, msg: Option<AppEvent>) {
        // TODO: Handle closed channel
        match msg {
            Some(AppEvent::QuitSignal) => unimplemented!("Signal to quit recieved, unhandled"),
            Some(AppEvent::Crossterm(e)) => self.window_state.handle_event(e).await,
            // XXX: Should be try_poll or similar? Poll the Future but don't await it?
            Some(AppEvent::Tick) => self.window_state.handle_tick().await,
            None => panic!("Channel closed"),
        }
    }
}
