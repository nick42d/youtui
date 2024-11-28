use crate::app::component::actionhandler::ComponentEffect;
use crate::app::keycommand::KeyCommand;
use crate::app::server::downloader::{DownloadProgressUpdate, DownloadProgressUpdateType};
use crate::app::server::{
    ArcServer, AutoplaySong, DecodeSong, DownloadSong, IncreaseVolume, PausePlay, PlaySong,
    QueueSong, Seek, Stop, TaskMetadata,
};
use crate::app::structures::{Percentage, SongListComponent};
use crate::app::view::draw::draw_table;
use crate::app::view::{BasicConstraint, DrawableMut, TableItem};
use crate::app::view::{Loadable, Scrollable, TableView};
use crate::app::{
    component::actionhandler::{Action, KeyRouter, TextHandler},
    structures::{AlbumSongsList, ListSong, ListSongID, PlayState},
    ui::{AppCallback, WindowContext},
};
use crate::async_rodio_sink::{
    AutoplayUpdate, PausePlayResponse, PlayUpdate, QueueUpdate, SeekDirection, Stopped,
    VolumeUpdate,
};
use crate::config::{AppAction, Config, KeyEnum, KeyEnumKey};
use crate::{app::structures::DownloadStatus, core::send_or_error};
use async_callback_manager::{AsyncTask, Constraint, TryBackendTaskExt};
use ratatui::widgets::TableState;
use ratatui::{layout::Rect, Frame};
use serde::{Deserialize, Serialize};
use std::iter;
use std::option::Option;
use std::sync::Arc;
use std::time::Duration;
use std::{borrow::Cow, fmt::Debug};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const SONGS_AHEAD_TO_BUFFER: usize = 3;
const SONGS_BEHIND_TO_SAVE: usize = 1;
// How soon to trigger gapless playback
const GAPLESS_PLAYBACK_THRESHOLD: Duration = Duration::from_secs(1);

pub struct Playlist {
    pub list: AlbumSongsList,
    pub cur_played_dur: Option<Duration>,
    pub play_status: PlayState,
    pub queue_status: QueueState,
    pub volume: Percentage,
    ui_tx: mpsc::Sender<AppCallback>,
    keybinds: Vec<KeyCommand<AppAction>>,
    cur_selected: usize,
    pub widget_state: TableState,
}
impl_youtui_component!(Playlist);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistAction {
    ViewBrowser,
    PlaySelected,
    DeleteSelected,
    DeleteAll,
}

