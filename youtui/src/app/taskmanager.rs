use super::server::{api, downloader, player, KillableServerRequest, UnkillableServerRequest};
use super::structures::ListSongID;
use super::ui::YoutuiWindow;
use crate::app::server::KillRequest;
use crate::app::server::{self};
use crate::config::ApiKey;
use crate::core::send_or_error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};
use ytmapi_rs::common::{ChannelID, VideoID};

const MESSAGE_QUEUE_LENGTH: usize = 256;

// XXX: Need to consider a mechanism for removing _completed_ tasks from the
// tasklist.
/// Middle layer between synchronous frontend and asynchronous, concurrent
/// backend. This is able to be called synchronously, to provide ergonomic
/// cancellation of server tasks, and better handle race conditions.
pub struct TaskManager {
    cur_id: TaskID,
    tasks: Vec<Task>,
    server_request_tx: mpsc::Sender<server::ServerRequest>,
    server_response_rx: mpsc::Receiver<server::Response>,
}

// Maybe should be in server
#[derive(Debug)]
pub struct KillableTask {
    pub id: TaskID,
    pub kill_rx: oneshot::Receiver<KillRequest>,
}

struct Task {
    id: TaskID,
    category: RequestCategory,
    task_type: TaskType,
}

enum TaskType {
    // A task that can be called by the caller.
    // Once killed, the caller
    // will stop receiving messages to prevent
    // race conditions.
    Killable(Option<oneshot::Sender<KillRequest>>),
    // A task that the caller can block from receiving further messages, but
    // cannot be killed.
    Blockable,
}

enum TaskMessage {
    Killable(KillableServerRequest),
    Unkillable(UnkillableServerRequest),
}

#[derive(PartialEq, Default, Debug, Copy, Clone)]
pub struct TaskID(usize);

