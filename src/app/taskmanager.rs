use super::server::{api, downloader, player};
use super::statemanager::StateUpdateMessage;
use super::structures::ListSongID;
use crate::app::server::KillRequest;
use crate::app::server::{self, KillableTask};
use crate::config::ApiKey;
use crate::core::send_or_error;
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};
use ytmapi_rs::{ChannelID, VideoID};

const MESSAGE_QUEUE_LENGTH: usize = 256;

pub struct TaskManager {
    cur_id: TaskID,
    tasks: Vec<Task>,
    _server_handle: tokio::task::JoinHandle<Result<()>>,
    server_request_tx: mpsc::Sender<server::Request>,
    server_response_rx: mpsc::Receiver<server::Response>,
    request_tx: mpsc::Sender<AppRequest>,
    request_rx: mpsc::Receiver<AppRequest>,
}

enum TaskType {
    Killable(KillableTask), // A task that can be called by the caller. Once killed, the caller will stop receiving messages to prevent race conditions.
    Blockable(TaskID), // A task that the caller can block from receiving further messages, but cannot be killed.
    Completable(TaskID), // A task that cannot be killed or blocked. Will always run until completion.
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
    GetPlayProgress(ListSongID),
    Stop(ListSongID),
    PausePlay(ListSongID),
}

