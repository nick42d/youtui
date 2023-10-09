use crate::app::server;
use crate::app::server::KillRequest;
use anyhow::Result;
use tracing::error;
use ytmapi_rs::{ChannelID, VideoID};

use super::structures::ListSongID;

const MESSAGE_QUEUE_LENGTH: usize = 256;

pub struct TaskRegister {
    cur_id: TaskID,
    tasks: Vec<Task>,
    _server_handle: tokio::task::JoinHandle<()>,
    request_tx: tokio::sync::mpsc::Sender<server::Request>,
    response_rx: tokio::sync::mpsc::Receiver<server::Response>,
}

#[derive(PartialEq, Default, Debug, Copy, Clone)]
pub struct TaskID(usize);

struct Task {
    id: TaskID,
    // then to check if valid, is it as simple as check if Option is taken?
    kill: Option<tokio::sync::oneshot::Sender<KillRequest>>,
    context: TaskContext,
}

#[derive(Clone)]
pub struct TaskContext {
    // Consider the caller as part of this context.
    pub request: AppRequest,
}

impl TaskContext {
    fn category(&self) -> RequestCategory {
        self.request.category()
    }
}

#[derive(Clone)]
pub enum AppRequest {
    SearchArtists(String),
    GetArtistSongs(ChannelID<'static>),
    Download(VideoID<'static>, ListSongID),
}

impl AppRequest {
    fn category(&self) -> RequestCategory {
        match self {
            Self::SearchArtists(_) => RequestCategory::Search,
            Self::GetArtistSongs(_) => RequestCategory::Get,
            Self::Download(..) => RequestCategory::Download,
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum RequestCategory {
    Search,
    Get,
    Download,
}

impl TaskRegister {
    // TODO: Error handling
    pub fn new() -> Self {
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let _server_handle = tokio::spawn(async {
            let mut a = server::Server::new(response_tx, request_rx);
            a.run().await;
        });
        Self {
            cur_id: TaskID::default(),
            tasks: Vec::new(),
            _server_handle,
            request_tx,
            response_rx,
        }
    }
    pub async fn send_request(&mut self, request: AppRequest) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let context = TaskContext { request };
        // TODO: Try remove allocation.
        let id = self.add_task(tx, context.clone());
        match context {
            TaskContext {
                request: AppRequest::SearchArtists(s),
                ..
            } => {
                self.request_tx
                    .send(server::Request::NewArtistSearch(s, id, rx))
                    .await?
            }
            TaskContext {
                request: AppRequest::GetArtistSongs(s),
                ..
            } => {
                self.request_tx
                    .send(server::Request::SearchSelectedArtist(s, id, rx))
                    .await?
            }
            TaskContext {
                request: AppRequest::Download(video_id, playlist_id),
                ..
            } => {
                self.request_tx
                    .send(server::Request::DownloadSong(video_id, playlist_id, id, rx))
                    .await?
            }
        };
        Ok(())
    }
    fn add_task(
        &mut self,
        kill: tokio::sync::oneshot::Sender<KillRequest>,
        context: TaskContext,
    ) -> TaskID {
        self.cur_id.0 += 1;
        self.tasks.push(Task {
            id: self.cur_id,
            kill: Some(kill),
            context,
        });
        self.cur_id
    }
    pub fn is_task_valid(&self, id: TaskID) -> bool {
        self.tasks.iter().any(|x| x.id == id)
    }
    pub fn kill_all_task_type(&mut self, request_category: RequestCategory) {
        for task in self
            .tasks
            .iter_mut()
            .filter(|x| x.context.category() == request_category)
        {
            if let Some(tx) = task.kill.take() {
                // TODO: Handle error
                tx.send(KillRequest)
                    .unwrap_or_else(|_e| error!("Error sending kill message"));
            }
        }
        self.tasks
            .retain(|x| x.context.category() != request_category);
    }
    pub fn try_recv(&mut self) -> Result<server::Response, tokio::sync::mpsc::error::TryRecvError> {
        self.response_rx.try_recv()
    }
}
