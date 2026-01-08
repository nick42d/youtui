use crate::app::component::actionhandler::ComponentEffect;
use crate::app::server::song_downloader::{DownloadProgressUpdate, DownloadProgressUpdateType};
use crate::app::server::song_thumbnail_downloader::{SongThumbnail, SongThumbnailID};
use crate::app::server::{ArcServer, TaskMetadata};
use crate::app::structures::ListSongID;
use crate::app::ui::playlist::Playlist;
use crate::async_rodio_sink::{
    AllStopped, AutoplayUpdate, PausePlayResponse, Paused, PlayUpdate, ProgressUpdate, QueueUpdate,
    Resumed, Stopped, VolumeUpdate,
};
use async_callback_manager::{AsyncTask, FrontendEffect};
use rodio::decoder::DecoderError;
use std::fmt::Debug;
use std::option::Option;
use tracing::error;

#[derive(Debug, PartialEq)]
pub struct HandleAllStopped;
#[derive(Debug, PartialEq)]
pub struct HandleStopped;
#[derive(Debug, PartialEq)]
pub struct HandleSetSongPlayProgress;
#[derive(Debug, PartialEq)]
pub struct HandleVolumeUpdate;
#[derive(Debug, PartialEq)]
pub struct HandleGetSongThumbnailOk;
#[derive(Debug, PartialEq)]
pub struct HandlePausePlayResponse;
#[derive(Debug, PartialEq)]
pub struct HandleResumeResponse;
#[derive(Debug, PartialEq)]
pub struct HandlePausedResponse;
#[derive(Debug, PartialEq)]
pub struct HandleGetSongThumbnailError(pub SongThumbnailID<'static>);
#[derive(Debug, PartialEq, Clone)]
pub struct HandlePlayUpdateOk;
#[derive(Debug, PartialEq, Clone)]
pub struct HandleAutoplayUpdateOk;
#[derive(Debug, PartialEq, Clone)]
pub struct HandleQueueUpdateOk;
#[derive(Debug, PartialEq, Clone)]
pub struct HandlePlayUpdateError(pub ListSongID);
#[derive(Debug, PartialEq, Clone)]
pub struct HandleSongDownloadProgressUpdate;

#[derive(Debug, PartialEq)]
enum PlaylistEffect {
    SetStatusStoppedIfSome(Option<AllStopped>),
    StopSongIDIfSomeAndCur(Option<Stopped<ListSongID>>),
    HandleSetSongPlayProgress(ProgressUpdate<ListSongID>),
    HandleVolumeUpdate(Option<VolumeUpdate>),
    HandlePausePlayResponse(PausePlayResponse<ListSongID>),
    HandleResumed(ListSongID),
    HandlePaused(ListSongID),
    HandlePlayUpdate(PlayUpdate<ListSongID>),
    HandleQueueUpdate(QueueUpdate<ListSongID>),
    HandleAutoplayUpdate(AutoplayUpdate<ListSongID>),
    HandleSetToError(ListSongID),
    HandleSongDownloadProgressUpdate {
        kind: DownloadProgressUpdateType,
        id: ListSongID,
    },
    SetSongThumbnailError(SongThumbnailID<'static>),
    AddSongThumbnail(SongThumbnail),
}
impl_youtui_task_handler!(
    HandleStopped,
    Option<Stopped<ListSongID>>,
    Playlist,
    |_, input| PlaylistEffect::StopSongIDIfSomeAndCur(input)
);
impl_youtui_task_handler!(
    HandleAllStopped,
    Option<AllStopped>,
    Playlist,
    |_, input| PlaylistEffect::SetStatusStoppedIfSome(input)
);
impl_youtui_task_handler!(
    HandleSetSongPlayProgress,
    ProgressUpdate<ListSongID>,
    Playlist,
    |_, input| PlaylistEffect::HandleSetSongPlayProgress(input)
);
impl_youtui_task_handler!(
    HandleVolumeUpdate,
    Option<VolumeUpdate>,
    Playlist,
    |_, input| PlaylistEffect::HandleVolumeUpdate(input)
);
impl_youtui_task_handler!(
    HandlePlayUpdateOk,
    PlayUpdate<ListSongID>,
    Playlist,
    |_, input| PlaylistEffect::HandlePlayUpdate(input)
);
impl_youtui_task_handler!(
    HandleQueueUpdateOk,
    QueueUpdate<ListSongID>,
    Playlist,
    |_, input| PlaylistEffect::HandleQueueUpdate(input)
);
impl_youtui_task_handler!(
    HandleAutoplayUpdateOk,
    AutoplayUpdate<ListSongID>,
    Playlist,
    |_, input| PlaylistEffect::HandleAutoplayUpdate(input)
);
impl_youtui_task_handler!(
    HandlePlayUpdateError,
    DecoderError,
    Playlist,
    |this: HandlePlayUpdateError, input| {
        error!("Error {input} received when trying to decode {:?}", this.0);
        PlaylistEffect::HandleSetToError(this.0)
    }
);
impl_youtui_task_handler!(
    HandleSongDownloadProgressUpdate,
    DownloadProgressUpdate,
    Playlist,
    |_, input| {
        let DownloadProgressUpdate { kind, id } = input;
        PlaylistEffect::HandleSongDownloadProgressUpdate { kind, id }
    }
);
impl_youtui_task_handler!(
    HandleGetSongThumbnailOk,
    SongThumbnail,
    Playlist,
    |_, input| PlaylistEffect::AddSongThumbnail(input)
);
impl_youtui_task_handler!(
    HandleGetSongThumbnailError,
    anyhow::Error,
    Playlist,
    |this: HandleGetSongThumbnailError, input| {
        error!("Error {input} getting album art");
        // TODO: if GetSongThumbnail error sends back it's ID, one less clone
        // is required.
        PlaylistEffect::SetSongThumbnailError(this.0)
    }
);
impl_youtui_task_handler!(
    HandlePausePlayResponse,
    PausePlayResponse<ListSongID>,
    Playlist,
    |_, input| PlaylistEffect::HandlePausePlayResponse(input)
);
impl_youtui_task_handler!(
    HandleResumeResponse,
    Resumed<ListSongID>,
    Playlist,
    |_, input: Resumed<_>| PlaylistEffect::HandleResumed(input.0)
);
impl_youtui_task_handler!(
    HandlePausedResponse,
    Paused<ListSongID>,
    Playlist,
    |_, input: Paused<_>| PlaylistEffect::HandlePaused(input.0)
);

impl FrontendEffect<Playlist, ArcServer, TaskMetadata> for PlaylistEffect {
    fn apply(self, target: &mut Playlist) -> ComponentEffect<Playlist> {
        match self {
            PlaylistEffect::SetStatusStoppedIfSome(msg) => {
                target.handle_all_stopped(msg);
            }
            PlaylistEffect::StopSongIDIfSomeAndCur(msg) => {
                target.handle_stopped(msg);
            }
            PlaylistEffect::HandlePausePlayResponse(msg) => {
                // Logic could go in handler instead.
                match msg {
                    PausePlayResponse::Paused(id) => target.handle_paused(id),
                    PausePlayResponse::Resumed(id) => target.handle_resumed(id),
                };
            }
            PlaylistEffect::HandleResumed(msg) => target.handle_resumed(msg),
            PlaylistEffect::HandlePaused(msg) => target.handle_paused(msg),
            PlaylistEffect::HandleSetSongPlayProgress(msg) => {
                return target.handle_set_song_play_progress(msg.duration, msg.identifier);
            }
            PlaylistEffect::HandleVolumeUpdate(msg) => target.handle_volume_update(msg),
            PlaylistEffect::HandleQueueUpdate(msg) => return target.handle_queue_update(msg),
            PlaylistEffect::HandlePlayUpdate(msg) => return target.handle_play_update(msg),
            PlaylistEffect::HandleAutoplayUpdate(msg) => {
                return target.handle_autoplay_update(msg);
            }
            PlaylistEffect::HandleSetToError(msg) => target.handle_set_to_error(msg),
            PlaylistEffect::HandleSongDownloadProgressUpdate { kind, id } => {
                return target.handle_song_download_progress_update(kind, id);
            }
            PlaylistEffect::SetSongThumbnailError(msg) => target.list.set_song_thumbnail_error(msg),
            PlaylistEffect::AddSongThumbnail(msg) => target.list.add_song_thumbnail(msg),
        }
        AsyncTask::new_no_op()
    }
}