impl Action for PlaylistAction {
    type State = Playlist;
    fn context(&self) -> std::borrow::Cow<str> {
        "Playlist".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            PlaylistAction::ViewBrowser => "View Browser",
            PlaylistAction::PlaySelected => "Play Selected",
            PlaylistAction::DeleteSelected => "Delete Selected",
            PlaylistAction::DeleteAll => "Delete All",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            PlaylistAction::ViewBrowser => state.view_browser().await,
            PlaylistAction::PlaySelected => return state.play_selected(),
            PlaylistAction::DeleteSelected => return state.delete_selected(),
            PlaylistAction::DeleteAll => return state.delete_all(),
        }
        AsyncTask::new_no_op()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum QueueState {
    NotQueued,
    Queued(ListSongID),
}

impl KeyRouter<AppAction> for Playlist {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &'_ KeyCommand<AppAction>> + '_ {
        self.get_active_keybinds()
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &'_ KeyCommand<AppAction>> + '_ {
        self.keybinds.iter()
    }
}

impl TextHandler for Playlist {
    fn is_text_handling(&self) -> bool {
        false
    }
    fn get_text(&self) -> &str {
        Default::default()
    }
    fn replace_text(&mut self, _text: impl Into<String>) {}
    fn clear_text(&mut self) -> bool {
        false
    }
    fn handle_event_repr(
        &mut self,
        _event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        None
    }
}

impl DrawableMut for Playlist {
    fn draw_mut_chunk(&mut self, f: &mut Frame, chunk: Rect, selected: bool) {
        self.widget_state = draw_table(f, self, chunk, selected);
    }
}

impl Loadable for Playlist {
    fn is_loading(&self) -> bool {
        false
    }
}

impl Scrollable for Playlist {
    fn increment_list(&mut self, amount: isize) {
        self.cur_selected = self
            .cur_selected
            .saturating_add_signed(amount)
            .min(self.list.get_list_iter().len().saturating_sub(1))
    }
    fn get_selected_item(&self) -> usize {
        self.cur_selected
    }
}

impl TableView for Playlist {
    fn get_state(&self) -> TableState {
        self.widget_state.clone()
    }
    fn get_title(&self) -> Cow<str> {
        format!("Local playlist - {} songs", self.list.get_list_iter().len()).into()
    }
    fn get_layout(&self) -> &[BasicConstraint] {
        // Not perfect as this method doesn't know the size of the parent.
        // TODO: Change the get_layout function to something more appropriate.
        &[
            BasicConstraint::Length(3),
            BasicConstraint::Length(6),
            BasicConstraint::Length(3),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Percentage(Percentage(33)),
            BasicConstraint::Length(9),
            BasicConstraint::Length(4),
        ]
    }
    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = TableItem> + '_> {
        Box::new(self.list.get_list_iter().enumerate().map(|(i, ls)| {
            let first_field = if Some(i) == self.get_cur_playing_index() {
                match self.play_status {
                    PlayState::NotPlaying => ">>>".to_string(),
                    PlayState::Playing(_) => "".to_string(),
                    PlayState::Paused(_) => "".to_string(),
                    PlayState::Stopped => ">>>".to_string(),
                    PlayState::Error(_) => ">>>".to_string(),
                    PlayState::Buffering(_) => "".to_string(),
                }
            } else {
                (i + 1).to_string()
            };
            Box::new(iter::once(first_field.to_string().into()).chain(ls.get_fields_iter()))
                as Box<dyn Iterator<Item = Cow<str>>>
        }))
    }
    fn get_headings(&self) -> Box<(dyn Iterator<Item = &'static str> + 'static)> {
        Box::new(
            [
                "p#", "", "t#", "Artist", "Album", "Song", "Duration", "Year",
            ]
            .into_iter(),
        )
    }
    fn get_highlighted_row(&self) -> Option<usize> {
        self.get_cur_playing_index()
    }
}

impl SongListComponent for Playlist {
    fn get_song_from_idx(&self, idx: usize) -> Option<&ListSong> {
        self.list.get_list_iter().nth(idx)
    }
}

// Primatives
impl Playlist {
    /// When creating a Playlist, an effect is also created.
    pub fn new(ui_tx: mpsc::Sender<AppCallback>, config: &Config) -> (Self, ComponentEffect<Self>) {
        // Ensure volume is synced with player.
        let task = AsyncTask::new_future(
            // Since IncreaseVolume responds back with player volume after change, this is a
            // neat hack.
            IncreaseVolume(0),
            Self::handle_volume_update,
            Some(Constraint::new_block_same_type()),
        );
        let playlist = Playlist {
            ui_tx,
            volume: Percentage(50),
            play_status: PlayState::NotPlaying,
            list: Default::default(),
            cur_played_dur: None,
            keybinds: playlist_keybinds(config),
            cur_selected: 0,
            queue_status: QueueState::NotQueued,
            widget_state: Default::default(),
        };
        (playlist, task)
    }
    /// Add a task to:
    /// - Stop playback of the song 'song_id', if it is still playing.
    /// - If stop was succesful, update state.
    pub fn stop_song_id(&self, song_id: ListSongID) -> ComponentEffect<Self> {
        AsyncTask::new_future(
            Stop(song_id),
            Self::handle_stopped,
            Some(Constraint::new_block_matching_metadata(
                TaskMetadata::PlayPause,
            )),
        )
    }
    /// Drop downloads no longer relevant for ID, download new
    /// relevant downloads, start playing song at ID, set PlayState. If the
    /// selected song is buffering, stop playback until it's complete.
    pub fn play_song_id(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        // Drop previous songs
        self.drop_unscoped_from_id(id);
        // Queue next downloads
        let mut effect = self.download_upcoming_from_id(id);
        // Reset duration
        self.cur_played_dur = None;
        if let Some(song_index) = self.get_index_from_id(id) {
            if let DownloadStatus::Downloaded(pointer) = &self
                .get_song_from_idx(song_index)
                .expect("Checked previously")
                .download_status
            {
                // This task has the metadata of both DecodeSong and PlaySong and returns
                // Result<PlayUpdate>.
                let task =
                    DecodeSong(pointer.clone()).map_stream(move |song| PlaySong { song, id });
                let constraint = Some(Constraint::new_block_matching_metadata(
                    TaskMetadata::PlayingSong,
                ));
                let handle_update = move |this: &mut Self, update| match update {
                    Ok(u) => this.handle_play_update(u),
                    Err(e) => {
                        error!("Error {e} received when trying to decode {:?}", id);
                        this.handle_set_to_error(id);
                        AsyncTask::new_no_op()
                    }
                };
                let effect = effect.push(AsyncTask::new_stream_chained(
                    task,
                    handle_update,
                    constraint,
                ));
                self.play_status = PlayState::Playing(id);
                self.queue_status = QueueState::NotQueued;
                return effect;
            } else {
                // Stop current song, but only if next song is buffering.
                let maybe_effect = self
                    .get_cur_playing_id()
                    .map(|cur_id| self.stop_song_id(cur_id));
                self.play_status = PlayState::Buffering(id);
                self.queue_status = QueueState::NotQueued;
                if let Some(stop_effect) = maybe_effect {
                    effect = effect.push(stop_effect);
                }
            }
        }
        effect
    }
    /// Drop downloads no longer relevant for ID, download new
    /// relevant downloads, start playing song at ID, set PlayState.
    pub fn autoplay_song_id(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        // Drop previous songs
        self.drop_unscoped_from_id(id);
        // Queue next downloads
        let mut effect = self.download_upcoming_from_id(id);
        // Reset duration
        self.cur_played_dur = None;
        if let Some(song_index) = self.get_index_from_id(id) {
            if let DownloadStatus::Downloaded(pointer) = &self
                .get_song_from_idx(song_index)
                .expect("Checked previously")
                .download_status
            {
                // This task has the metadata of both DecodeSong and AutoplaySong and returns
                // Result<AutoplayUpdate>.
                let task =
                    DecodeSong(pointer.clone()).map_stream(move |song| AutoplaySong { song, id });
                let handle_update = move |this: &mut Self, update| match update {
                    Ok(u) => this.handle_autoplay_update(u),
                    Err(e) => {
                        error!("Error {e} received when trying to decode {:?}", id);
                        this.handle_set_to_error(id);
                        AsyncTask::new_no_op()
                    }
                };
                let effect = effect.push(AsyncTask::new_stream_chained(task, handle_update, None));
                self.play_status = PlayState::Playing(id);
                self.queue_status = QueueState::NotQueued;
                return effect;
            } else {
                // Stop current song, but only if next song is buffering.
                let maybe_effect = self
                    .get_cur_playing_id()
                    // TODO: Consider how race condition is supposed to be handled with this.
                    .map(|cur_id| self.stop_song_id(cur_id));
                self.play_status = PlayState::Buffering(id);
                self.queue_status = QueueState::NotQueued;
                if let Some(stop_effect) = maybe_effect {
                    effect = effect.push(stop_effect);
                }
            }
        };
        effect
    }
    /// Stop playing and clear playlist.
    pub fn reset(&mut self) -> ComponentEffect<Self> {
        let mut effect = AsyncTask::new_no_op();
        // Stop playback, if playing.
        if let Some(cur_id) = self.get_cur_playing_id() {
            // TODO: Consider how race condition is supposed to be handled with this.
            effect = self.stop_song_id(cur_id);
        }
        self.clear();
        effect
        // XXX: Also need to kill pending download tasks
        // Alternatively, songs could kill their own download tasks on drop
        // (RAII).
    }
    /// Clear all songs, reset PlayState to NotPlaying, set cur_played_dur to 0.
    pub fn clear(&mut self) {
        self.cur_played_dur = None;
        self.play_status = PlayState::NotPlaying;
        self.list.clear();
    }
    /// If currently playing, play previous song.
    pub fn play_prev(&mut self) -> ComponentEffect<Self> {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play prev, but not currently playing");
            }
            PlayState::Paused(id)
            | PlayState::Playing(id)
            | PlayState::Buffering(id)
            | PlayState::Error(id) => {
                let prev_song_id = self
                    .get_index_from_id(*id)
                    .and_then(|i| i.checked_sub(1))
                    .and_then(|i| self.get_song_from_idx(i))
                    .map(|i| i.id);
                info!("Next song id {:?}", prev_song_id);
                match prev_song_id {
                    Some(id) => {
                        return self.play_song_id(id);
                    }
                    None => {
                        // TODO: Reset song to start if got here.
                        warn!("No previous song. Doing nothing")
                    }
                }
            }
        }
        AsyncTask::new_no_op()
    }
    /// Play song at ID, if it was buffering.
    pub fn handle_song_downloaded(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        if let PlayState::Buffering(target_id) = self.play_status {
            if target_id == id {
                info!("Playing");
                return self.play_song_id(id);
            }
        }
        AsyncTask::new_no_op()
    }
    /// Download song at ID, if it is still in the list.
    pub fn download_song_if_exists(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        let Some(song_index) = self.get_index_from_id(id) else {
            return AsyncTask::new_no_op();
        };
        let song = self
            .list
            .get_list_iter_mut()
            .nth(song_index)
            .expect("We got the index from the id, so song must exist");
        // Won't download if already downloaded, or downloading.
        match song.download_status {
            DownloadStatus::Downloading(_)
            | DownloadStatus::Downloaded(_)
            | DownloadStatus::Queued => return AsyncTask::new_no_op(),
            _ => (),
        };
        // TODO: Consider how to handle race conditions.
        let effect = AsyncTask::new_stream_chained(
            DownloadSong(song.raw.video_id.clone(), id),
            |this: &mut Playlist, item| {
                let DownloadProgressUpdate { kind, id } = item;
                this.handle_song_download_progress_update(kind, id)
            },
            None,
        );
        song.download_status = DownloadStatus::Queued;
        effect
    }
    /// Update the volume in the UI for immediate visual feedback - response
    /// will be delayed one tick. Note that this does not actually change the
    /// volume!
    // NOTE: could cause some visual race conditions.
    pub fn increase_volume(&mut self, inc: i8) {
        self.volume.0 = self.volume.0.saturating_add_signed(inc).clamp(0, 100);
    }
    /// Add a song list to the playlist. Returns the ID of the first song added.
    pub fn push_song_list(&mut self, song_list: Vec<ListSong>) -> ListSongID {
        self.list.push_song_list(song_list)
        // Consider then triggering the download function.
    }
    /// Play the next song in the list if it exists, otherwise, stop playing.
    pub fn play_next_or_stop(&mut self, prev_id: ListSongID) -> ComponentEffect<Self> {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play next, but not currently playing");
                AsyncTask::new_no_op()
            }
            PlayState::Paused(id)
            | PlayState::Playing(id)
            | PlayState::Buffering(id)
            | PlayState::Error(id) => {
                // Guard against duplicate message received.
                if id > &prev_id {
                    return AsyncTask::new_no_op();
                }
                let next_song_id = self
                    .get_index_from_id(*id)
                    .map(|i| i + 1)
                    .and_then(|i| self.get_id_from_index(i));
                match next_song_id {
                    Some(id) => self.play_song_id(id),
                    None => {
                        info!("No next song - finishing playback");
                        self.queue_status = QueueState::NotQueued;
                        self.stop_song_id(*id)
                    }
                }
            }
        }
    }
    /// Autoplay the next song in the list if it exists, otherwise, set to
    /// stopped. This is triggered when a song has finished playing. The
    /// softer, Autoplay message, lets the Player use gapless playback if songs
    /// are queued correctly.
    pub fn autoplay_next_or_stop(&mut self, prev_id: ListSongID) -> ComponentEffect<Self> {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play next, but not currently playing");
                AsyncTask::new_no_op()
            }
            PlayState::Paused(id)
            | PlayState::Playing(id)
            | PlayState::Buffering(id)
            | PlayState::Error(id) => {
                // Guard against duplicate message received.
                if id > &prev_id {
                    return AsyncTask::new_no_op();
                }
                let next_song_id = self
                    .get_index_from_id(*id)
                    .map(|i| i + 1)
                    .and_then(|i| self.get_id_from_index(i));
                match next_song_id {
                    Some(id) => self.autoplay_song_id(id),
                    None => {
                        info!("No next song - resetting play status");
                        self.queue_status = QueueState::NotQueued;
                        // As a neat hack I only need to ask the player to stop current ID - even if
                        // it's playing the queued track, it doesn't know about it.
                        self.stop_song_id(*id)
                    }
                }
            }
        }
    }
    /// Download some upcoming songs, if they aren't already downloaded.
    pub fn download_upcoming_from_id(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        // Won't download if already downloaded.
        let Some(song_index) = self.get_index_from_id(id) else {
            return AsyncTask::new_no_op();
        };
        let mut song_ids_list = Vec::new();
        song_ids_list.push(id);
        for i in 1..SONGS_AHEAD_TO_BUFFER {
            let next_id = self.get_song_from_idx(song_index + i).map(|song| song.id);
            if let Some(id) = next_id {
                song_ids_list.push(id);
            }
        }
        // TODO: Don't love the way metadata and constraints are handled with this task
        // type that is collected, find a better way.
        song_ids_list
            .into_iter()
            .map(|song_id| self.download_song_if_exists(song_id))
            .collect()
    }
    /// Drop strong reference from previous songs or songs above the buffer list
    /// size to drop them from memory.
    pub fn drop_unscoped_from_id(&mut self, id: ListSongID) {
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let forward_limit = song_index + SONGS_AHEAD_TO_BUFFER;
        let backwards_limit = song_index.saturating_sub(SONGS_BEHIND_TO_SAVE);
        for song in self.list.get_list_iter_mut().take(backwards_limit) {
            // TODO: Also cancel in progress downloads
            // TODO: Write a change download status function that will warn if song is not
            // dropped from memory.
            song.download_status = DownloadStatus::None
        }
        for song in self.list.get_list_iter_mut().skip(forward_limit) {
            // TODO: Also cancel in progress downloads
            // TODO: Write a change download status function that will warn if song is not
            // dropped from memory.
            song.download_status = DownloadStatus::None
        }
    }
    pub fn get_cur_playing_id(&self) -> Option<ListSongID> {
        match self.play_status {
            PlayState::Error(id)
            | PlayState::Playing(id)
            | PlayState::Paused(id)
            | PlayState::Buffering(id) => Some(id),
            PlayState::NotPlaying | PlayState::Stopped => None,
        }
    }
    pub fn get_cur_playing_song(&self) -> Option<&ListSong> {
        self.get_cur_playing_id()
            .and_then(|id| self.get_song_from_id(id))
    }
    pub fn get_next_song(&self) -> Option<&ListSong> {
        self.get_cur_playing_id()
            .and_then(|id| self.get_index_from_id(id))
            .and_then(|idx| self.list.get_list_iter().nth(idx + 1))
    }
    pub fn get_index_from_id(&self, id: ListSongID) -> Option<usize> {
        self.list.get_list_iter().position(|s| s.id == id)
    }
    pub fn get_id_from_index(&self, index: usize) -> Option<ListSongID> {
        self.get_song_from_idx(index).map(|s| s.id)
    }
    pub fn get_mut_song_from_id(&mut self, id: ListSongID) -> Option<&mut ListSong> {
        self.list.get_list_iter_mut().find(|s| s.id == id)
    }
    pub fn get_song_from_id(&self, id: ListSongID) -> Option<&ListSong> {
        self.list.get_list_iter().find(|s| s.id == id)
    }
    pub fn check_id_is_cur(&self, check_id: ListSongID) -> bool {
        self.get_cur_playing_id().is_some_and(|id| id == check_id)
    }
    pub fn get_cur_playing_index(&self) -> Option<usize> {
        self.get_cur_playing_id()
            .and_then(|id| self.get_index_from_id(id))
    }
}
// Event handlers
impl Playlist {
    // Placeholder for future use
    pub async fn handle_tick(&mut self) {
        // XXX: Consider downloading upcoming songs here.
        // self.download_upcoming_songs().await;
    }
    /// Handle seek command (from global keypress).
    pub fn handle_seek(
        &mut self,
        duration: Duration,
        direction: SeekDirection,
    ) -> ComponentEffect<Self> {
        // Consider if we also want to update current duration.
        AsyncTask::new_future_chained(
            Seek {
                duration,
                direction,
            },
            |this: &mut Playlist, response| {
                let Some(response) = response else {
                    return AsyncTask::new_no_op();
                };
                this.handle_set_song_play_progress(response.duration, response.identifier)
            },
            None,
        )
    }
    /// Handle next command (from global keypress), if currently playing.
    pub fn handle_next(&mut self) -> ComponentEffect<Self> {
        match self.play_status {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play next, but not currently playing");
                AsyncTask::new_no_op()
            }
            PlayState::Paused(id)
            | PlayState::Playing(id)
            | PlayState::Buffering(id)
            | PlayState::Error(id) => self.play_next_or_stop(id),
        }
    }
    /// Handle previous command (from global keypress).
    pub fn handle_previous(&mut self) -> ComponentEffect<Self> {
        self.play_prev()
    }
    /// Play the song under the cursor (from local keypress)
    pub fn play_selected(&mut self) -> ComponentEffect<Self> {
        let Some(id) = self.get_id_from_index(self.cur_selected) else {
            return AsyncTask::new_no_op();
        };
        self.play_song_id(id)
    }
    /// Delete the song under the cursor (from local keypress). If it was
    /// playing, stop it and set PlayState to NotPlaying.
    pub fn delete_selected(&mut self) -> ComponentEffect<Self> {
        let mut return_task = AsyncTask::new_no_op();
        let cur_selected_idx = self.cur_selected;
        // If current song is playing, stop it.
        if let Some(cur_playing_id) = self.get_cur_playing_id() {
            if Some(cur_selected_idx) == self.get_cur_playing_index() {
                self.play_status = PlayState::NotPlaying;
                return_task = self.stop_song_id(cur_playing_id);
            }
        }
        self.list.remove_song_index(cur_selected_idx);
        // If we are removing a song at a position less than current index, decrement
        // current index. NOTE: Ok to simply take, if list only had one element.
        if self.cur_selected >= cur_selected_idx && cur_selected_idx != 0 {
            // Safe, as checked above that cur_idx >= 0
            self.cur_selected -= 1;
        };
        return_task
    }
    /// Delete all songs.
    pub fn delete_all(&mut self) -> ComponentEffect<Self> {
        self.reset()
    }
    /// Change to Browser window.
    pub async fn view_browser(&mut self) {
        send_or_error(
            &self.ui_tx,
            AppCallback::ChangeContext(WindowContext::Browser),
        )
        .await;
    }
    /// Handle global pause/play action. Toggle state (visual), toggle playback
    /// (server).
    pub fn pauseplay(&mut self) -> ComponentEffect<Self> {
        let id = match self.play_status {
            PlayState::Playing(id) => {
                self.play_status = PlayState::Paused(id);
                id
            }
            PlayState::Paused(id) => {
                self.play_status = PlayState::Playing(id);
                id
            }
            _ => return AsyncTask::new_no_op(),
        };
        AsyncTask::new_future(
            PausePlay(id),
            |this: &mut Playlist, response| {
                let Some(response) = response else { return };
                match response {
                    PausePlayResponse::Paused(id) => this.handle_paused(id),
                    PausePlayResponse::Resumed(id) => this.handle_resumed(id),
                };
            },
            Some(Constraint::new_block_matching_metadata(
                TaskMetadata::PlayPause,
            )),
        )
    }
}
// Server handlers
impl Playlist {
    /// Handle song progress update from server.
    pub fn handle_song_download_progress_update(
        &mut self,
        update: DownloadProgressUpdateType,
        id: ListSongID,
    ) -> ComponentEffect<Self> {
        // Not valid if song doesn't exist or hasn't initiated download (i.e - task
        // cancelled).
        if let Some(song) = self.get_song_from_id(id) {
            match song.download_status {
                DownloadStatus::None | DownloadStatus::Downloaded(_) | DownloadStatus::Failed => {
                    return AsyncTask::new_no_op()
                }
                _ => (),
            }
        } else {
            return AsyncTask::new_no_op();
        }
        tracing::info!("Task valid - updating song download status");
        match update {
            DownloadProgressUpdateType::Started => {
                if let Some(song) = self.list.get_list_iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Queued;
                }
            }
            DownloadProgressUpdateType::Completed(song_buf) => {
                if let Some(new_id) = self.get_mut_song_from_id(id).map(|s| {
                    s.download_status = DownloadStatus::Downloaded(Arc::new(song_buf));
                    s.id
                }) {
                    return self.handle_song_downloaded(new_id);
                };
            }
            DownloadProgressUpdateType::Error => {
                if let Some(song) = self.list.get_list_iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Failed;
                }
            }
            DownloadProgressUpdateType::Retrying { times_retried } => {
                if let Some(song) = self.list.get_list_iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Retrying { times_retried };
                }
            }
            DownloadProgressUpdateType::Downloading(p) => {
                if let Some(song) = self.list.get_list_iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Downloading(p);
                }
            }
        }
        AsyncTask::new_no_op()
    }
    /// Handle volume message from server
    pub fn handle_volume_update(&mut self, response: Option<VolumeUpdate>) {
        if let Some(v) = response {
            self.volume = Percentage(v.0.into())
        }
    }
    pub fn handle_play_update(&mut self, update: PlayUpdate<ListSongID>) -> ComponentEffect<Self> {
        match update {
            PlayUpdate::PlayProgress(duration, id) => {
                return self.handle_set_song_play_progress(duration, id)
            }
            PlayUpdate::Playing(duration, id) => self.handle_playing(duration, id),
            PlayUpdate::DonePlaying(id) => return self.handle_done_playing(id),
            // This is a player invariant.
            PlayUpdate::Error(e) => error!("{e}"),
        }
        AsyncTask::new_no_op()
    }
    pub fn handle_queue_update(
        &mut self,
        update: QueueUpdate<ListSongID>,
    ) -> ComponentEffect<Self> {
        match update {
            QueueUpdate::PlayProgress(duration, id) => {
                return self.handle_set_song_play_progress(duration, id)
            }
            QueueUpdate::Queued(duration, id) => self.handle_queued(duration, id),
            QueueUpdate::DonePlaying(id) => return self.handle_done_playing(id),
            QueueUpdate::Error(e) => error!("{e}"),
        }
        AsyncTask::new_no_op()
    }
    pub fn handle_autoplay_update(
        &mut self,
        update: AutoplayUpdate<ListSongID>,
    ) -> ComponentEffect<Self> {
        match update {
            AutoplayUpdate::PlayProgress(duration, id) => {
                return self.handle_set_song_play_progress(duration, id)
            }
            AutoplayUpdate::Playing(duration, id) => self.handle_playing(duration, id),
            AutoplayUpdate::DonePlaying(id) => return self.handle_done_playing(id),
            AutoplayUpdate::AutoplayQueued(id) => self.handle_autoplay_queued(id),
            AutoplayUpdate::Error(e) => error!("{e}"),
        }
        AsyncTask::new_no_op()
    }
    /// Handle song progress message from server
    pub fn handle_set_song_play_progress(
        &mut self,
        d: Duration,
        id: ListSongID,
    ) -> ComponentEffect<Self> {
        if !self.check_id_is_cur(id) {
            return AsyncTask::new_no_op();
        }
        self.cur_played_dur = Some(d);
        // If less than the gapless playback threshold remaining, queue up the next
        // song, if it's downloaded, and hasn't already been queued.
        if let Some(duration_dif) = {
            let cur_dur = self
                .get_cur_playing_song()
                .and_then(|song| song.actual_duration);
            self.cur_played_dur
                .as_ref()
                .zip(cur_dur)
                .map(|(d1, d2)| d2.saturating_sub(*d1))
        } {
            if duration_dif
                .saturating_sub(GAPLESS_PLAYBACK_THRESHOLD)
                .is_zero()
                && !matches!(self.queue_status, QueueState::Queued(_))
            {
                if let Some(next_song) = self.get_next_song() {
                    if let DownloadStatus::Downloaded(song) = &next_song.download_status {
                        // This task has the metadata of both DecodeSong and QueueSong and returns
                        // Result<QueueUpdate>.
                        let task =
                            DecodeSong(song.clone()).map_stream(move |song| QueueSong { song, id });
                        info!("Queuing up song!");
                        let handle_update = move |this: &mut Self, update| match update {
                            Ok(u) => this.handle_queue_update(u),
                            Err(e) => {
                                error!("Error {e} received when trying to decode {:?}", id);
                                this.handle_set_to_error(id);
                                AsyncTask::new_no_op()
                            }
                        };
                        let effect = AsyncTask::new_stream_chained(task, handle_update, None);
                        self.queue_status = QueueState::Queued(next_song.id);
                        return effect;
                    }
                }
            }
        }
        AsyncTask::new_no_op()
    }
    /// Handle done playing message from server
    pub fn handle_done_playing(&mut self, id: ListSongID) -> ComponentEffect<Self> {
        if QueueState::Queued(id) == self.queue_status {
            self.queue_status = QueueState::NotQueued;
            return AsyncTask::new_no_op();
        }
        if !self.check_id_is_cur(id) {
            return AsyncTask::new_no_op();
        }
        self.autoplay_next_or_stop(id)
    }
    /// Handle queued message from server
    pub fn handle_queued(&mut self, duration: Option<Duration>, id: ListSongID) {
        if let Some(song) = self.get_mut_song_from_id(id) {
            song.actual_duration = duration;
        }
    }
    /// Handle autoplay queued message from server.
    /// This message means that the song that was queued up has played
    /// successfully.
    /// If this occurs, we can clear the queued track since we know that it's
    /// playing.
    pub fn handle_autoplay_queued(&mut self, id: ListSongID) {
        match self.queue_status {
            QueueState::NotQueued => (),
            QueueState::Queued(q_id) => {
                if id == q_id {
                    self.queue_status = QueueState::NotQueued
                }
            }
        }
    }
    /// Handle playing message from server
    pub fn handle_playing(&mut self, duration: Option<Duration>, id: ListSongID) {
        // NOTE: Happens twice, if song already was queued.
        if let Some(song) = self.get_mut_song_from_id(id) {
            song.actual_duration = duration;
        }
        if let PlayState::Paused(p_id) = self.play_status {
            if p_id == id {
                self.play_status = PlayState::Playing(id)
            }
        }
    }
    /// Handle set to error message from server (playback)
    pub fn handle_set_to_error(&mut self, id: ListSongID) {
        info!("Received message that song had a playback error {:?}", id);
        if self.check_id_is_cur(id) {
            info!("Setting song state to Error {:?}", id);
            self.play_status = PlayState::Error(id)
        }
    }
    /// Handle set to paused message from server
    pub fn handle_paused(&mut self, s_id: ListSongID) {
        if let PlayState::Playing(p_id) = self.play_status {
            if p_id == s_id {
                self.play_status = PlayState::Paused(s_id)
            }
        }
    }
    /// Handle resumed message from server
    pub fn handle_resumed(&mut self, id: ListSongID) {
        if let PlayState::Paused(p_id) = self.play_status {
            if p_id == id {
                self.play_status = PlayState::Playing(id)
            }
        }
    }
    /// Handle stopped message from server
    pub fn handle_stopped(&mut self, id: Option<Stopped<ListSongID>>) {
        let Some(Stopped(id)) = id else { return };
        // TODO: Hoist info up.
        info!("Received message that playback {:?} has been stopped", id);
        if self.check_id_is_cur(id) {
            info!("Stopping {:?}", id);
            self.play_status = PlayState::Stopped
        }
    }
}

fn playlist_keybinds(config: &Config) -> Vec<KeyCommand<AppAction>> {
    config
        .keybinds
        .playlist
        .iter()
        .map(|(kb, ke)| match ke {
            KeyEnum::Key(KeyEnumKey {
                action,
                value,
                visibility,
            }) => KeyCommand::new_modified_from_code_with_visibility(
                kb.code,
                kb.modifiers,
                visibility.clone(),
                action.clone(),
            ),
            KeyEnum::Mode(_) => todo!(),
        })
        .collect()
}
