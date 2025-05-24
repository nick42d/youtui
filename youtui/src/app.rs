use super::appevent::{AppEvent, EventHandler};
use crate::core::get_limited_sequential_file;
use crate::{get_data_dir, RuntimeInfo};
use anyhow::Result;
use async_callback_manager::{AsyncCallbackManager, TaskOutcome};
use component::actionhandler::YoutuiEffect;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use media_controls::MediaController;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use server::{ArcServer, Server, TaskMetadata};
use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use structures::ListSong;
use tracing::{error, info};
use tracing_subscriber::prelude::*;
use ui::{WindowContext, YoutuiWindow};

#[macro_use]
pub mod component;
mod media_controls;
mod server;
mod structures;
pub mod ui;
pub mod view;

// We need this thread_local to ensure we know which is the main thread. Panic
// hook that destructs terminal should only run on the main thread.
thread_local! {
    static IS_MAIN_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const CALLBACK_CHANNEL_SIZE: usize = 64;
const EVENT_CHANNEL_SIZE: usize = 256;
const LOG_FILE_NAME: &str = "debug";
const LOG_FILE_EXT: &str = "log";
const MAX_LOG_FILES: u16 = 5;

pub struct Youtui {
    status: AppStatus,
    event_handler: EventHandler,
    window_state: YoutuiWindow,
    task_manager: AsyncCallbackManager<YoutuiWindow, ArcServer, TaskMetadata>,
    server: Arc<Server>,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    media_controls: MediaController,
}

#[derive(PartialEq)]
pub enum AppStatus {
    Running,
    // Cow: Message
    Exiting(Cow<'static, str>),
}

// A callback from one of the application components to the top level.
#[derive(Debug)]
#[must_use]
pub enum AppCallback {
    Quit,
    ChangeContext(WindowContext),
    AddSongsToPlaylist(Vec<ListSong>),
    AddSongsToPlaylistAndPlay(Vec<ListSong>),
}

impl Youtui {
    pub async fn new(rt: RuntimeInfo) -> Result<Youtui> {
        let RuntimeInfo {
            api_key,
            debug,
            po_token,
            config,
        } = rt;
        // Setup tracing and link to tui_logger.
        // NOTE: File logging is always enabled for now - I can't think of a use case
        // where we wouldn't want this.
        init_tracing(debug, true).await?;
        match debug {
            true => info!("Starting in debug mode"),
            false => info!("Starting"),
        }
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture,)?;
        // Ensure clean return to shell if panic.
        IS_MAIN_THREAD.with(|flag| flag.set(true));
        std::panic::set_hook(Box::new(|panic_info| {
            if IS_MAIN_THREAD.with(|flag| flag.get()) {
                // If we fail to destruct terminal, ignore the error as panicking anyway.
                let _ = destruct_terminal();
                println!("{}", panic_info);
            }
        }));
        // Setup components
        let mut task_manager = async_callback_manager::AsyncCallbackManager::new()
            .with_on_task_spawn_callback(|task| {
                info!(
                    "Received task {:?}: type_id: {:?},  constraint: {:?}",
                    task.type_debug, task.type_id, task.constraint
                )
            });
        let server = Arc::new(server::Server::new(api_key, po_token));
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let (media_controls, media_control_event_stream) = MediaController::new()?;
        let event_handler = EventHandler::new(EVENT_CHANNEL_SIZE, media_control_event_stream)?;
        let (window_state, effect) = YoutuiWindow::new(config);
        // Even the creation of a YoutuiWindow causes an effect. We'll spawn it straight
        // away.
        task_manager.spawn_task(&server, effect);
        Ok(Youtui {
            status: AppStatus::Running,
            event_handler,
            window_state,
            task_manager,
            server,
            terminal,
            media_controls,
        })
    }
    pub async fn run(&mut self) -> Result<()> {
        loop {
            match &self.status {
                AppStatus::Running => {
                    // Write to terminal, using UI state as the input
                    // We draw after handling the event, as the event could be a keypress we want to
                    // instantly react to.
                    // Draw occurs before the first event, to ensure up loads immediately.
                    self.terminal.draw(|f| {
                        ui::draw::draw_app(f, &mut self.window_state);
                    })?;
                    self.media_controls.update_controls(
                        ui::draw_media_controls::draw_app_media_controls(&self.window_state),
                    )?;
                    // When running, the app is event based, and will block until one of the
                    // following 2 message types is received.
                    tokio::select! {
                        // Get the next event from the event_handler and process it.
                        // TODO: Consider checking here if redraw is required.
                        Some(event) = self.event_handler.next() =>
                            self.handle_event(event).await,
                        // Process the next manager event.
                        Some(outcome) = self.task_manager.get_next_response() =>
                            self.handle_effect(outcome),
                    }
                }
                AppStatus::Exiting(s) => {
                    // Once we're done running, destruct the terminal and print the exit message.
                    destruct_terminal()?;
                    println!("{s}");
                    break;
                }
            }
        }
        Ok(())
    }
    fn handle_effect(&mut self, effect: TaskOutcome<YoutuiWindow, ArcServer, TaskMetadata>) {
        match effect {
            async_callback_manager::TaskOutcome::StreamClosed => {
                info!("Received a stream closed message from task manager")
            }
            async_callback_manager::TaskOutcome::TaskPanicked {
                type_debug, error, ..
            } => {
                error!("Task {type_debug} panicked!");
                std::panic::resume_unwind(error.into_panic())
            }
            async_callback_manager::TaskOutcome::MutationReceived {
                mutation,
                type_id,
                type_debug,
                task_id,
                ..
            } => {
                info!(
                    "Received response to {:?}: type_id: {:?}, task_id: {:?}",
                    type_debug, type_id, task_id
                );
                let next_task = mutation(&mut self.window_state);
                self.task_manager.spawn_task(&self.server, next_task);
            }
        }
    }
    async fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Tick => self.window_state.handle_tick().await,
            AppEvent::Crossterm(e) => {
                let YoutuiEffect { effect, callback } =
                    self.window_state.handle_crossterm_event(e).await;
                self.task_manager.spawn_task(&self.server, effect);
                if let Some(callback) = callback {
                    self.handle_callback(callback);
                }
            }
            AppEvent::MediaControls(e) => {
                let YoutuiEffect { effect, callback } =
                    self.window_state.handle_media_event(e).await;
                self.task_manager.spawn_task(&self.server, effect);
                if let Some(callback) = callback {
                    self.handle_callback(callback);
                }
            }
            AppEvent::QuitSignal => self.status = AppStatus::Exiting("Quit signal received".into()),
        }
    }
    pub fn handle_callback(&mut self, callback: AppCallback) {
        match callback {
            AppCallback::Quit => self.status = AppStatus::Exiting("Quitting".into()),
            AppCallback::ChangeContext(context) => self.window_state.handle_change_context(context),
            AppCallback::AddSongsToPlaylist(song_list) => {
                self.window_state.handle_add_songs_to_playlist(song_list)
            }
            AppCallback::AddSongsToPlaylistAndPlay(song_list) => self.task_manager.spawn_task(
                &self.server,
                self.window_state
                    .handle_add_songs_to_playlist_and_play(song_list),
            ),
        }
    }
}

