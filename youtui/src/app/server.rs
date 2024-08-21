use super::taskmanager::{KillableTask, TaskID};
use crate::{config::ApiKey, Result};
use api::ConcurrentApi;
use futures::{future::Shared, Future};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

pub use messages::*;

pub mod api;
pub mod downloader;
pub mod messages;
pub mod player;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.
const MAX_RETRIES: usize = 5;
const AUDIO_QUALITY: rusty_ytdl::VideoQuality = rusty_ytdl::VideoQuality::HighestAudio;

/// A component of the server can handle requests.
/// Note, the handler functions should not significantly block - these run
/// sequentially, and so a blocked handler will prevent the next handler from
/// running. Instead a handler should spawn any significant amounts of work.
trait ServerComponent {
    type KillableRequestType;
    type UnkillableRequestType;
    async fn handle_killable_request(
        &self,
        request: Self::KillableRequestType,
        task: KillableTask,
    ) -> Result<()>;
    async fn handle_unkillable_request(
        &self,
        request: Self::UnkillableRequestType,
        task: TaskID,
    ) -> Result<()>;
}

/// Application backend that is capable of spawning concurrent tasks in response
/// to requests. Tasks each receive a handle to respond back to the caller.
/// Generic across 'T' - 'T' is a future but we need to use generics to allow
/// use of concrete type.
pub struct Server<T> {
    api: api::Api<T>,
    player: player::Player,
    downloader: downloader::Downloader,
    request_rx: mpsc::Receiver<ServerRequest>,
}

impl Server<()> {
    pub fn new(
        api_key: ApiKey,
        response_tx: mpsc::Sender<ServerResponse>,
        request_rx: mpsc::Receiver<ServerRequest>,
    ) -> Server<Shared<impl Future<Output = Arc<Result<ConcurrentApi>>>>> {
        let api = api::Api::new(api_key, response_tx.clone());
        let player = player::Player::new(response_tx.clone());
        let downloader = downloader::Downloader::new(response_tx.clone());
        Server {
            api,
            player,
            downloader,
            request_rx,
        }
    }
}

impl<T> Server<Shared<T>>
where
    T: Future<Output = Arc<Result<ConcurrentApi>>>,
{
    pub async fn run(&mut self) {
        while let Some(request) = self.request_rx.recv().await {
            let outcome = match request {
                ServerRequest::Killable {
                    killable_task,
                    request,
                } => self.handle_killable_request(request, killable_task).await,
                ServerRequest::Unkillable { task_id, request } => {
                    self.handle_unkillable_request(request, task_id).await
                }
            };
            if let Err(e) = outcome {
                error!("Error handling request: {:?}", e)
            }
        }
    }
}

impl<T> ServerComponent for Server<Shared<T>>
where
    T: Future<Output = Arc<Result<ConcurrentApi>>>,
{
    type KillableRequestType = KillableServerRequest;
    type UnkillableRequestType = UnkillableServerRequest;
    async fn handle_killable_request(
        &self,
        request: Self::KillableRequestType,
        task: KillableTask,
    ) -> Result<()> {
        match request {
            KillableServerRequest::Api(r) => self.api.handle_killable_request(r, task).await,
            KillableServerRequest::Player(r) => self.player.handle_killable_request(r, task).await,
            KillableServerRequest::Downloader(r) => {
                self.downloader.handle_killable_request(r, task).await
            }
        }
    }
    async fn handle_unkillable_request(
        &self,
        request: Self::UnkillableRequestType,
        task: TaskID,
    ) -> Result<()> {
        match request {
            UnkillableServerRequest::Api(r) => self.api.handle_unkillable_request(r, task).await,
            UnkillableServerRequest::Player(r) => {
                self.player.handle_unkillable_request(r, task).await
            }
            UnkillableServerRequest::Downloader(r) => {
                self.downloader.handle_unkillable_request(r, task).await
            }
        }
    }
}

fn spawn_unkillable(future: impl futures::Future<Output = ()> + Send + 'static) {
    tokio::spawn(future);
}

fn spawn_run_or_kill(
    future: impl futures::Future<Output = ()> + Send + 'static,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::spawn(run_or_kill(future, kill_rx));
}

async fn run_or_kill(
    future: impl futures::Future<Output = ()>,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::select! {
        _ = future => (),
        _ = kill_rx => info!("Task killed by caller"),
    }
}
