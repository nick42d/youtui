use self::{
    artistalbums::{albumsongs::AlbumSongsPanel, artistsearch::ArtistSearchPanel},
    draw::draw_browser,
};
use super::{
    action::{AppAction, ListAction, TextEntryAction},
    AppCallback, WindowContext,
};
use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, Component, ComponentEffect, DominantKeyRouter, KeyRouter,
            Suggestable, TextHandler,
        },
        server::{
            api::GetArtistSongsProgressUpdate, ArcServer, GetArtistSongs, SearchArtists,
            TaskMetadata,
        },
        structures::{ListStatus, SongListComponent},
        view::{DrawableMut, ListView, Scrollable, TableView},
    },
    config::keymap::Keymap,
};
use crate::{config::Config, core::send_or_error};
use async_callback_manager::{AsyncTask, Constraint};
use itertools::Either;
use serde::{Deserialize, Serialize};
use std::{iter::Iterator, mem};
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::{
    common::{AlbumID, SearchSuggestion},
    parse::{AlbumSong, SearchResultArtist},
};

const PAGE_KEY_LINES: isize = 10;

pub mod artistalbums;
mod draw;

#[derive(PartialEq)]
pub enum InputRouting {
    Artist,
    Song,
}

pub struct Browser {
    pub callback_tx: mpsc::Sender<AppCallback>,
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub artist_list: ArtistSearchPanel,
    pub album_songs_list: AlbumSongsPanel,
    keybinds: Keymap<AppAction>,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    ViewPlaylist,
    Search,
    Left,
    Right,
}

impl Action for BrowserAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Browser".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            BrowserAction::ViewPlaylist => "View Playlist",
            BrowserAction::Search => "Toggle Search",
            BrowserAction::Left => "Left",
            BrowserAction::Right => "Right",
        }
        .into()
    }
}

impl InputRouting {
    pub fn left(&self) -> Self {
        match self {
            Self::Song => Self::Artist,
            Self::Artist => Self::Artist,
        }
    }
    pub fn right(&self) -> Self {
        match self {
            Self::Artist => Self::Song,
            Self::Song => Self::Song,
        }
    }
}
impl Scrollable for Browser {
    fn increment_list(&mut self, amount: isize) {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.increment_list(amount),
            InputRouting::Song => self.artist_list.increment_list(amount),
        }
    }
    fn is_scrollable(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.is_scrollable(),
            InputRouting::Song => self.artist_list.is_scrollable(),
        }
    }
}
impl ActionHandler<BrowserAction> for Browser {
    async fn apply_action(
        &mut self,
        action: BrowserAction,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self>
    where
        Self: Sized,
    {
        match action {
            BrowserAction::Left => self.left(),
            BrowserAction::Right => self.right(),
            BrowserAction::ViewPlaylist => {
                send_or_error(
                    &self.callback_tx,
                    AppCallback::ChangeContext(WindowContext::Playlist),
                )
                .await
            }
            BrowserAction::Search => self.handle_toggle_search(),
        }
        AsyncTask::new_no_op()
    }
}
// Should this really be implemented on the Browser...
impl Suggestable for Browser {
    fn get_search_suggestions(&self) -> &[SearchSuggestion] {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.get_search_suggestions(),
            InputRouting::Song => &[],
        }
    }
    fn has_search_suggestions(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.has_search_suggestions(),
            InputRouting::Song => false,
        }
    }
}
impl TextHandler for Browser {
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.is_text_handling(),
            InputRouting::Song => self.album_songs_list.is_text_handling(),
        }
    }
    fn get_text(&self) -> &str {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.get_text(),
            InputRouting::Song => self.album_songs_list.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.replace_text(text),
            InputRouting::Song => self.album_songs_list.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.clear_text(),
            InputRouting::Song => self.album_songs_list.clear_text(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.input_routing {
            InputRouting::Artist => self
                .artist_list
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.artist_list)),
            InputRouting::Song => self
                .album_songs_list
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.album_songs_list)),
        }
    }
}

