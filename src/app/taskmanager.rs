use std::sync::Arc;

use crate::app::server::KillRequest;
use crate::app::server::{self, KillableTask};
use crate::core::send_or_error;
use crate::Result;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::{error, warn};
use ytmapi_rs::{ChannelID, VideoID};

use super::server::{api, downloader, player};
use super::structures::{ListSongID, Percentage};
use super::ui::{StateUpdateMessage, UIMessage};

const MESSAGE_QUEUE_LENGTH: usize = 256;

pub struct TaskManager {
    cur_id: TaskID,
    tasks: Vec<Task>,
    _server_handle: tokio::task::JoinHandle<()>,
    server_request_tx: mpsc::Sender<server::Request>,
    server_response_rx: mpsc::Receiver<server::Response>,
    request_tx: mpsc::Sender<AppRequest>,
    request_rx: mpsc::Receiver<AppRequest>,
}

#[derive(PartialEq, Default, Debug, Copy, Clone)]
pub struct TaskID(usize);

struct Task {
    id: TaskID,
    // XXX: to check if valid, is it as simple as check if Option is taken?
    kill: Option<oneshot::Sender<KillRequest>>,
    message: AppRequest,
}

#[derive(Clone)]
pub enum AppRequest {
    SearchArtists(String),
    GetSearchSuggestions(String),
    GetArtistSongs(ChannelID<'static>),
    Download(VideoID<'static>, ListSongID),
    IncreaseVolume(i8),
    GetVolume,
    PlaySong(Arc<Vec<u8>>, ListSongID),
    GetProgress(ListSongID),
    Stop,
    PausePlay, // XXX: Add ID?
}

impl AppRequest {
    fn category(&self) -> RequestCategory {
        match self {
            AppRequest::SearchArtists(_) => RequestCategory::Search,
            AppRequest::GetSearchSuggestions(_) => RequestCategory::GetSearchSuggestions,
            AppRequest::GetArtistSongs(_) => RequestCategory::Get,
            AppRequest::Download(..) => RequestCategory::Download,
            AppRequest::IncreaseVolume(_) => RequestCategory::Unkillable,
            AppRequest::GetVolume => RequestCategory::GetVolume,
            AppRequest::PlaySong(..) => RequestCategory::Unkillable,
            AppRequest::GetProgress(_) => RequestCategory::ProgressUpdate,
            AppRequest::Stop => RequestCategory::Unkillable,
            AppRequest::PausePlay => RequestCategory::Unkillable,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum RequestCategory {
    Search,
    Get,
    Download,
    GetSearchSuggestions,
    GetVolume,
    ProgressUpdate,
    Unkillable,
}

impl TaskManager {
    // This should handle messages as well.
    // TODO: Error handling
    pub fn new() -> Self {
        let (server_request_tx, server_request_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let (server_response_tx, server_response_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let (request_tx, request_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let _server_handle = tokio::spawn(async {
            let mut a = server::Server::new(server_response_tx, server_request_rx);
            a.run().await;
        });
        Self {
            cur_id: TaskID::default(),
            tasks: Vec::new(),
            _server_handle,
            server_request_tx,
            server_response_rx,
            request_tx,
            request_rx,
        }
    }
    pub fn get_sender_clone(&self) -> mpsc::Sender<AppRequest> {
        self.request_tx.clone()
    }
    pub async fn process_requests(&mut self) {
        while let Ok(msg) = self.request_rx.try_recv() {
            self.send_request(msg).await;
        }
    }
    async fn send_request(&mut self, request: AppRequest) -> Result<()> {
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        // NOTE: We allocate as we want to keep a copy of the same message that was sent.
        let id = self.add_task(kill_tx, request.clone());
        match request {
            AppRequest::SearchArtists(a) => self.spawn_search_artists(a, id, kill_rx).await,
            AppRequest::GetSearchSuggestions(q) => {
                self.spawn_get_search_suggestions(q, id, kill_rx).await
            }
            AppRequest::GetArtistSongs(a_id) => {
                self.spawn_get_artist_songs(a_id, id, kill_rx).await
            }
            AppRequest::Download(v_id, s_id) => self.spawn_download(v_id, s_id, id, kill_rx).await,
            AppRequest::IncreaseVolume(i) => self.spawn_increase_volume(i, id).await,
            AppRequest::GetVolume => self.spawn_get_volume(id, kill_rx).await,
            AppRequest::PlaySong(_, _) => todo!(),
            AppRequest::GetProgress(_) => todo!(),
            AppRequest::Stop => todo!(),
            AppRequest::PausePlay => todo!(),
        };
        Ok(())
    }
    // TODO: Consider if this should create it's own channel and return a KillableTask.
    fn add_task(
        &mut self,
        kill: tokio::sync::oneshot::Sender<KillRequest>,
        message: AppRequest,
    ) -> TaskID {
        self.cur_id.0 += 1;
        self.tasks.push(Task {
            id: self.cur_id,
            kill: Some(kill),
            message,
        });
        self.cur_id
    }
    pub async fn spawn_search_artists(
        &mut self,
        artist: String,
        id: TaskID,
        kill_rx: oneshot::Receiver<KillRequest>,
    ) {
        // Supersedes previous tasks of same type.
        // TODO: Use this as a pattern.
        self.kill_all_task_type(RequestCategory::Search);
        send_or_error(
            &self.server_request_tx,
            server::Request::Api(server::api::Request::NewArtistSearch(
                artist,
                KillableTask::new(id, kill_rx),
            )),
        )
        .await
    }
    pub async fn spawn_get_search_suggestions(
        &mut self,
        query: String,
        id: TaskID,
        kill_rx: oneshot::Receiver<KillRequest>,
    ) {
        self.kill_all_task_type(RequestCategory::GetSearchSuggestions);
        send_or_error(
            &self.server_request_tx,
            server::Request::Api(server::api::Request::GetSearchSuggestions(
                query,
                KillableTask::new(id, kill_rx),
            )),
        )
        .await
    }
    pub async fn spawn_get_artist_songs(
        &mut self,
        artist_id: ChannelID<'static>,
        id: TaskID,
        kill_rx: oneshot::Receiver<KillRequest>,
    ) {
        self.kill_all_task_type(RequestCategory::Get);
        send_or_error(
            &self.server_request_tx,
            server::Request::Api(server::api::Request::SearchSelectedArtist(
                artist_id,
                KillableTask::new(id, kill_rx),
            )),
        )
        .await
    }
    pub async fn spawn_download(
        &mut self,
        video_id: VideoID<'static>,
        list_song_id: ListSongID,
        id: TaskID,
        kill_rx: oneshot::Receiver<KillRequest>,
    ) {
        send_or_error(
            // Does not kill previous tasks!
            &self.server_request_tx,
            server::Request::Downloader(server::downloader::Request::DownloadSong(
                video_id,
                list_song_id,
                KillableTask::new(id, kill_rx),
            )),
        )
        .await
    }
    pub async fn spawn_increase_volume(&mut self, vol_inc: i8, id: TaskID) {
        // Does not kill previous tasks - these are additive requests.
        // Does this make this than an unkillable task?
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::IncreaseVolume(vol_inc, id)),
        )
        .await
    }
    pub async fn spawn_get_volume(&mut self, id: TaskID, kill_rx: oneshot::Receiver<KillRequest>) {
        self.kill_all_task_type(RequestCategory::GetVolume);
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::GetVolume(KillableTask::new(
                id, kill_rx,
            ))),
        )
        .await
    }
    pub fn is_task_valid(&self, id: TaskID) -> bool {
        self.tasks.iter().any(|x| x.id == id)
    }
    pub fn kill_all_task_type(&mut self, request_category: RequestCategory) {
        tracing::debug!(
            "Received message to kill all pending {:?} tasks",
            request_category
        );
        for task in self
            .tasks
            .iter_mut()
            .filter(|x| x.message.category() == request_category)
        {
            if let Some(tx) = task.kill.take() {
                // TODO: Handle error
                tx.send(KillRequest)
                    .unwrap_or_else(|_e| error!("Error sending kill message"));
            }
        }
        self.tasks
            .retain(|x| x.message.category() != request_category);
    }
    pub fn process_messages(&mut self) -> Vec<StateUpdateMessage> {
        let mut state_update_list = Vec::new();
        while let Ok(msg) = self.server_response_rx.try_recv() {
            match msg {
                server::Response::Api(msg) => state_update_list.push(self.process_api_msg(msg)),
                server::Response::Player(msg) => {
                    state_update_list.push(self.process_player_msg(msg))
                }
                server::Response::Downloader(msg) => {
                    state_update_list.push(self.process_downloader_msg(msg))
                }
            }
        }
        state_update_list
    }
    pub fn process_api_msg(&self, msg: api::Response) -> StateUpdateMessage {
        match msg {
            api::Response::ReplaceArtistList(_, _) => todo!(),
            api::Response::SearchArtistError(_) => todo!(),
            api::Response::ReplaceSearchSuggestions(_, _, _) => todo!(),
            api::Response::SongListLoading(_) => todo!(),
            api::Response::SongListLoaded(_) => todo!(),
            api::Response::NoSongsFound(_) => todo!(),
            api::Response::SongsFound(_) => todo!(),
            api::Response::AppendSongList(_, _, _, _, _) => todo!(),
        }
    }
    pub fn process_downloader_msg(&self, msg: downloader::Response) -> StateUpdateMessage {
        match msg {
            downloader::Response::SongProgressUpdate(update_type, song_id, task_id) => {
                StateUpdateMessage::SetSongProgress(update_type, song_id)
            }
        };
        todo!()
    }
    pub fn process_player_msg(&self, msg: player::Response) -> StateUpdateMessage {
        match msg {
            player::Response::DonePlaying(_) => todo!(),
            player::Response::Paused(_) => todo!(),
            player::Response::Playing(_) => todo!(),
            player::Response::Stopped => todo!(),
            player::Response::ProgressUpdate(_, _) => todo!(),
            player::Response::VolumeUpdate(vol, id) => {
                // TODO: check task is valid
                warn!("Race condition check for volume update not yet implemented");
                StateUpdateMessage::SetVolume(Percentage(vol))
            }
        }
    }
}
