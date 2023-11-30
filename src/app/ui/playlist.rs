use crate::app::server::downloader::DownloadProgressUpdateType;
use crate::app::structures::Percentage;
use crate::app::view::draw::draw_table;
use crate::app::view::{BasicConstraint, DrawableMut, TableItem};
use crate::app::view::{Loadable, Scrollable, TableView};
use crate::app::{
    component::actionhandler::{
        Action, ActionHandler, ActionProcessor, KeyHandler, KeyRouter, Keybind, TextHandler,
    },
    structures::{AlbumSongsList, ListSong, ListSongID, PlayState},
    ui::{AppCallback, WindowContext},
};

use crate::{app::structures::DownloadStatus, core::send_or_error};
use crossterm::event::KeyCode;
use ratatui::{backend::Backend, layout::Rect, terminal::Frame};
use std::iter;
use std::sync::Arc;
use std::{borrow::Cow, fmt::Debug};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::YoutuiMutableState;

const SONGS_AHEAD_TO_BUFFER: usize = 3;
const SONGS_BEHIND_TO_SAVE: usize = 1;

pub struct Playlist {
    pub list: AlbumSongsList,
    pub cur_played_secs: Option<f64>,
    pub play_status: PlayState,
    pub volume: Percentage,
    ui_tx: mpsc::Sender<AppCallback>,
    pub help_shown: bool,
    keybinds: Vec<Keybind<PlaylistAction>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlaylistAction {
    ViewBrowser,
    Down,
    Up,
    PageDown,
    PageUp,
    PlaySelected,
    DeleteSelected,
    DeleteAll,
}

impl Action for PlaylistAction {
    fn context(&self) -> Cow<str> {
        "Playlist".into()
    }
    fn describe(&self) -> Cow<str> {
        match self {
            PlaylistAction::ViewBrowser => "View Browser",
            PlaylistAction::Down => "Down",
            PlaylistAction::Up => "Up",
            PlaylistAction::PageDown => "Page Down",
            PlaylistAction::PageUp => "Page Up",
            PlaylistAction::PlaySelected => "Play Selected",
            PlaylistAction::DeleteSelected => "Delete Selected",
            PlaylistAction::DeleteAll => "Delete All",
        }
        .into()
    }
}

impl KeyHandler<PlaylistAction> for Playlist {
    fn get_keybinds<'a>(
        &'a self,
    ) -> Box<
        dyn Iterator<Item = &'a crate::app::component::actionhandler::Keybind<PlaylistAction>> + 'a,
    > {
        Box::new(self.keybinds.iter())
    }
}
impl KeyRouter<PlaylistAction> for Playlist {
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<
        dyn Iterator<Item = &'a crate::app::component::actionhandler::Keybind<PlaylistAction>> + 'a,
    > {
        self.get_keybinds()
    }
}

impl ActionProcessor<PlaylistAction> for Playlist {}

impl TextHandler for Playlist {
    fn push_text(&mut self, _c: char) {}
    fn pop_text(&mut self) {}
    fn is_text_handling(&self) -> bool {
        false
    }
    fn take_text(&mut self) -> String {
        Default::default()
    }
    fn replace_text(&mut self, _text: String) {}
}

impl DrawableMut for Playlist {
    fn draw_mut_chunk<B: Backend>(
        &self,
        f: &mut Frame<B>,
        chunk: Rect,
        mutable_state: &mut YoutuiMutableState,
    ) {
        draw_table(f, self, chunk, &mut mutable_state.playlist, true);
    }
}

impl Loadable for Playlist {
    fn is_loading(&self) -> bool {
        false
    }
}

impl Scrollable for Playlist {
    fn increment_list(&mut self, amount: isize) {
        self.list.increment_list(amount)
    }
    fn get_selected_item(&self) -> usize {
        self.list.get_selected_item()
    }
}