#[derive(Debug)]
// App request MUST be an enum, whilst it's tempting to use structs here to
// take advantage of generics, every message sent to channel must be the same
// size.
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
        tokio::spawn(async {
            server::Server::new(api_key, server_response_tx, server_request_rx)
                .run()
                .await;
        });
        Self {
            cur_id: TaskID::default(),
            tasks: Vec::new(),
            server_request_tx,
            server_response_rx,
        }
    }
    pub async fn send_spawn_request(&mut self, request: AppRequest) {
        // Kill needs to happen before block, block will prevent kill since it will drop
        // the kill senders.
        if let Some(k) = request.kill_category() {
            self.kill_all_task_type(k);
        };
        if let Some(b) = request.block_category() {
            self.block_all_task_type(b);
        };
        let category = request.category();
        match request.into_kind() {
            TaskMessage::Killable(request) => {
                let killable_task = self.add_killable_task(category);
                send_or_error(
                    self.server_request_tx.clone(),
                    server::ServerRequest::Killable {
                        killable_task,
                        request,
                    },
                )
                .await;
            }
            TaskMessage::Unkillable(request) => {
                let task_id = self.add_unkillable_task(category);
                send_or_error(
                    self.server_request_tx.clone(),
                    server::ServerRequest::Unkillable { task_id, request },
                )
                .await;
            }
        };
    }
    /// Get the value next TaskID. Note that this could overflow, and will warn
    /// if it does.
    fn get_next_id(&mut self) -> TaskID {
        // If we exceed usize, we'll overflow instead of crash.
        // The chance of a negative impact due to this logic should be extremely slim.
        let (new_id, overflowed) = self.cur_id.0.overflowing_add(1);
        self.cur_id.0 = new_id;
        if overflowed {
            warn!("Task ID generation has overflowed");
        }
        self.cur_id
    }
    fn add_killable_task(&mut self, category: RequestCategory) -> KillableTask {
        let id = self.get_next_id();
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        self.tasks.push(Task {
            task_type: TaskType::Killable(Some(kill_tx)),
            id,
            category,
        });
        KillableTask { id, kill_rx }
    }
    fn add_unkillable_task(&mut self, category: RequestCategory) -> TaskID {
        let id = self.get_next_id();
        self.tasks.push(Task {
            task_type: TaskType::Blockable,
            id,
            category,
        });
        id
    }
    pub fn is_task_valid(&self, id: TaskID) -> bool {
        self.tasks.iter().any(|x| x.id == id)
    }
    pub fn kill_all_task_type(&mut self, request_category: RequestCategory) {
        debug!("Killing all pending {:?} tasks", request_category,);
        self.tasks.retain_mut(|x| {
            if x.category == request_category {
                if let TaskType::Killable(tx) = &mut x.task_type {
                    if let Some(tx) = tx.take() {
                        tx.send(KillRequest).unwrap_or_else(|_| {
                            info!("Tried to kill {:?}, but it had already completed", x.id)
                        });
                    }
                }
                return false;
            };
            true
        });
    }
    // Stop receiving tasks from the category, but do not kill them.
    pub fn block_all_task_type(&mut self, request_category: RequestCategory) {
        info!("Blocking all pending {:?} tasks", request_category);
        self.tasks.retain(|x| x.category != request_category);
    }
    pub async fn action_messages(&mut self, ui_state: &mut YoutuiWindow) {
        // XXX: Consider general case to check if task is valid.
        // In this case, message could implement Task with get_id() function?
        while let Ok(msg) = self.server_response_rx.try_recv() {
            match msg {
                server::Response::Api(msg) => self.process_api_msg(msg, ui_state).await,
                server::Response::Player(msg) => self.process_player_msg(msg, ui_state).await,
                server::Response::Downloader(msg) => {
                    self.process_downloader_msg(msg, ui_state).await
                }
            };
        }
    }
    pub async fn process_api_msg(&self, msg: api::Response, ui_state: &mut YoutuiWindow) {
        tracing::debug!("Processing {:?}", msg);
        match msg {
            api::Response::ReplaceArtistList(list, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_replace_artist_list(list).await;
            }
            api::Response::SearchArtistError(id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_search_artist_error();
            }
            api::Response::ReplaceSearchSuggestions(runs, id, search) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state
                    .handle_replace_search_suggestions(runs, search)
                    .await;
            }
            api::Response::SongListLoading(id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_song_list_loading();
            }
            api::Response::SongListLoaded(id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_song_list_loaded();
            }
            api::Response::NoSongsFound(id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_no_songs_found();
            }
            api::Response::SongsFound(id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_songs_found();
            }
            api::Response::AppendSongList {
                song_list,
                album,
                year,
                artist,
                id,
            } => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_append_song_list(song_list, album, year, artist);
            }
        }
    }
    pub async fn process_downloader_msg(
        &self,
        msg: downloader::Response,
        ui_state: &mut YoutuiWindow,
    ) {
        match msg {
            downloader::Response::DownloadProgressUpdate(update_type, song_id, task_id) => {
                if !self.is_task_valid(task_id) {
                    return;
                }
                ui_state
                    .handle_set_song_download_progress(update_type, song_id)
                    .await;
            }
        }
    }
    pub async fn process_player_msg(&self, msg: player::Response, ui_state: &mut YoutuiWindow) {
        match msg {
            // XXX: Why are these not blockable tasks? As receiver responsible for race conditions?
            // Is a task with race conditions a RaceConditionTask?
            player::Response::DonePlaying(song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_done_playing(song_id).await;
            }
            player::Response::Paused(song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_set_to_paused(song_id).await;
            }
            player::Response::Playing(song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_set_to_playing(song_id).await;
            }
            player::Response::Stopped(song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_set_to_stopped(song_id).await;
            }
            player::Response::Error(song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_set_to_error(song_id).await;
            }
            player::Response::ProgressUpdate(dur, song_id, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                // TODO: use duration properly
                ui_state.handle_set_song_play_progress(dur.as_secs_f64(), song_id);
            }
            player::Response::VolumeUpdate(vol, id) => {
                if !self.is_task_valid(id) {
                    return;
                }
                ui_state.handle_set_volume(vol);
            }
        }
    }
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
    fn into_kind(self) -> TaskMessage {
        match self {
            AppRequest::SearchArtists(artist) => TaskMessage::Killable(KillableServerRequest::Api(
                api::KillableServerRequest::NewArtistSearch(artist),
            )),
            AppRequest::GetSearchSuggestions(text) => TaskMessage::Killable(
                KillableServerRequest::Api(api::KillableServerRequest::GetSearchSuggestions(text)),
            ),
            AppRequest::GetArtistSongs(artist_channel) => {
                TaskMessage::Killable(KillableServerRequest::Api(
                    api::KillableServerRequest::SearchSelectedArtist(artist_channel),
                ))
            }
            AppRequest::Download(video_id, song_id) => {
                TaskMessage::Killable(KillableServerRequest::Downloader(
                    downloader::KillableServerRequest::DownloadSong(video_id, song_id),
                ))
            }
            AppRequest::IncreaseVolume(vol_inc) => {
                TaskMessage::Unkillable(UnkillableServerRequest::Player(
                    player::UnkillableServerRequest::IncreaseVolume(vol_inc),
                ))
            }
            AppRequest::GetVolume => TaskMessage::Killable(KillableServerRequest::Player(
                player::KillableServerRequest::GetVolume,
            )),
            AppRequest::PlaySong(song_pointer, song_id) => {
                TaskMessage::Unkillable(UnkillableServerRequest::Player(
                    player::UnkillableServerRequest::PlaySong(song_pointer, song_id),
                ))
            }
            AppRequest::GetPlayProgress(song_id) => {
                TaskMessage::Unkillable(UnkillableServerRequest::Player(
                    player::UnkillableServerRequest::GetPlayProgress(song_id),
                ))
            }
            AppRequest::Stop(song_id) => TaskMessage::Unkillable(UnkillableServerRequest::Player(
                player::UnkillableServerRequest::Stop(song_id),
            )),
            AppRequest::PausePlay(song_id) => {
                TaskMessage::Unkillable(UnkillableServerRequest::Player(
                    player::UnkillableServerRequest::PausePlay(song_id),
                ))
            }
        }
    }
    fn block_category(&self) -> Option<RequestCategory> {
        match self {
            AppRequest::SearchArtists(_) => None,
            AppRequest::GetSearchSuggestions(_) => None,
            AppRequest::GetArtistSongs(_) => None,
            AppRequest::Download(..) => None,
            AppRequest::IncreaseVolume(_) => Some(RequestCategory::IncreaseVolume),
            AppRequest::GetVolume => Some(RequestCategory::IncreaseVolume),
            AppRequest::PlaySong(..) => Some(RequestCategory::PlayPauseStop),
            AppRequest::GetPlayProgress(_) => None,
            AppRequest::Stop(_) => Some(RequestCategory::PlayPauseStop),
            AppRequest::PausePlay(_) => Some(RequestCategory::PlayPauseStop),
        }
    }
    fn kill_category(&self) -> Option<RequestCategory> {
        match self {
            AppRequest::SearchArtists(_) => Some(RequestCategory::Search),
            AppRequest::GetSearchSuggestions(_) => Some(RequestCategory::GetSearchSuggestions),
            AppRequest::GetArtistSongs(_) => Some(RequestCategory::Get),
            AppRequest::Download(..) => None,
            AppRequest::IncreaseVolume(_) => Some(RequestCategory::IncreaseVolume),
            AppRequest::GetVolume => Some(RequestCategory::GetVolume),
            AppRequest::PlaySong(..) => None,
            AppRequest::GetPlayProgress(_) => None,
            AppRequest::Stop(_) => None,
            AppRequest::PausePlay(_) => None,
        }
    }
}
