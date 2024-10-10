use super::appevent::{AppEvent, EventHandler};
use super::Result;
use crate::{get_data_dir, RuntimeInfo};
use async_callback_manager::AsyncCallbackManager;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::widgets::{ListState, TableState};
use ratatui::{backend::CrosstermBackend, Terminal};
use server::downloader::InMemSong;
use server::Server;
use std::borrow::Cow;
use std::{io, sync::Arc};
use structures::{ListSong, ListSongID};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::prelude::*;
use ui::WindowContext;
use ui::YoutuiWindow;
use ytmapi_rs::common::{ArtistChannelID, VideoID};

mod component;
mod keycommand;
mod musiccache;
mod server;
mod structures;
#[cfg(FALSE)]
mod taskmanager;
mod ui;
mod view;

// We need this thread_local to ensure we know which is the main thread. Panic
// hook that destructs terminal should only run on the main thread.
thread_local! {
    static IS_MAIN_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const CALLBACK_CHANNEL_SIZE: usize = 64;
const ASYNC_CALLBACK_MANAGER_CHANNEL_SIZE: usize = 64;
const ASYNC_CALLBACK_SENDER_CHANNEL_SIZE: usize = 64;
const EVENT_CHANNEL_SIZE: usize = 256;
const LOG_FILE_NAME: &str = "debug.log";

pub struct Youtui {
    status: AppStatus,
    event_handler: EventHandler,
    window_state: YoutuiWindow,
    window_mutable_state: YoutuiMutableState,
    task_manager: AsyncCallbackManager<Server>,
    server: Server,
    callback_rx: mpsc::Receiver<AppCallback>,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

// Mutable state for scrollable widgets.
// This needs to be stored seperately so that we don't have concurrent mutable
// access.
#[derive(Default)]
pub struct YoutuiMutableState {
    pub filter_state: ListState,
    pub help_state: TableState,
    pub browser_album_songs_state: TableState,
    pub browser_artists_state: ListState,
    pub playlist_state: TableState,
}

#[derive(PartialEq)]
pub enum AppStatus {
    Running,
    // Cow: Message
    Exiting(Cow<'static, str>),
}

// A callback from one of the application components to the top level.
#[derive(Debug)]
pub enum AppCallback {
    DownloadSong(VideoID<'static>, ListSongID),
    Quit,
    ChangeContext(WindowContext),
    IncreaseVolume(i8),
    SearchArtist(String),
    GetSearchSuggestions(String),
    GetArtistSongs(ArtistChannelID<'static>),
    AddSongsToPlaylist(Vec<ListSong>),
    AddSongsToPlaylistAndPlay(Vec<ListSong>),
    PlaySong(Arc<InMemSong>, ListSongID),
    QueueSong(Arc<InMemSong>, ListSongID),
    AutoplaySong(Arc<InMemSong>, ListSongID),
    PausePlay(ListSongID),
    Stop(ListSongID),
    Seek(i8),
}

impl Youtui {
    pub fn new(rt: RuntimeInfo) -> Result<Youtui> {
        let RuntimeInfo {
            api_key,
            debug,
            po_token,
            ..
        } = rt;
        // Setup tracing and link to tui_logger.
        init_tracing(debug)?;
        info!("Starting");
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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
        let (callback_tx, callback_rx) = mpsc::channel(CALLBACK_CHANNEL_SIZE);
        let mut task_manager =
            async_callback_manager::AsyncCallbackManager::new(ASYNC_CALLBACK_MANAGER_CHANNEL_SIZE)
                .with_on_task_received_callback(|(type_id, sender_id, constraint)| {
                    info!(
                        "Received a task - type_id: {:?}, sender_id: {:?}, constraint: {:?}",
                        type_id, sender_id, constraint
                    )
                })
                .with_on_response_received_callback(|response| {
                    info!(
                        "Received a response - type_id: {:?}, sender_id: {:?}, task_id: {:?}",
                        response.type_id, response.sender_id, response.task_id
                    )
                });
        let server = server::Server::new(api_key, po_token);
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let event_handler = EventHandler::new(EVENT_CHANNEL_SIZE)?;
        let window_state = YoutuiWindow::new(callback_tx, &mut task_manager);
        Ok(Youtui {
            status: AppStatus::Running,
            event_handler,
            window_state,
            window_mutable_state: Default::default(),
            task_manager,
            server,
            callback_rx,
            terminal,
        })
    }
    pub async fn run(&mut self) -> Result<()> {
        let mut redraw = true;
        loop {
            match &self.status {
                AppStatus::Running => {
                    // Write to terminal, using UI state as the input
                    // We draw after handling the event, as the event could be a keypress we want to
                    // instantly react to.
                    // Draw occurs before the first event, to ensure up loads immediately.
                    if redraw {
                        self.terminal.draw(|f| {
                            ui::draw::draw_app(
                                f,
                                &self.window_state,
                                &mut self.window_mutable_state,
                            );
                        })?;
                    };
                    redraw = true;
                    // When running, the app is event based, and will block until one of the
                    // following 4 message types is received.
                    tokio::select! {
                        // Get the next event from the event_handler and process it.
                        Some(event) = self.event_handler.next() => self.handle_event(event).await,
                        // Process any top-level callbacks in the queue.
                        Some(callback) = self.callback_rx.recv() => self.handle_callback(callback).await,
                        // Process the next manager event.
                        // If all the manager has done is spawn tasks, there's no need to draw.
                        Some(manager_event) = self.task_manager.manage_next_event(&self.server) => if manager_event.is_spawned_task() {
                            redraw = false;
                        },
                        _ = self.window_state.async_update() => (),
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
    async fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Tick => self.window_state.handle_tick().await,
            AppEvent::Crossterm(e) => self.window_state.handle_event(e).await,
            AppEvent::QuitSignal => self.status = AppStatus::Exiting("Quit signal received".into()),
        }
    }
    pub async fn handle_callback(&mut self, callback: AppCallback) {
        match callback {
            AppCallback::Quit => self.status = AppStatus::Exiting("Quitting".into()),
            AppCallback::ChangeContext(context) => self.window_state.handle_change_context(context),
            AppCallback::AddSongsToPlaylist(song_list) => {
                self.window_state.handle_add_songs_to_playlist(song_list);
            }
            AppCallback::AddSongsToPlaylistAndPlay(song_list) => {
                self.window_state
                    .handle_add_songs_to_playlist_and_play(song_list)
                    .await
            }
            _ => todo!(),
        }
    }
}

/// Cleanly exit the tui
fn destruct_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(io::stdout(), crossterm::cursor::Show)?;
    Ok(())
}

/// Initialise tracing and subscribers such as tuilogger and file logging.
/// # Panics
/// If tracing fails to initialise, function will panic
fn init_tracing(debug: bool) -> Result<()> {
    // NOTE: It seems that tui-logger only displays events at info or higher,
    // possibly a limitation with the implementation.
    // https://github.com/gin66/tui-logger/issues/66
    // TODO: PR upstream
    let tui_logger_layer = tui_logger::tracing_subscriber_layer();
    if debug {
        let log_file_name = get_data_dir()?.join(LOG_FILE_NAME);
        let log_file = std::fs::File::create(&log_file_name)?;
        let log_file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(log_file));
        // TODO: Confirm if this filter is correct.
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing::Level::DEBUG);
        tracing_subscriber::registry()
            .with(tui_logger_layer.and_then(log_file_layer))
            .with(context_layer)
            .init();
        info!("Started in debug mode, logging to {:?}.", log_file_name);
    } else {
        // TODO: Confirm if this filter is correct.
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing::Level::TRACE);
        tracing_subscriber::registry()
            .with(tui_logger_layer)
            .with(context_layer)
            .init();
    }
    Ok(())
}
