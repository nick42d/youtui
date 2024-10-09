use self::{
    artistalbums::{
        albumsongs::{AlbumSongsPanel, ArtistSongsAction},
        artistsearch::{ArtistAction, ArtistSearchPanel},
    },
    draw::draw_browser,
};
use super::{AppCallback, WindowContext};
use crate::app::{
    component::actionhandler::{
        Action, ActionHandler, DominantKeyRouter, KeyRouter, Suggestable, TextHandler,
    },
    server::Server,
    structures::{ListStatus, SongListComponent},
    view::{DrawableMut, Scrollable},
    YoutuiMutableState, CALLBACK_CHANNEL_SIZE,
};
use crate::{app::keycommand::KeyCommand, core::send_or_error};
use async_callback_manager::{AsyncCallbackManager, AsyncCallbackSender};
use crossterm::event::KeyCode;
use std::{borrow::Cow, mem};
use tokio::sync::mpsc;
use tracing::error;
use ytmapi_rs::{
    common::SearchSuggestion,
    parse::{AlbumSong, SearchResultArtist},
};

const PAGE_KEY_LINES: isize = 10;

mod artistalbums;
mod draw;

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserAction {
    ViewPlaylist,
    ToggleSearch,
    Left,
    Right,
    Artist(ArtistAction),
    ArtistSongs(ArtistSongsAction),
}

#[derive(PartialEq)]
pub enum InputRouting {
    Artist,
    Song,
}

pub struct Browser {
    callback_tx: mpsc::Sender<AppCallback>,
    pub input_routing: InputRouting,
    pub prev_input_routing: InputRouting,
    pub artist_list: ArtistSearchPanel,
    pub album_songs_list: AlbumSongsPanel,
    keybinds: Vec<KeyCommand<BrowserAction>>,
    async_tx: AsyncCallbackSender<Server, Self>,
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
impl Action for BrowserAction {
    fn context(&self) -> Cow<str> {
        let context = "Browser";
        match self {
            Self::Artist(a) => format!("{context}->{}", a.context()).into(),
            Self::ArtistSongs(a) => format!("{context}->{}", a.context()).into(),
            _ => context.into(),
        }
    }
    fn describe(&self) -> Cow<str> {
        match self {
            Self::Left => "Left".into(),
            Self::Right => "Right".into(),
            Self::ViewPlaylist => "View Playlist".into(),
            Self::ToggleSearch => "Toggle Search".into(),
            Self::Artist(x) => x.describe(),
            Self::ArtistSongs(x) => x.describe(),
        }
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
    fn push_text(&mut self, c: char) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_list.push_text(c);
                // Should be on artist_list instead?
                self.fetch_search_suggestions();
            }
            InputRouting::Song => self.album_songs_list.push_text(c),
        }
    }
    fn pop_text(&mut self) {
        match self.input_routing {
            InputRouting::Artist => {
                self.artist_list.pop_text();
                // Should be on artist_list instead?
                self.fetch_search_suggestions();
            }
            InputRouting::Song => self.album_songs_list.pop_text(),
        }
    }
    fn is_text_handling(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.is_text_handling(),
            InputRouting::Song => self.album_songs_list.is_text_handling(),
        }
    }
    fn take_text(&mut self) -> String {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.take_text(),
            InputRouting::Song => self.album_songs_list.take_text(),
        }
    }
    fn replace_text(&mut self, text: String) {
        match self.input_routing {
            InputRouting::Artist => self.artist_list.replace_text(text),
            InputRouting::Song => self.album_songs_list.replace_text(text),
        }
    }
}