impl DrawableMut for Browser {
    fn draw_mut_chunk(
        &mut self,
        f: &mut ratatui::Frame,
        chunk: ratatui::prelude::Rect,
        selected: bool,
    ) {
        draw_browser(f, self, chunk, selected);
    }
}
impl KeyRouter<AppAction> for Browser {
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        std::iter::once(&self.keybinds)
            .chain(self.artist_list.get_all_keybinds())
            .chain(self.album_songs_list.get_all_keybinds())
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        let additional_binds = match self.input_routing {
            InputRouting::Song => Either::Left(self.album_songs_list.get_active_keybinds()),
            InputRouting::Artist => Either::Right(self.artist_list.get_active_keybinds()),
        };
        // TODO: Better implementation
        if self.album_songs_list.dominant_keybinds_active()
            || self.album_songs_list.dominant_keybinds_active()
        {
            Either::Left(additional_binds)
        } else {
            Either::Right(std::iter::once(&self.keybinds).chain(additional_binds))
        }
    }
}
impl DominantKeyRouter<AppAction> for Browser {
    fn dominant_keybinds_active(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => false,
            InputRouting::Song => self.album_songs_list.dominant_keybinds_active(),
        }
    }
    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        match self.input_routing {
            InputRouting::Artist => Either::Left(self.artist_list.get_active_keybinds()),
            InputRouting::Song => Either::Right(self.album_songs_list.get_dominant_keybinds()),
        }
    }
}