/// Cleanly exit the tui
fn destruct_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        crossterm::cursor::Show
    )?;
    Ok(())
}

/// Initialise tracing and subscribers such as tuilogger and file logging.
/// # Panics
/// If tracing fails to initialise, function will panic
async fn init_tracing(debug: bool, logging: bool) -> Result<()> {
    let tui_logger_layer = tui_logger::tracing_subscriber_layer();
    let (tracing_log_level, tui_logger_log_level) = if debug {
        (tracing::Level::DEBUG, tui_logger::LevelFilter::Debug)
    } else {
        (tracing::Level::INFO, tui_logger::LevelFilter::Info)
    };
    let context_layer =
        tracing_subscriber::filter::Targets::new().with_target("youtui", tracing_log_level);
    if logging {
        let (log_file, log_file_name) = get_limited_sequential_file(
            &get_data_dir()?,
            LOG_FILE_NAME,
            LOG_FILE_EXT,
            MAX_LOG_FILES,
        )
        .await?;
        let log_file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(
            log_file
                .try_into_std()
                .expect("No file operation should be in-flight yet"),
        ));
        tracing_subscriber::registry()
            .with(tui_logger_layer.and_then(log_file_layer))
            .with(context_layer)
            .init();
        info!("Logging to {:?}.", log_file_name);
    } else {
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing_log_level);
        tracing_subscriber::registry()
            .with(tui_logger_layer)
            .with(context_layer)
            .init();
    }
    tui_logger::init_logger(tui_logger_log_level)
        .expect("Expected logger to initialise succesfully");
    Ok(())
}