impl DrawableMut for Browser {
    fn draw_mut_chunk(
        &self,
        f: &mut ratatui::Frame,
        chunk: ratatui::prelude::Rect,
        mutable_state: &mut YoutuiMutableState,
        selected: bool,
    ) {
        draw_browser(
            f,
            self,
            chunk,
            &mut mutable_state.browser_artists_state,
            &mut mutable_state.browser_album_songs_state,
            selected,
        );
    }
}
impl KeyRouter<BrowserAction> for Browser {
    fn get_all_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        Box::new(
            self.keybinds
                .iter()
                .chain(self.artist_list.get_all_keybinds())
                .chain(self.album_songs_list.get_all_keybinds()),
        )
    }
    fn get_routed_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<BrowserAction>> + 'a> {
        let additional_binds = match self.input_routing {
            InputRouting::Song => self.album_songs_list.get_routed_keybinds(),
            InputRouting::Artist => self.artist_list.get_routed_keybinds(),
        };
        // TODO: Better implementation
        if self.album_songs_list.dominant_keybinds_active()
            || self.album_songs_list.dominant_keybinds_active()
        {
            additional_binds
        } else {
            Box::new(self.keybinds.iter().chain(additional_binds))
        }
    }
}
impl ActionHandler<ArtistAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistAction) {
        match action {
            ArtistAction::DisplayAlbums => self.get_songs().await,
            ArtistAction::Search => self.search().await,
            ArtistAction::Up => self.artist_list.increment_list(-1),
            ArtistAction::Down => self.artist_list.increment_list(1),
            ArtistAction::PageUp => self.artist_list.increment_list(-10),
            ArtistAction::PageDown => self.artist_list.increment_list(10),
            ArtistAction::PrevSearchSuggestion => self.artist_list.search.increment_list(-1),
            ArtistAction::NextSearchSuggestion => self.artist_list.search.increment_list(1),
        }
    }
}
impl ActionHandler<ArtistSongsAction> for Browser {
    async fn handle_action(&mut self, action: &ArtistSongsAction) {
        match action {
            ArtistSongsAction::PlayAlbum => self.play_album().await,
            ArtistSongsAction::PlaySong => self.play_song().await,
            ArtistSongsAction::PlaySongs => self.play_songs().await,
            ArtistSongsAction::AddAlbumToPlaylist => self.add_album_to_playlist().await,
            ArtistSongsAction::AddSongToPlaylist => self.add_song_to_playlist().await,
            ArtistSongsAction::AddSongsToPlaylist => self.add_songs_to_playlist().await,
            ArtistSongsAction::Up => self.album_songs_list.increment_list(-1),
            ArtistSongsAction::Down => self.album_songs_list.increment_list(1),
            ArtistSongsAction::PageUp => self.album_songs_list.increment_list(-PAGE_KEY_LINES),
            ArtistSongsAction::PageDown => self.album_songs_list.increment_list(PAGE_KEY_LINES),
            ArtistSongsAction::PopSort => self.album_songs_list.handle_pop_sort(),
            ArtistSongsAction::CloseSort => self.album_songs_list.close_sort(),
            ArtistSongsAction::ClearSort => self.album_songs_list.handle_clear_sort(),
            ArtistSongsAction::SortUp => self.album_songs_list.handle_sort_up(),
            ArtistSongsAction::SortDown => self.album_songs_list.handle_sort_down(),
            ArtistSongsAction::SortSelectedAsc => self.album_songs_list.handle_sort_cur_asc(),
            ArtistSongsAction::SortSelectedDesc => self.album_songs_list.handle_sort_cur_desc(),
            ArtistSongsAction::ToggleFilter => self.album_songs_list.toggle_filter(),
            ArtistSongsAction::ApplyFilter => self.album_songs_list.apply_filter(),
            ArtistSongsAction::ClearFilter => self.album_songs_list.clear_filter(),
        }
    }
}
impl ActionHandler<BrowserAction> for Browser {
    async fn handle_action(&mut self, action: &BrowserAction) {
        match action {
            BrowserAction::ArtistSongs(a) => self.handle_action(a).await,
            BrowserAction::Artist(a) => self.handle_action(a).await,
            BrowserAction::Left => self.left(),
            BrowserAction::Right => self.right(),
            BrowserAction::ViewPlaylist => {
                send_or_error(
                    &self.callback_tx,
                    AppCallback::ChangeContext(WindowContext::Playlist),
                )
                .await
            }
            BrowserAction::ToggleSearch => self.handle_toggle_search(),
        }
    }
}

impl DominantKeyRouter for Browser {
    fn dominant_keybinds_active(&self) -> bool {
        match self.input_routing {
            InputRouting::Artist => false,
            InputRouting::Song => self.album_songs_list.dominant_keybinds_active(),
        }
    }
}