impl TableView for Playlist {
    fn get_title(&self) -> Cow<str> {
        format!("Local playlist - {} songs", self.list.list.len()).into()
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
        Box::new(self.list.list.iter().enumerate().map(|(i, ls)| {
            Box::new(iter::once((i + 1).to_string().into()).chain(ls.get_fields_iter()))
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
}

impl ActionHandler<PlaylistAction> for Playlist {
    async fn handle_action(&mut self, action: &PlaylistAction) {
        match action {
            PlaylistAction::ViewBrowser => self.view_browser().await,
            PlaylistAction::Down => self.increment_list(1),
            PlaylistAction::Up => self.increment_list(-1),
            PlaylistAction::PageDown => self.increment_list(10),
            PlaylistAction::PageUp => self.increment_list(-10),
            PlaylistAction::PlaySelected => self.play_selected().await,
            PlaylistAction::DeleteSelected => self.delete_selected().await,
            PlaylistAction::DeleteAll => self.delete_all().await,
        }
    }
}

impl Playlist {
    pub fn new(ui_tx: mpsc::Sender<AppCallback>) -> Self {
        // This could fail, made to try send to avoid needing to change function signature to asynchronous. Should change.
        ui_tx
            .try_send(AppCallback::GetVolume)
            .unwrap_or_else(|e| error!("Error <{e}> received sending Get Volume message"));
        Playlist {
            help_shown: false,
            ui_tx,
            volume: Percentage(50),
            play_status: PlayState::NotPlaying,
            list: Default::default(),
            cur_played_secs: None,
            keybinds: playlist_keybinds(),
        }
    }
    pub async fn handle_tick(&mut self) {
        self.check_song_progress().await;
        // XXX: Consider downloading upcoming songs here.
        // self.download_upcoming_songs().await;
    }
    pub async fn check_song_progress(&mut self) {
        // Ask player for a progress update.
        if let PlayState::Playing(id) = self.play_status {
            info!("Tick received - requesting song progress update");
            let _ = self.ui_tx.send(AppCallback::GetProgress(id)).await;
        }
    }
    pub async fn handle_song_progress_update(
        &mut self,
        update: DownloadProgressUpdateType,
        id: ListSongID,
    ) {
        // Not valid if song doesn't exist or hasn't initiated download (i.e - task cancelled).
        if let Some(song) = self.get_song_from_id(id) {
            match song.download_status {
                DownloadStatus::None | DownloadStatus::Downloaded(_) | DownloadStatus::Failed => {
                    return
                }
                _ => (),
            }
        } else {
            return;
        }
        tracing::info!("Task valid - updating song download status");
        match update {
            DownloadProgressUpdateType::Started => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Queued;
                    // while let Ok(_) = self.player_rx.try_recv() {}
                }
            }
            DownloadProgressUpdateType::Completed(song_buf) => {
                let fut = self
                    .get_mut_song_from_id(id)
                    .map(|s| {
                        s.download_status = DownloadStatus::Downloaded(Arc::new(song_buf));
                        s.id
                    })
                    .map(|id| async move { self.play_if_was_buffering(id).await });
                if let Some(f) = fut {
                    f.await
                }
            }
            DownloadProgressUpdateType::Error => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Failed;
                }
            }
            DownloadProgressUpdateType::Downloading(p) => {
                if let Some(song) = self.list.list.iter_mut().find(|x| x.id == id) {
                    song.download_status = DownloadStatus::Downloading(p);
                }
            }
        }
    }
    pub fn handle_set_volume(&mut self, p: Percentage) {
        self.volume = p;
    }
    pub fn handle_set_song_play_progress(&mut self, f: f64, id: ListSongID) {
        if !self.check_id_is_cur(id) {
            return;
        }
        self.cur_played_secs = Some(f);
    }

