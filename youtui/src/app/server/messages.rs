use super::{player::PlayerManager, spawn_run_or_kill, KillableTask, Server};
use crate::app::{structures::ListSongID, taskmanager::TaskID};
use std::sync::Arc;
use tracing::error;
use ytmapi_rs::common::{ChannelID, VideoID};

pub enum RequestEither {
    Killable {
        killable_task: KillableTask,
        request: KillableRequest,
    },
    Unkillable {
        task: TaskID,
        request: UnkillableRequest,
    },
}
enum KillableRequest {
    GetSearchSuggestions(String),
    NewArtistSearch(String),
    SearchSelectedArtist(ChannelID<'static>),
    DownloadSong {
        video_id: VideoID<'static>,
        song_id: ListSongID,
    },
    GetVolume,
}
enum UnkillableRequest {
    IncreaseVolume(i8),
    PlaySong {
        song: Arc<Vec<u8>>,
        song_id: ListSongID,
    },
    Stop(ListSongID),
    PausePlay(ListSongID),
}
impl KillableRequest {
    async fn task(self, id: TaskID, runner: &Server) {}
}
impl UnkillableRequest {
    async fn task(self, id: TaskID, runner: &Server) {}
}
impl RequestEither {
    async fn spawn(self, runner: &Server) {
        match self {
            RequestEither::Killable {
                killable_task,
                request,
            } => {
                let KillableTask { id, kill_rx } = killable_task;
                spawn_run_or_kill(request.task(id, runner), kill_rx)
            }
            RequestEither::Unkillable { task, request } => {
                request.get(runner).await;
                super::spawn_unkillable(request.task(task, runner))
            }
        }
    }
}
struct UnkillableTaskType<T> {
    kt: KillableTask,
    request: T,
}

struct TaskType<T> {
    t: TaskID,
    request: T,
}

trait KillableTasks
where
    Self: Sized,
{
    type Runner;
    async fn spawn(t: TaskType<Self>, runner: &mut Self::Runner);
}

trait Task {
    type Runner;
    async fn spawns(self, runner: &mut Self::Runner);
}

impl<T: KillableTasks> Task for TaskType<T> {
    type Runner = T::Runner;
    async fn spawns(self, runner: &mut Self::Runner) {
        T::spawn(self, runner).await
    }
}

impl KillableTasks for GetSearchSuggestions {
    type Runner = super::api::Api;
    async fn spawn(t: TaskType<Self>, runner: &mut Self::Runner) {
        tracing::info!("Getting search suggestions for {}", t.request.0);
        let query = ytmapi_rs::query::GetSearchSuggestionsQuery::new(&t.request.0);
        let search_suggestions =
            match super::api::query_api_with_retry(runner.get_api().await.unwrap(), query).await {
                Ok(t) => t,
                Err(e) => {
                    error!("Received error on search suggestions query \"{}\"", e);
                    return;
                }
            };
        tracing::info!("Requesting caller to replace search suggestions");
        let _ = tx
            .send(super::Response::Api(Response::ReplaceSearchSuggestions(
                search_suggestions,
                id,
                text,
            )))
            .await;
    }
}
