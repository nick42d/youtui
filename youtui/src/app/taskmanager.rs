use super::server::messages::{KillableServerRequest, UnkillableServerRequest};
use super::server::{api, downloader, player};
use super::structures::ListSongID;
use super::ui::YoutuiWindow;
use crate::app::server::messages::{KillRequest, ServerRequest};
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
    server_request_tx: mpsc::Sender<ServerRequest>,
    server_response_rx: mpsc::Receiver<server::messages::ServerResponse>,
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

#[derive(PartialEq, Default, Debug, Copy, Clone)]
pub struct TaskID(usize);

enum TaskType {
    // A task that can be called by the caller.
    // Once killed, the caller
    // will stop receiving messages to prevent
    // race conditions.
    Killable(Option<oneshot::Sender<KillRequest>>),
    // A task that the caller can block from receiving further messages, but
    // cannot be killed.
    Unkillable,
}

enum TaskMessage {
    Killable(KillableServerRequest),
    Unkillable(UnkillableServerRequest),
}

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
    Seek(i8),
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
    PlayPause,
    PlayStop,
}

// Custom debug due to size
impl std::fmt::Debug for AppRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppRequest::SearchArtists(a) => f.debug_tuple("SearchArtists").field(a).finish(),
            AppRequest::GetSearchSuggestions(a) => {
                f.debug_tuple("GetSearchSuggestions").field(a).finish()
            }
            AppRequest::GetArtistSongs(a) => f.debug_tuple("GetArtistSongs").field(a).finish(),
            AppRequest::Download(a, b) => f.debug_tuple("Download").field(a).field(b).finish(),
            AppRequest::IncreaseVolume(a) => f.debug_tuple("IncreaseVolume").field(a).finish(),
            AppRequest::GetVolume => f.debug_tuple("GetVolume").finish(),
            AppRequest::PlaySong(_, b) => f
                .debug_tuple("PlaySong")
                .field(&"Arc<..>")
                .field(b)
                .finish(),
            AppRequest::GetPlayProgress(a) => f.debug_tuple("GetPlayProgress").field(a).finish(),
            AppRequest::Stop(a) => f.debug_tuple("Stop").field(a).finish(),
            AppRequest::PausePlay(a) => f.debug_tuple("PausePlay").field(a).finish(),
            AppRequest::Seek(a) => f.debug_tuple("Seek").field(a).finish(),
        }
    }
}

impl TaskManager {
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
        info!("Received app request: {:?}", request);
        // Kill needs to happen before block, block will prevent kill since it will drop
        // the kill senders.
        if let Some(k) = request.kill_category() {
            self.kill_all_task_type(k);
        };
        if let Some(b) = request.block_category() {
            self.block_all_task_type(b);
        };
        let category = request.category();
        let tx = self.server_request_tx.clone();
        let message = match request.into_kind() {
            TaskMessage::Killable(request) => {
                let killable_task = self.add_killable_task(category);
                ServerRequest::Killable {
                    killable_task,
                    request,
                }
            }
            TaskMessage::Unkillable(request) => {
                let task_id = self.add_unkillable_task(category);

                ServerRequest::Unkillable { task_id, request }
            }
        };
        debug!("Sending {:?}", message);
        send_or_error(tx, message).await;
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
            task_type: TaskType::Unkillable,
            id,
            category,
        });
        id
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
    /// Process ALL pending messages from the server.
    pub async fn action_messages(&mut self, ui_state: &mut YoutuiWindow) {
        while let Ok(msg) = self.server_response_rx.try_recv() {
            info!("Processing {:?}", msg);
            if !self.is_task_valid(msg.id) {
                info!("Task {:?} was no longer valid", msg.id);
                continue;
            }
            match msg.response {
                server::ServerResponseType::Api(msg) => self.process_api_msg(msg, ui_state).await,
                server::ServerResponseType::Player(msg) => {
                    self.process_player_msg(msg, ui_state).await
                }
                server::ServerResponseType::Downloader(msg) => {
                    self.process_downloader_msg(msg, ui_state).await
                }
            };
        }
    }
    pub async fn process_api_msg(&self, msg: api::Response, ui_state: &mut YoutuiWindow) {
        match msg {
            api::Response::ReplaceArtistList(list) => {
                ui_state.handle_replace_artist_list(list).await
            }
            api::Response::SearchArtistError => ui_state.handle_search_artist_error(),
            api::Response::ReplaceSearchSuggestions(runs, search) => {
                ui_state
                    .handle_replace_search_suggestions(runs, search)
                    .await
            }
            api::Response::SongListLoading => ui_state.handle_song_list_loading(),
            api::Response::SongListLoaded => ui_state.handle_song_list_loaded(),
            api::Response::NoSongsFound => ui_state.handle_no_songs_found(),
            api::Response::SongsFound => ui_state.handle_songs_found(),
            api::Response::AppendSongList {
                song_list,
                album,
                year,
                artist,
            } => ui_state.handle_append_song_list(song_list, album, year, artist),
        }
    }
    pub async fn process_downloader_msg(
        &self,
        msg: downloader::Response,
        ui_state: &mut YoutuiWindow,
    ) {
        match msg {
            downloader::Response::DownloadProgressUpdate(update_type, song_id) => {
                ui_state
                    .handle_set_song_download_progress(update_type, song_id)
                    .await;
            }
        }
    }
    pub async fn process_player_msg(&self, msg: player::Response, ui_state: &mut YoutuiWindow) {
        match msg {
            player::Response::DonePlaying(song_id) => ui_state.handle_done_playing(song_id).await,
            player::Response::Paused(song_id) => ui_state.handle_set_to_paused(song_id).await,
            player::Response::Playing(song_id) => ui_state.handle_set_to_playing(song_id).await,
            player::Response::Stopped(song_id) => ui_state.handle_set_to_stopped(song_id).await,
            player::Response::Error(song_id) => ui_state.handle_set_to_error(song_id).await,
            player::Response::ProgressUpdate(dur, song_id) => {
                ui_state.handle_set_song_play_progress(dur, song_id)
            }
            player::Response::VolumeUpdate(vol) => ui_state.handle_set_volume(vol),
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
            // Notionally, this could also be blocked by a Stop message.
            AppRequest::PlaySong(..) => RequestCategory::PlayStop,
            AppRequest::GetPlayProgress(_) => RequestCategory::ProgressUpdate,
            // Notionally, this could also be blocked by a PlaySong message.
            AppRequest::Stop(_) => RequestCategory::PlayStop,
            AppRequest::PausePlay(_) => RequestCategory::PlayPause,
            AppRequest::Seek(_) => RequestCategory::ProgressUpdate,
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
            AppRequest::Seek(inc) => TaskMessage::Unkillable(UnkillableServerRequest::Player(
                player::UnkillableServerRequest::Seek(inc),
            )),
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
            AppRequest::PlaySong(..) => Some(RequestCategory::PlayPause),
            AppRequest::GetPlayProgress(_) => None,
            AppRequest::Stop(_) => Some(RequestCategory::PlayPause),
            AppRequest::PausePlay(_) => Some(RequestCategory::PlayPause),
            AppRequest::Seek(_) => None,
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
            AppRequest::Seek(_) => None,
        }
    }
}
