use crossterm::event::{Event, EventStream, MouseEvent, MouseEventKind};
use futures::StreamExt;
use std::time::Duration;
use tokio::signal::unix::SignalKind;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::warn;

use crate::Result;

const TICK_RATE: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub enum AppEvent {
    Tick,
    Crossterm(Event),
    QuitSignal,
}

pub struct EventHandler {
    _tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    _ticker: EventSpawner<Ticker>,
    _signal_watcher: EventSpawner<SignalWatcher>,
    _crossterm_watcher: EventSpawner<CrosstermWatcher>,
}

struct Ticker;
struct SignalWatcher;
struct CrosstermWatcher;

struct EventSpawner<T> {
    _handler: JoinHandle<()>,
    _tx: Sender<AppEvent>,
    _spawner_type: T,
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
                    _ = tokio::signal::ctrl_c() => {}
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
    pub fn new(channel_size: usize) -> Result<Self> {
        let (tx, rx) = channel(channel_size);
        let _ticker = EventSpawner::new_ticker(&tx);
        let _signal_watcher = EventSpawner::new_signal_watcher(&tx)?;
        let _crossterm_watcher = EventSpawner::new_crossterm_watcher(&tx);
        Ok(Self {
            rx,
            _tx: tx,
            _ticker,
            _signal_watcher,
            _crossterm_watcher,
        })
    }
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