impl Browser {
    pub fn new(ui_tx: mpsc::Sender<AppCallback>, config: &Config) -> Self {
        Self {
            callback_tx: ui_tx,
            artist_list: ArtistSearchPanel::new(config),
            album_songs_list: AlbumSongsPanel::new(config),
            input_routing: InputRouting::Artist,
            prev_input_routing: InputRouting::Artist,
            keybinds: config.keybinds.browser.clone(),
        }
    }
    pub fn left(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.left();
    }
    pub fn right(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.right();
    }
    pub fn handle_list_action(&mut self, action: ListAction) -> ComponentEffect<Self> {
        match self.input_routing {
            InputRouting::Artist => self
                .artist_list
                .handle_list_action(action)
                .map(|this: &mut Self| &mut this.artist_list),
            InputRouting::Song => self
                .album_songs_list
                .handle_list_action(action)
                .map(|this: &mut Self| &mut this.album_songs_list),
        }
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        if self.is_text_handling()
            && self.artist_list.search_popped
            && self.input_routing == InputRouting::Artist
        {
            match action {
                TextEntryAction::Submit => {
                    return self.search();
                }
                // Handled by old handle_text_event_impl.
                //
                // TODO: remove the duplication of responsibilities between this function and
                // handle_text_event_impl.
                TextEntryAction::Left => (),
                TextEntryAction::Right => (),
                TextEntryAction::Backspace => (),
            }
        }
        AsyncTask::new_no_op()
    }
    pub fn handle_toggle_search(&mut self) {
        if self.artist_list.search_popped {
            self.artist_list.close_search();
            self.revert_routing();
        } else {
            self.artist_list.open_search();
            self.change_routing(InputRouting::Artist);
        }
    }
    pub async fn play_song(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_song_idx = self.album_songs_list.get_selected_item();
        if let Some(cur_song) = self.album_songs_list.get_song_from_idx(cur_song_idx) {
            send_or_error(
                &self.callback_tx,
                AppCallback::AddSongsToPlaylistAndPlay(vec![cur_song.clone()]),
            )
            .await;
        }
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub async fn play_songs(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        let song_list = self
            .album_songs_list
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        send_or_error(
            &self.callback_tx,
            AppCallback::AddSongsToPlaylistAndPlay(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub async fn add_songs_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        let song_list = self
            .album_songs_list
            .get_filtered_list_iter()
            .skip(cur_idx)
            .cloned()
            .collect();
        send_or_error(
            &self.callback_tx,
            AppCallback::AddSongsToPlaylist(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub async fn add_song_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        if let Some(cur_song) = self.album_songs_list.get_song_from_idx(cur_idx) {
            send_or_error(
                &self.callback_tx,
                AppCallback::AddSongsToPlaylist(vec![cur_song.clone()]),
            )
            .await;
        }
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub async fn add_album_to_playlist(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        let Some(cur_song) = self.album_songs_list.get_song_from_idx(cur_idx) else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            // Even if list is filtered, still play the whole album.
            .get_list_iter()
            .filter(|song| song.album_id == cur_song.album_id)
            .cloned()
            .collect();
        send_or_error(
            &self.callback_tx,
            AppCallback::AddSongsToPlaylist(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub async fn play_album(&mut self) {
        // Consider how resource intensive this is as it runs in the main thread.
        let cur_idx = self.album_songs_list.get_selected_item();
        let Some(cur_song) = self.album_songs_list.get_song_from_idx(cur_idx) else {
            return;
        };
        let song_list = self
            .album_songs_list
            .list
            // Even if list is filtered, still play the whole album.
            .get_list_iter()
            .filter(|song| song.album_id == cur_song.album_id)
            // XXX: Could instead be inside an Rc.
            .cloned()
            .collect();
        send_or_error(
            &self.callback_tx,
            AppCallback::AddSongsToPlaylistAndPlay(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    pub fn get_songs(&mut self) -> AsyncTask<Self, ArcServer, TaskMetadata> {
        let selected = self.artist_list.get_selected_item();
        self.change_routing(InputRouting::Song);
        self.album_songs_list.list.clear();

        let Some(cur_artist_id) = self
            .artist_list
            .list
            .get(selected)
            .cloned()
            .map(|a| a.browse_id)
        else {
            tracing::warn!("Tried to get item from list with index out of range");
            return AsyncTask::new_no_op();
        };

        let handler = |this: &mut Self, item| match item {
            GetArtistSongsProgressUpdate::Loading => this.handle_song_list_loading(),
            GetArtistSongsProgressUpdate::NoSongsFound => this.handle_no_songs_found(),
            GetArtistSongsProgressUpdate::SearchArtistError => this.handle_search_artist_error(),
            GetArtistSongsProgressUpdate::SongsFound => this.handle_songs_found(),
            GetArtistSongsProgressUpdate::Songs {
                song_list,
                album,
                year,
                artist,
                album_id,
            } => this.handle_append_song_list(song_list, album, album_id, year, artist),
            GetArtistSongsProgressUpdate::AllSongsSent => this.handle_song_list_loaded(),
        };

        AsyncTask::new_stream(
            GetArtistSongs(cur_artist_id),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn search(&mut self) -> ComponentEffect<Self> {
        self.artist_list.close_search();
        let search_query = self.artist_list.search.get_text().to_string();
        self.artist_list.clear_text();

        let handler = |this: &mut Self, results| match results {
            Ok(artists) => {
                this.replace_artist_list(artists);
            }
            Err(e) => {
                error!("Error <{e}> recieved getting artists.");
            }
        };
        AsyncTask::new_future(
            SearchArtists(search_query),
            handler,
            Some(Constraint::new_kill_same_type()),
        )
    }
    pub fn handle_search_artist_error(&mut self) {
        self.album_songs_list.list.state = ListStatus::Error;
    }
    pub fn handle_song_list_loaded(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loaded;
    }
    pub fn handle_song_list_loading(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loading;
    }
    pub fn replace_artist_list(&mut self, artist_list: Vec<SearchResultArtist>) {
        self.artist_list.list = artist_list;
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0);
    }
    pub fn handle_no_songs_found(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loaded;
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<AlbumSong>,
        album: String,
        album_id: AlbumID<'static>,
        year: String,
        artist: String,
    ) {
        self.album_songs_list
            .list
            .append_raw_songs(song_list, album, album_id, year, artist);
        // If sort commands exist, sort the list.
        // Naive - can result in multiple calls to sort every time songs are appended.
        self.album_songs_list.apply_sort_commands();
        self.album_songs_list.list.state = ListStatus::InProgress;
    }
    pub fn handle_songs_found(&mut self) {
        self.album_songs_list.handle_songs_found()
    }
    fn increment_cur_list(&mut self, increment: isize) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_list.increment_list(increment);
            }
            InputRouting::Song => {
                self.album_songs_list.increment_list(increment);
            }
        };
    }
    #[deprecated]
    pub fn revert_routing(&mut self) {
        mem::swap(&mut self.input_routing, &mut self.prev_input_routing);
    }
    // Could be in trait.
    #[deprecated = "Should be in a trait"]
    pub fn change_routing(&mut self, input_routing: InputRouting) {
        self.prev_input_routing = mem::replace(&mut self.input_routing, input_routing);
    }
}
impl Component for Browser {
    type Bkend = ArcServer;
    type Md = TaskMetadata;
}