    pub async fn handle_set_to_paused(&mut self, s_id: ListSongID) {
        if let PlayState::Playing(p_id) = self.play_status {
            if p_id == s_id {
                self.play_status = PlayState::Paused(s_id)
            }
        }
    }
    pub async fn handle_done_playing(&mut self, id: ListSongID) {
        self.play_next_or_finish(id).await;
    }
    pub fn handle_set_to_playing(&mut self, id: ListSongID) {
        if let PlayState::Paused(p_id) = self.play_status {
            if p_id == id {
                self.play_status = PlayState::Playing(id)
            }
        }
    }
    pub fn handle_set_to_stopped(&mut self, id: ListSongID) {
        info!("Received message to stop {:?}", id);
        if self.check_id_is_cur(id) {
            info!("Stopping {:?}", id);
            self.play_status = PlayState::Stopped
        }
    }
    pub async fn play_selected(&mut self) {
        let Some(index) = self.list.cur_selected else {
            return;
        };
        let Some(id) = self.get_id_from_index(index) else {
            return;
        };
        self.play_song_id(id).await;
    }
    pub async fn delete_selected(&mut self) {
        info!("Cur selected: {:?}", self.list.cur_selected);
        let Some(cur_selected_idx) = self.list.cur_selected else {
            return;
        };
        // If current song is playing, stop it.
        if let Some(cur_playing_id) = self.get_cur_playing_id() {
            if Some(cur_selected_idx) == self.get_cur_playing_index() {
                self.play_status = PlayState::NotPlaying;
                send_or_error(&self.ui_tx, AppCallback::Stop(cur_playing_id)).await;
            }
        }
        // TODO: Resolve offset commands
        // TODO: Test mut ListState functionality to see if a better substitute for using offsetcommands.
        self.list.remove_song_index(cur_selected_idx);
        // todo!("Fix visual bug where \"Not Playing\" displayed");
    }
    pub async fn delete_all(&mut self) {
        self.reset().await;
    }
    pub async fn view_browser(&mut self) {
        send_or_error(
            &self.ui_tx,
            AppCallback::ChangeContext(WindowContext::Browser),
        )
        .await;
    }
    pub async fn handle_next(&mut self) {
        match self.play_status {
            PlayState::Playing(id) => {
                self.play_next_or_finish(id).await;
            }
            _ => (),
        }
    }
    pub async fn handle_previous(&mut self) {
        self.play_prev().await;
    }
    pub fn increase_volume(&mut self, inc: i8) {
        // Update the volume in the UI for immediate visual feedback - response will be delayed one tick.
        // NOTE: could cause some visual race conditions.
        self.volume.0 = self.volume.0.saturating_add_signed(inc).clamp(0, 100);
    }
    // Returns the ID of the first song added.
    pub fn push_song_list(&mut self, song_list: Vec<ListSong>) -> ListSongID {
        self.list.push_song_list(song_list)
    }
    pub async fn play_if_was_buffering(&mut self, id: ListSongID) {
        if let PlayState::Buffering(target_id) = self.play_status {
            if target_id == id {
                info!("Playing");
                self.play_song_id(id).await;
            }
        }
    }
    pub async fn reset(&mut self) {
        // Stop playback, if playing.
        if let Some(cur_id) = self.get_cur_playing_id() {
            send_or_error(&self.ui_tx, AppCallback::Stop(cur_id)).await;
        }
        self.clear()
        // XXX: Also need to kill pending download tasks
        // Alternatively, songs could kill their own download tasks on drop (RAII).
    }
    pub fn clear(&mut self) {
        self.cur_played_secs = None;
        self.play_status = PlayState::NotPlaying;
        self.list.clear();
    }
    pub async fn play_song_id(&mut self, id: ListSongID) {
        if let Some(cur_id) = self.get_cur_playing_id() {
            send_or_error(&self.ui_tx, AppCallback::Stop(cur_id)).await;
        }
        // Drop previous songs
        self.drop_unscoped_from_id(id);
        // Queue next downloads
        self.download_upcoming_from_id(id).await;
        if let Some(song_index) = self.get_index_from_id(id) {
            if let DownloadStatus::Downloaded(pointer) = &self
                .list
                .list
                .get(song_index)
                .expect("Checked previously")
                .download_status
            {
                send_or_error(&self.ui_tx, AppCallback::PlaySong(pointer.clone(), id)).await;
                self.play_status = PlayState::Playing(id);
            } else {
                self.play_status = PlayState::Buffering(id);
            }
        }
    }
    pub async fn download_song_if_exists(&mut self, id: ListSongID) {
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let song = self
            .list
            .list
            .get_mut(song_index)
            .expect("We got the index from the id, so song must exist");
        // Won't download if already downloaded, or downloading.
        match song.download_status {
            DownloadStatus::Downloading(_)
            | DownloadStatus::Downloaded(_)
            | DownloadStatus::Queued => return,
            _ => (),
        };
        send_or_error(
            &self.ui_tx,
            AppCallback::DownloadSong(song.raw.get_video_id().clone(), id),
        )
        .await;
        song.download_status = DownloadStatus::Queued;
    }
    pub async fn play_next_or_finish(&mut self, prev_id: ListSongID) {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play next, but not currently playing");
            }
            PlayState::Paused(id) | PlayState::Playing(id) | PlayState::Buffering(id) => {
                // Guard against duplicate message received.
                if id > &prev_id {
                    return;
                }
                let next_song_id = self
                    .get_index_from_id(*id)
                    .map(|i| i + 1)
                    .and_then(|i| self.get_id_from_index(i));
                match next_song_id {
                    Some(id) => {
                        self.play_song_id(id).await;
                    }
                    None => {
                        info!("No next song - finishing playback");
                        send_or_error(&self.ui_tx, AppCallback::Stop(*id)).await;
                    }
                }
            }
        }
    }
    pub async fn download_upcoming_from_id(&mut self, id: ListSongID) {
        // Won't download if already downloaded.
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let mut song_ids_list = Vec::new();
        song_ids_list.push(id);
        for i in 1..SONGS_AHEAD_TO_BUFFER {
            let next_id = self.list.list.get(song_index + i).map(|song| song.id);
            if let Some(id) = next_id {
                song_ids_list.push(id);
            }
        }
        for song_id in song_ids_list {
            self.download_song_if_exists(song_id).await;
        }
    }
    /// Drop strong reference from previous songs or songs above the buffer list size to drop them from memory.
    pub fn drop_unscoped_from_id(&mut self, id: ListSongID) {
        let Some(song_index) = self.get_index_from_id(id) else {
            return;
        };
        let forward_limit = song_index + SONGS_AHEAD_TO_BUFFER;
        let backwards_limit = song_index.saturating_sub(SONGS_BEHIND_TO_SAVE);
        for song in self
            .list
            .list
            .get_mut(0..backwards_limit)
            .into_iter()
            .flatten()
        {
            // TODO: Also cancel in progress downloads
            // TODO: Write a change download status function that will warn if song is not dropped from memory.
            song.download_status = DownloadStatus::None
        }
        for song in self
            .list
            .list
            .get_mut(forward_limit..)
            .into_iter()
            .flatten()
        {
            // TODO: Also cancel in progress downloads
            // TODO: Write a change download status function that will warn if song is not dropped from memory.
            song.download_status = DownloadStatus::None
        }
    }
    pub async fn play_prev(&mut self) {
        let cur = &self.play_status;
        match cur {
            PlayState::NotPlaying | PlayState::Stopped => {
                warn!("Asked to play prev, but not currently playing");
            }
            PlayState::Paused(id) | PlayState::Playing(id) | PlayState::Buffering(id) => {
                let prev_song_id = self
                    .get_index_from_id(*id)
                    .and_then(|i| i.checked_sub(1))
                    .and_then(|i| self.list.list.get(i))
                    .map(|i| i.id);
                info!("Next song id {:?}", prev_song_id);
                match prev_song_id {
                    Some(id) => {
                        self.play_song_id(id).await;
                    }
                    None => {
                        // TODO: Reset song to start if got here.
                        warn!("No previous song. Doing nothing")
                    }
                }
            }
        }
    }
    pub async fn pauseplay(&mut self) {
        let id = match self.play_status {
            PlayState::Playing(id) => {
                self.play_status = PlayState::Paused(id);
                id
            }
            PlayState::Paused(id) => {
                self.play_status = PlayState::Playing(id);
                id
            }
            _ => return,
        };
        send_or_error(&self.ui_tx, AppCallback::PausePlay(id)).await;
    }
    pub fn get_cur_playing_id(&self) -> Option<ListSongID> {
        match self.play_status {
            PlayState::Playing(id) | PlayState::Paused(id) | PlayState::Buffering(id) => Some(id),
            _ => None,
        }
    }
    pub fn get_index_from_id(&self, id: ListSongID) -> Option<usize> {
        self.list.list.iter().position(|s| s.id == id)
    }
    pub fn get_id_from_index(&self, index: usize) -> Option<ListSongID> {
        self.list.list.get(index).map(|s| s.id)
    }
    pub fn get_mut_song_from_id(&mut self, id: ListSongID) -> Option<&mut ListSong> {
        self.list.list.iter_mut().find(|s| s.id == id)
    }
    pub fn get_song_from_id(&self, id: ListSongID) -> Option<&ListSong> {
        self.list.list.iter().find(|s| s.id == id)
    }
    pub fn check_id_is_cur(&self, check_id: ListSongID) -> bool {
        self.get_cur_playing_id().is_some_and(|id| id == check_id)
    }
    pub fn get_cur_playing_index(&self) -> Option<usize> {
        self.get_cur_playing_id()
            .and_then(|id| self.get_index_from_id(id))
    }
}

fn playlist_keybinds() -> Vec<Keybind<PlaylistAction>> {
    vec![
        Keybind::new_global_from_code(KeyCode::F(5), PlaylistAction::ViewBrowser),
        Keybind::new_from_code(KeyCode::Down, PlaylistAction::Down),
        Keybind::new_from_code(KeyCode::Up, PlaylistAction::Up),
        Keybind::new_from_code(KeyCode::PageDown, PlaylistAction::PageDown),
        Keybind::new_from_code(KeyCode::PageUp, PlaylistAction::PageUp),
        Keybind::new_action_only_mode(
            vec![
                (KeyCode::Enter, PlaylistAction::PlaySelected),
                (KeyCode::Char('d'), PlaylistAction::DeleteSelected),
                (KeyCode::Char('D'), PlaylistAction::DeleteAll),
            ],
            KeyCode::Enter,
            "Playlist Action",
        ),
    ]
}