impl Browser {
    pub fn new(
        callback_manager: &mut AsyncCallbackManager<Server>,
        ui_tx: mpsc::Sender<AppCallback>,
    ) -> Self {
        Self {
            callback_tx: ui_tx,
            artist_list: ArtistSearchPanel::new(),
            album_songs_list: AlbumSongsPanel::new(),
            input_routing: InputRouting::Artist,
            prev_input_routing: InputRouting::Artist,
            keybinds: browser_keybinds(),
            async_tx: callback_manager.new_sender(CALLBACK_CHANNEL_SIZE),
        }
    }
    pub async fn async_update(&mut self) {
        // TODO: Size
        self.async_tx.get_next_mutations(10).await.apply(self)
    }
    fn left(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.left();
    }
    fn right(&mut self) {
        // Doesn't consider previous routing.
        self.input_routing = self.input_routing.right();
    }
    fn handle_toggle_search(&mut self) {
        if self.artist_list.search_popped {
            self.artist_list.close_search();
            self.revert_routing();
        } else {
            self.artist_list.open_search();
            self.change_routing(InputRouting::Artist);
        }
    }
    // Ask the UI for search suggestions for the current query
    // XXX: Currently has race conditions - if list is cleared response will arrive
    // afterwards. Proposal: When recieving a message from the app validate
    // against query string.
    fn fetch_search_suggestions(&mut self) {
        // No need to fetch search suggestions if contents is empty.
        if self.artist_list.search.search_contents.is_empty() {
            self.artist_list.search.search_suggestions.clear();
            return;
        }
        if let Err(e) = self.callback_tx.try_send(AppCallback::GetSearchSuggestions(
            self.artist_list.search.search_contents.clone(),
        )) {
            error!("Error <{e}> recieved sending message")
        };
    }
    async fn play_song(&mut self) {
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
    async fn play_songs(&mut self) {
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
    async fn add_songs_to_playlist(&mut self) {
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
    async fn add_song_to_playlist(&mut self) {
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
    async fn add_album_to_playlist(&mut self) {
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
            .filter(|song| song.get_album() == cur_song.get_album())
            .cloned()
            .collect();
        send_or_error(
            &self.callback_tx,
            AppCallback::AddSongsToPlaylist(song_list),
        )
        .await;
        // XXX: Do we want to indicate that song has been added to playlist?
    }
    async fn play_album(&mut self) {
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
            .filter(|song| song.get_album() == cur_song.get_album())
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
    async fn get_songs(&mut self) {
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
            return;
        };
        send_or_error(
            &self.callback_tx,
            AppCallback::GetArtistSongs(cur_artist_id),
        )
        .await;
        tracing::info!("Sent request to UI to get songs");
    }
    async fn search(&mut self) {
        self.artist_list.close_search();
        let search_query = self.artist_list.search.take_text();
        send_or_error(&self.callback_tx, AppCallback::SearchArtist(search_query)).await;
        tracing::info!("Sent request to UI to search");
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
    pub async fn handle_replace_artist_list(&mut self, artist_list: Vec<SearchResultArtist>) {
        self.artist_list.list = artist_list;
        // XXX: What to do if position in list was greater than new list length?
        // Handled by this function?
        self.increment_cur_list(0);
    }
    pub fn handle_replace_search_suggestions(
        &mut self,
        search_suggestions: Vec<SearchSuggestion>,
        search: String,
    ) {
        if self.artist_list.search.search_contents == search {
            self.artist_list.search.search_suggestions = search_suggestions;
            self.artist_list.search.suggestions_cur = None;
        }
    }
    pub fn handle_no_songs_found(&mut self) {
        self.album_songs_list.list.state = ListStatus::Loaded;
    }
    pub fn handle_append_song_list(
        &mut self,
        song_list: Vec<AlbumSong>,
        album: String,
        year: String,
        artist: String,
    ) {
        self.album_songs_list
            .list
            .append_raw_songs(song_list, album, year, artist);
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

fn browser_keybinds() -> Vec<KeyCommand<BrowserAction>> {
    vec![
        KeyCommand::new_global_from_code(KeyCode::F(5), BrowserAction::ViewPlaylist),
        KeyCommand::new_global_from_code(KeyCode::F(2), BrowserAction::ToggleSearch),
        KeyCommand::new_from_code(KeyCode::Left, BrowserAction::Left),
        KeyCommand::new_from_code(KeyCode::Right, BrowserAction::Right),
    ]
}
