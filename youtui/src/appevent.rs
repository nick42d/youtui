/// NOTE: WASM currently not supported.
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use futures::{Stream, StreamExt};
use souvlaki::{MediaControlEvent, MediaControls};
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, warn};

#[cfg(target_family = "unix")]
use tokio::signal::unix::SignalKind;

const TICK_RATE: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub enum AppEvent {
    Tick,
    Crossterm(Event),
    MediaControls(MediaControlEvent),
    QuitSignal,
}

pub struct EventHandler {
    _tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    _ticker: EventSpawner<Ticker>,
    _signal_watcher: EventSpawner<SignalWatcher>,
    _crossterm_watcher: EventSpawner<CrosstermWatcher>,
    _media_controls_watcher: EventSpawner<MediaControlsWatcher>,
}

struct Ticker;
struct SignalWatcher;
struct CrosstermWatcher;
struct MediaControlsWatcher;

struct EventSpawner<T> {
    _handler: JoinHandle<()>,
    _tx: Sender<AppEvent>,
    _spawner_type: T,
}

impl EventSpawner<MediaControlsWatcher> {
    fn new_media_controls_watcher(
        tx: &Sender<AppEvent>,
        mut media_events: impl Stream<Item = MediaControlEvent> + Unpin + Send + 'static,
    ) -> EventSpawner<MediaControlsWatcher> {
        let handler_tx = tx.clone();
        let _tx = tx.clone();
        let _spawner_type = MediaControlsWatcher;
        // Required due to definition of EventSpawner.
        let _handler = tokio::spawn(async move {
            loop {
                let e = media_events.next().await.unwrap();
                handler_tx
                    .send(AppEvent::MediaControls(e))
                    .await
                    .unwrap_or_else(|e| warn!("Error {:?} receieved when sending tick event", e));
            }
        });
        Self {
            _tx,
            _handler,
            _spawner_type,
        }
    }
}

impl EventSpawner<Ticker> {
    fn new_ticker(tx: &Sender<AppEvent>) -> EventSpawner<Ticker> {
        let handler_tx = tx.clone();
        let _tx = tx.clone();
        let mut interval = interval(TICK_RATE);
        let _spawner_type = Ticker;
        let _handler = tokio::spawn(async move {
            loop {
                interval.tick().await;
                handler_tx
                    .send(AppEvent::Tick)
                    .await
                    .unwrap_or_else(|e| warn!("Error {:?} receieved when sending tick event", e));
            }
        });
        Self {
            _tx,
            _handler,
            _spawner_type,
        }
    }
}

#[cfg(unix)]
impl EventSpawner<SignalWatcher> {
    fn new_signal_watcher(tx: &Sender<AppEvent>) -> Result<EventSpawner<SignalWatcher>> {
        let handler_tx = tx.clone();
        let _tx = tx.clone();
        let _spawner_type = SignalWatcher;

        let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
        let mut sigquit = tokio::signal::unix::signal(SignalKind::quit())?;
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

        let _handler = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sigint.recv() => {}
                    _ = sigquit.recv() => {}
                    _ = sigterm.recv() => {}
                }
                handler_tx
                    .send(AppEvent::QuitSignal)
                    .await
                    .unwrap_or_else(|e| warn!("Error {:?} receieved when sending signal event", e));
            }
        });
        Ok(Self {
            _tx,
            _handler,
            _spawner_type,
        })
    }
}

#[cfg(windows)]
impl EventSpawner<SignalWatcher> {
    fn new_signal_watcher(tx: &Sender<AppEvent>) -> Result<EventSpawner<SignalWatcher>> {
        let handler_tx = tx.clone();
        let _tx = tx.clone();
        let _spawner_type = SignalWatcher;

        let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
        let mut ctrl_close = tokio::signal::windows::ctrl_close()?;
        let mut ctrl_logoff = tokio::signal::windows::ctrl_logoff()?;
        let mut ctrl_shutdown = tokio::signal::windows::ctrl_shutdown()?;

        let _handler = tokio::spawn(async move {
            loop {
                tokio::select! {
                   _ = ctrl_break.recv() => {}
                   _ = ctrl_close.recv() => {}
                   _ = ctrl_logoff.recv() => {}
                   _ = ctrl_shutdown.recv() => {}
                }
                handler_tx
                    .send(AppEvent::QuitSignal)
                    .await
                    .unwrap_or_else(|e| warn!("Error {:?} receieved when sending signal event", e));
            }
        });
        Ok(Self {
            _tx,
            _handler,
            _spawner_type,
        })
    }
}

impl EventSpawner<CrosstermWatcher> {
    fn new_crossterm_watcher(tx: &Sender<AppEvent>) -> EventSpawner<CrosstermWatcher> {
        let handler_tx = tx.clone();
        let _tx = tx.clone();
        let _spawner_type = CrosstermWatcher;
        let mut events = EventStream::new();
        let _handler = tokio::spawn(async move {
            while let Some(Ok(event)) = events.next().await {
                match event {
                    // Don't send mouse move or drag events back to application -
                    // Each application event causes a UI render.
                    Event::Mouse(MouseEvent {
                        kind: MouseEventKind::Drag(_) | MouseEventKind::Moved,
                        ..
                    }) => (),
                    // Avoid duplicate keypresses on Windows.
                    // https://ratatui.rs/faq/duplicate-key-events-windows.html
                    // NOTE: There is also a Repeat KeyEventKind which we aren't ignoring for now.
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Release,
                        ..
                    }) => (),
                    _ => handler_tx
                        .send(AppEvent::Crossterm(event))
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Error {:?} receieved when sending Crossterm event", e)
                        }),
                }
            }
        });
        Self {
            _tx,
            _handler,
            _spawner_type,
        }
    }
}
impl EventHandler {
    pub fn new(
        channel_size: usize,
        media_events: impl Stream<Item = MediaControlEvent> + Send + Unpin + 'static,
    ) -> Result<Self> {
        let (tx, rx) = channel(channel_size);
        let _ticker = EventSpawner::new_ticker(&tx);
        let _signal_watcher = EventSpawner::new_signal_watcher(&tx)?;
        let _crossterm_watcher = EventSpawner::new_crossterm_watcher(&tx);
        let _media_controls_watcher = EventSpawner::new_media_controls_watcher(&tx, media_events);
        Ok(Self {
            rx,
            _tx: tx,
            _ticker,
            _signal_watcher,
            _crossterm_watcher,
            _media_controls_watcher,
        })
    }
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