impl AppRequest {
    fn category(&self) -> RequestCategory {
        match self {
            AppRequest::SearchArtists(_) => RequestCategory::Search,
            AppRequest::GetSearchSuggestions(_) => RequestCategory::GetSearchSuggestions,
            AppRequest::GetArtistSongs(_) => RequestCategory::Get,
            AppRequest::Download(..) => RequestCategory::Download,
            AppRequest::IncreaseVolume(_) => RequestCategory::IncreaseVolume,
            AppRequest::GetVolume => RequestCategory::GetVolume,
            AppRequest::PlaySong(..) => RequestCategory::PlayPauseStop,
            AppRequest::GetPlayProgress(_) => RequestCategory::ProgressUpdate,
            AppRequest::Stop(_) => RequestCategory::PlayPauseStop,
            AppRequest::PausePlay(_) => RequestCategory::PlayPauseStop,
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
    IncreaseVolume, // TODO: generalize
    PlayPauseStop,
}

impl TaskManager {
    // This should handle messages as well.
    // TODO: Error handling
    pub fn new(api_key: ApiKey) -> Self {
        let (server_request_tx, server_request_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let (server_response_tx, server_response_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let (request_tx, request_rx) = mpsc::channel(MESSAGE_QUEUE_LENGTH);
        let _server_handle = tokio::spawn(async {
            let mut a = server::Server::new(api_key, server_response_tx, server_request_rx)?;
            a.run().await;
            Ok(())
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
            AppRequest::PlaySong(song, song_id) => self.spawn_play_song(song, song_id, id).await,
            AppRequest::GetPlayProgress(song_id) => self.spawn_get_play_progress(song_id, id).await,
            AppRequest::Stop(song_id) => self.spawn_stop(song_id, id).await,
            AppRequest::PausePlay(song_id) => self.spawn_pause_play(song_id, id).await,
        };
        Ok(())
    }
    // TODO: Consider if this should create it's own channel and return a KillableTask.
    fn add_task(
        &mut self,
        kill: tokio::sync::oneshot::Sender<KillRequest>,
        message: AppRequest,
    ) -> TaskID {
        // If we exceed usize, we'll overflow instead of crash.
        // The chance of a negative impact due to this logic should be extremely slim.
        let (new_id, overflowed) = self.cur_id.0.overflowing_add(1);
        self.cur_id.0 = new_id;
        if overflowed {
            warn!("Task ID generation has overflowed");
        }
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
        self.kill_all_task_type_except_id(RequestCategory::Search, id);
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
        self.kill_all_task_type_except_id(RequestCategory::GetSearchSuggestions, id);
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
        self.kill_all_task_type_except_id(RequestCategory::Get, id);
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
            // Does not kill previous tasks, as multiple concurrent downloads can occur.
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
        self.block_all_task_type_except_id(RequestCategory::IncreaseVolume, id);
        self.kill_all_task_type_except_id(RequestCategory::GetVolume, id);
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::IncreaseVolume(vol_inc, id)),
        )
        .await
    }
    pub async fn spawn_stop(&mut self, song_id: ListSongID, id: TaskID) {
        self.block_all_task_type_except_id(RequestCategory::PlayPauseStop, id);
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::Stop(song_id, id)),
        )
        .await
    }
    pub async fn spawn_pause_play(&mut self, song_id: ListSongID, id: TaskID) {
        self.block_all_task_type_except_id(RequestCategory::PlayPauseStop, id);
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::PausePlay(song_id, id)),
        )
        .await
    }
    pub async fn spawn_get_play_progress(&mut self, song_id: ListSongID, id: TaskID) {
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::GetPlayProgress(song_id, id)),
        )
        .await
    }
    pub async fn spawn_play_song(&mut self, song: Arc<Vec<u8>>, song_id: ListSongID, id: TaskID) {
        info!("Sending message to player to play song");
        self.block_all_task_type_except_id(RequestCategory::PlayPauseStop, id);
        send_or_error(
            &self.server_request_tx,
            server::Request::Player(server::player::Request::PlaySong(song, song_id, id)),
        )
        .await
    }
    pub async fn spawn_get_volume(&mut self, id: TaskID, kill_rx: oneshot::Receiver<KillRequest>) {
        self.block_all_task_type_except_id(RequestCategory::IncreaseVolume, id);
        self.kill_all_task_type_except_id(RequestCategory::GetVolume, id);
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
    pub fn kill_all_task_type_except_id(&mut self, request_category: RequestCategory, id: TaskID) {
        debug!(
            "Killing all pending {:?} tasks except {:?}",
            request_category, id,
        );
        for task in self
            .tasks
            .iter_mut()
            .filter(|x| x.message.category() == request_category && x.id != id)
        {
            if let Some(tx) = task.kill.take() {
                tx.send(KillRequest)
                    .unwrap_or_else(|_| error!("Error sending kill message"));
            }
        }
        self.tasks
            .retain(|x| x.message.category() != request_category || x.id == id);
    }
    // Stop receiving tasks from the category, but do not kill them.
    // TODO: generalize using enums/types.
    pub fn block_all_task_type_except_id(&mut self, request_category: RequestCategory, id: TaskID) {
        info!(
            "Blocking all pending {:?} tasks except {:?}",
            request_category, id
        );
        self.tasks
            .retain(|x| x.message.category() != request_category || x.id == id);
    }
    pub fn process_messages(&mut self) -> Vec<StateUpdateMessage> {
        // XXX: Consider general case to check if task is valid.
        // In this case, message could implement Task with get_id() function?
        let mut state_update_list = Vec::new();
        while let Ok(msg) = self.server_response_rx.try_recv() {
            match msg {
                server::Response::Api(msg) => {
                    if let Some(state_msg) = self.process_api_msg(msg) {
                        state_update_list.push(state_msg)
                    }
                }
                server::Response::Player(msg) => {
                    if let Some(state_msg) = self.process_player_msg(msg) {
                        state_update_list.push(state_msg)
                    }
                }
                server::Response::Downloader(msg) => {
                    if let Some(state_msg) = self.process_downloader_msg(msg) {
                        state_update_list.push(state_msg)
                    }
                }
            }
        }
        state_update_list
    }
    pub fn process_api_msg(&self, msg: api::Response) -> Option<StateUpdateMessage> {
        match msg {
            api::Response::ReplaceArtistList(list, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::ReplaceArtistList(list))
            }
            api::Response::SearchArtistError(id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::HandleSearchArtistError)
            }
            api::Response::ReplaceSearchSuggestions(runs, id, search) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::ReplaceSearchSuggestions(runs, search))
            }
            api::Response::SongListLoading(id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::HandleSongListLoading)
            }
            api::Response::SongListLoaded(id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::HandleSongListLoaded)
            }
            api::Response::NoSongsFound(id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::HandleNoSongsFound)
            }
            api::Response::SongsFound(id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::HandleSongsFound)
            }
            api::Response::AppendSongList {
                song_list,
                album,
                year,
                artist,
                id,
            } => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::AppendSongList {
                    song_list,
                    album,
                    year,
                    artist,
                })
            }
            api::Response::ApiError(e) => Some(StateUpdateMessage::HandleApiError(e)),
        }
    }
    pub fn process_downloader_msg(&self, msg: downloader::Response) -> Option<StateUpdateMessage> {
        match msg {
            downloader::Response::DownloadProgressUpdate(update_type, song_id, task_id) => {
                if !self.is_task_valid(task_id) {
                    return None;
                }
                Some(StateUpdateMessage::SetSongDownloadProgress(
                    update_type,
                    song_id,
                ))
            }
        }
    }
    pub fn process_player_msg(&self, msg: player::Response) -> Option<StateUpdateMessage> {
        match msg {
            // XXX: Why are these not blockable tasks? As receiver responsible for race conditions?
            // Is a task with race conditions a RaceConditionTask?
            player::Response::DonePlaying(song_id) => {
                Some(StateUpdateMessage::HandleDonePlaying(song_id))
            }
            player::Response::Paused(song_id, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::SetToPaused(song_id))
            }
            player::Response::Playing(song_id, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::SetToPlaying(song_id))
            }
            player::Response::Stopped(song_id, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::SetToStopped(song_id))
            }
            player::Response::ProgressUpdate(perc, song_id, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::SetSongPlayProgress(perc, song_id))
            }
            player::Response::VolumeUpdate(vol, id) => {
                if !self.is_task_valid(id) {
                    return None;
                }
                Some(StateUpdateMessage::SetVolume(vol))
            }
        }
    }
}
