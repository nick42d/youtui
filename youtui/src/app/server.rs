use std::sync::Arc;

use api::{ConcurrentApi, Request};
use futures::Future;
use messages::RequestEither;
use tokio::sync::{mpsc, oneshot};
mod structures;
use super::taskmanager::TaskID;
use crate::{config::ApiKey, Result};
use tracing::info;

pub mod api;
pub mod downloader;
mod messages;
pub mod player;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.
const MAX_RETRIES: usize = 5;
const AUDIO_QUALITY: rusty_ytdl::VideoQuality = rusty_ytdl::VideoQuality::HighestAudio;

#[derive(Debug)]
pub struct KillRequest;

#[derive(Debug)]
pub struct KillableTask {
    pub id: TaskID,
    pub kill_rx: oneshot::Receiver<KillRequest>,
}

impl KillableTask {
    pub fn new(id: TaskID, kill_rx: oneshot::Receiver<KillRequest>) -> Self {
        Self { id, kill_rx }
    }
}

trait TaskTrait {
    fn spawn(&self) {}
}

trait KillableTaskTrait {
    fn id(&self) -> TaskID;
    fn kill_rx(&self) -> oneshot::Receiver<KillRequest>;
    fn task(&self) -> impl futures::Future<Output = ()> + Send + 'static;
}

impl<T: KillableTaskTrait> Task for T {
    fn spawn(&self) {
        // spawn_run_or_kill(self.task(), self.kill_rx());
    }
}

fn spawn<T: Task>(task: T) {
    task.spawn()
}

pub enum TaskType {
    KillableTask,
    BlockableTask,
}

pub enum Request {
    Api(api::Request),
    Player(player::Request),
    Downloader(downloader::Request),
}
// Should this implement something like Killable/Blockable?
#[derive(Debug)]
pub enum Response {
    Api(api::Response),
    Player(player::Response),
    Downloader(downloader::Response),
}

pub struct Server<T>
where
    T: Future<Output = Arc<Result<ConcurrentApi>>>,
{
    // Do I want to keep track of tasks here in a joinhandle?
    api: api::Api<T>,
    player: player::PlayerManager,
    downloader: downloader::Downloader,
    _response_tx: mpsc::Sender<Response>,
    request_rx: mpsc::Receiver<Request>,
}

impl<T> Server<T>
where
    T: Future<Output = Arc<Result<ConcurrentApi>>>,
{
    pub fn new(
        api_key: ApiKey,
        response_tx: mpsc::Sender<Response>,
        request_rx: mpsc::Receiver<Request>,
    ) -> Result<Self> {
        let api = api::Api::new(api_key, response_tx.clone());
        // TODO: Error handling
        let player = player::PlayerManager::new(response_tx.clone())?;
        let downloader = downloader::Downloader::new(response_tx.clone());
        Ok(Self {
            api,
            player,
            downloader,
            request_rx,
            _response_tx: response_tx,
        })
    }
    pub async fn run(&mut self) {
        while let Some(request) = self.request_rx.recv().await {
            match request {
                // Handler functions should be short lived, as they will block the next request.
                Request::Api(rx) => self.api.handle_request(rx).await,
                Request::Downloader(rx) => self.downloader.handle_request(rx).await,
                Request::Player(rx) => self.player.handle_request(rx).await,
            }
        }
    }
}

fn spawn_run_or_kill(
    future: impl futures::Future<Output = ()> + Send + 'static,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::spawn(run_or_kill(future, kill_rx));
}

fn spawn_unkillable(future: impl futures::Future<Output = ()> + Send + 'static) {
    tokio::spawn(future);
}

async fn run_or_kill(
    future: impl futures::Future<Output = ()>,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::select! {
        _ = future => (),
        _ = kill_rx => info!("Task killed by caller"), // Is there a better way to do this?
    }
}
