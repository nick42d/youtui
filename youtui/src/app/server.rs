use tokio::sync::mpsc;
use tokio::sync::oneshot;
mod structures;
use crate::config::ApiKey;
use crate::Result;
use tracing::info;

use super::taskmanager::TaskID;

pub mod api;
pub mod downloader;
pub mod player;

const DL_CALLBACK_CHUNK_SIZE: u64 = 100000; // How often song download will pause to execute code.

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

pub struct Server {
    // Do I want to keep track of tasks here in a joinhandle?
    api: api::Api,
    player: player::PlayerManager,
    downloader: downloader::Downloader,
    _response_tx: mpsc::Sender<Response>,
    request_rx: mpsc::Receiver<Request>,
}

impl Server {
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
    pub async fn run(&mut self) -> Result<()> {
        // Could be a while let
        // Consider parallelism.
        while let Some(request) = self.request_rx.recv().await {
            match request {
                // TODO: Error handling for the queues.
                Request::Api(rx) => self.api.handle_request(rx).await?,
                Request::Downloader(rx) => self.downloader.handle_request(rx).await,
                Request::Player(rx) => self.player.handle_request(rx).await?,
            }
        }
        Ok(())
    }
}
// Consider using this instead of macro above.
async fn run_or_kill(
    future: impl futures::Future<Output = ()>,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::select! {
        _ = future => (),
        _ = kill_rx => info!("Task killed by caller"), // Is there a better way to do this?
    }
}

async fn spawn_run_or_kill(
    future: impl futures::Future<Output = ()> + Send + 'static,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::spawn(run_or_kill(future, kill_rx));
}
