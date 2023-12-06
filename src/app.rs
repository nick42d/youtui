use self::structures::{ListSong, ListSongID};
use self::taskmanager::{AppRequest, TaskManager};
use self::ui::WindowContext;
use super::appevent::{AppEvent, EventHandler};
use super::Result;
use crate::error::Error;
use crate::RuntimeInfo;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::widgets::{ListState, TableState};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::borrow::Cow;
use std::{io, sync::Arc};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::prelude::*;
use ui::YoutuiWindow;
use ytmapi_rs::{ChannelID, VideoID};

mod component;
mod keycommand;
mod musiccache;
mod server;
mod structures;
mod taskmanager;
mod ui;
mod view;

const CALLBACK_CHANNEL_SIZE: usize = 64;
const EVENT_CHANNEL_SIZE: usize = 256;
const _LOG_FILE_NAME: &str = "debug.log";

pub struct Youtui {
    status: AppStatus,
    event_handler: EventHandler,
    window_state: YoutuiWindow,
    window_mutable_state: YoutuiMutableState,
    task_manager: TaskManager,
    callback_rx: mpsc::Receiver<AppCallback>,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

// Mutable state for scrollable widgets.
// This needs to be stored seperately so that we don't have concurrent mutable access.
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
    GetVolume,
    GetProgress(ListSongID),
    Quit,
    ChangeContext(WindowContext),
    // Perhaps shiould not be here.
    HandleApiError(Error),
    IncreaseVolume(i8),
    SearchArtist(String),
    GetSearchSuggestions(String),
    GetArtistSongs(ChannelID<'static>),
    AddSongsToPlaylist(Vec<ListSong>),
    AddSongsToPlaylistAndPlay(Vec<ListSong>),
    PlaySong(Arc<Vec<u8>>, ListSongID),
    PausePlay(ListSongID),
    Stop(ListSongID),
}

impl Youtui {
    pub fn new(rt: RuntimeInfo) -> Result<Youtui> {
        let RuntimeInfo { api_key, .. } = rt;
        // TODO: Handle errors
        // Setup tracing and link to tui_logger.
        let tui_logger_layer = tui_logger::tracing_subscriber_layer();
        // Hold off implementing log file until dirs improved.
        // let log_file = std::fs::File::create(get_data_dir()?.join(LOG_FILE_NAME))?;
        // let log_file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(log_file));
        // TODO: Confirm if this filter is correct.
        let context_layer =
            tracing_subscriber::filter::Targets::new().with_target("youtui", tracing::Level::DEBUG);
        tracing_subscriber::registry()
            .with(
                tui_logger_layer, // Hold off from implementing log file until dirs support improved.
                                  // .and_then(log_file_layer)
            )
            .with(context_layer)
            .init();
        info!("Starting");
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        // Ensure clean return to shell if panic.
        std::panic::set_hook(Box::new(|panic_info| {
            // If we fail to destruct terminal, ignore the error as panicking anyway.
            let _ = destruct_terminal();
            println!("{}", panic_info);
        }));
        // Setup components
        let (callback_tx, callback_rx) = mpsc::channel(CALLBACK_CHANNEL_SIZE);
        let task_manager = taskmanager::TaskManager::new(api_key);
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let event_handler = EventHandler::new(EVENT_CHANNEL_SIZE)?;
        let window_state = YoutuiWindow::new(callback_tx);
        Ok(Youtui {
            status: AppStatus::Running,
            terminal,
            event_handler,
            window_state,
            window_mutable_state: Default::default(),
            task_manager,
            callback_rx,
        })
    }
    pub async fn run(&mut self) -> Result<()> {
        loop {
            match &self.status {
                AppStatus::Running => {
                    // Get the next event from the event_handler and process it.
                    self.handle_next_event().await;
                    // Process any callbacks in the queue.
                    self.process_callbacks().await;
                    // Get the state update events from the task manager and apply them to the window state.
                    self.synchronize_state().await;
                    // Write to terminal, using UI state as the input
                    // We draw after handling the event, as the event could be a keypress we want to instantly react to.
                    self.terminal.draw(|f| {
                        ui::draw::draw_app(f, &self.window_state, &mut self.window_mutable_state);
                    })?;
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
    async fn synchronize_state(&mut self) {
        self.task_manager
            .action_messages(&mut self.window_state)
            .await;
    }
    async fn handle_next_event(&mut self) {
        let msg = self.event_handler.next().await;
        // TODO: Handle closed channel better
        match msg {
            Some(AppEvent::QuitSignal) => {
                self.status = AppStatus::Exiting("Quit signal received".into())
            }
            Some(AppEvent::Crossterm(e)) => self.window_state.handle_event(e).await,
            // XXX: Should be try_poll or similar? Poll the Future but don't await it?
            Some(AppEvent::Tick) => self.window_state.handle_tick().await,
            None => panic!("Channel closed"),
        }
    }
    pub async fn process_callbacks(&mut self) {
        while let Ok(msg) = self.callback_rx.try_recv() {
            match msg {
                AppCallback::DownloadSong(video_id, playlist_id) => {
                    self.task_manager
                        .send_request(AppRequest::Download(video_id, playlist_id))
                        .await;
                }
                AppCallback::Quit => self.status = AppStatus::Exiting("Quitting".into()),
                AppCallback::HandleApiError(e) => {
                    self.status = AppStatus::Exiting(format!("{e}").into())
                }

                AppCallback::ChangeContext(context) => {
                    self.window_state.handle_change_context(context)
                }
                AppCallback::IncreaseVolume(i) => {
                    self.task_manager
                        .send_request(AppRequest::IncreaseVolume(i))
                        .await;
                }
                AppCallback::GetSearchSuggestions(text) => {
                    self.task_manager
                        .send_request(AppRequest::GetSearchSuggestions(text))
                        .await;
                }
                AppCallback::SearchArtist(artist) => {
                    self.task_manager
                        .send_request(AppRequest::SearchArtists(artist))
                        .await;
                }
                AppCallback::GetArtistSongs(id) => {
                    self.task_manager
                        .send_request(AppRequest::GetArtistSongs(id))
                        .await;
                }
                AppCallback::AddSongsToPlaylist(song_list) => {
                    self.window_state.handle_add_songs_to_playlist(song_list);
                }
                AppCallback::AddSongsToPlaylistAndPlay(song_list) => {
                    self.window_state
                        .handle_add_songs_to_playlist_and_play(song_list)
                        .await
                }
                AppCallback::PlaySong(song, id) => {
                    self.task_manager
                        .send_request(AppRequest::PlaySong(song, id))
                        .await;
                }

                AppCallback::PausePlay(id) => {
                    self.task_manager
                        .send_request(AppRequest::PausePlay(id))
                        .await;
                }
                AppCallback::Stop(id) => {
                    self.task_manager.send_request(AppRequest::Stop(id)).await;
                }
                AppCallback::GetVolume => {
                    self.task_manager.send_request(AppRequest::GetVolume).await;
                }
                AppCallback::GetProgress(id) => {
                    self.task_manager
                        .send_request(AppRequest::GetPlayProgress(id))
                        .await;
                }
            }
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
