use self::draw::draw_browser;
use super::action::{AppAction, TextEntryAction};
use super::{AppCallback, WindowContext};
use crate::app::component::actionhandler::{
    Action, ActionHandler, ComponentEffect, DelegateScrollable, DominantKeyRouter, KeyRouter,
    Scrollable, TextHandler, YoutuiEffect, apply_action_mapped,
};
use crate::app::ui::browser::playlistsearch::PlaylistSearchBrowser;
use crate::app::ui::browser::playlistsearch::search_panel::BrowserPlaylistsAction;
use crate::app::ui::browser::playlistsearch::songs_panel::BrowserPlaylistSongsAction;
use crate::app::view::{DrawableMut, HasTabs};
use crate::config::Config;
use crate::config::keymap::Keymap;
use artistsearch::ArtistSearchBrowser;
use artistsearch::search_panel::BrowserArtistsAction;
use artistsearch::songs_panel::BrowserArtistSongsAction;
use async_callback_manager::AsyncTask;
use itertools::Either;
use serde::{Deserialize, Serialize};
use shared_components::{BrowserSearchAction, FilterAction, SortAction};
use songsearch::{BrowserSongsAction, SongSearchBrowser};
use std::borrow::Cow;
use std::convert::Into;
use std::iter::{IntoIterator, Iterator};
use tracing::warn;

pub mod artistsearch;
mod draw;
pub mod playlistsearch;
pub mod shared_components;
pub mod songsearch;

#[derive(Default, Copy, Clone)]
enum BrowserVariant {
    #[default]
    Artist,
    Song,
    Playlist,
}

pub struct Browser {
    variant: BrowserVariant,
    artist_search_browser: ArtistSearchBrowser,
    song_search_browser: SongSearchBrowser,
    playlist_search_browser: PlaylistSearchBrowser,
}
impl_youtui_component!(Browser);

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    ViewPlaylist,
    Search,
    Left,
    Right,
    ChangeSearchType,
}

impl Action for BrowserAction {
    fn context(&self) -> Cow<'_, str> {
        "Browser".into()
    }
    fn describe(&self) -> Cow<'_, str> {
        match self {
            BrowserAction::ViewPlaylist => "View Playlist",
            BrowserAction::Search => "Toggle Search",
            BrowserAction::Left => "Left",
            BrowserAction::Right => "Right",
            BrowserAction::ChangeSearchType => "Change Search Type",
        }
        .into()
    }
}

impl DelegateScrollable for Browser {
    fn delegate_mut(&mut self) -> &mut dyn Scrollable {
        match self.variant {
            BrowserVariant::Artist => &mut self.artist_search_browser as &mut dyn Scrollable,
            BrowserVariant::Song => &mut self.song_search_browser as &mut dyn Scrollable,
            BrowserVariant::Playlist => &mut self.playlist_search_browser as &mut dyn Scrollable,
        }
    }
    fn delegate_ref(&self) -> &dyn Scrollable {
        match self.variant {
            BrowserVariant::Artist => &self.artist_search_browser as &dyn Scrollable,
            BrowserVariant::Song => &self.song_search_browser as &dyn Scrollable,
            BrowserVariant::Playlist => &self.playlist_search_browser as &dyn Scrollable,
        }
    }
}
impl ActionHandler<BrowserSearchAction> for Browser {
    fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => apply_action_mapped(self, action, |this: &mut Self| {
                &mut this.artist_search_browser
            }),
            BrowserVariant::Song => apply_action_mapped(self, action, |this: &mut Self| {
                &mut this.song_search_browser
            }),
            BrowserVariant::Playlist => apply_action_mapped(self, action, |this: &mut Self| {
                &mut this.playlist_search_browser
            }),
        }
    }
}
impl ActionHandler<BrowserArtistSongsAction> for Browser {
    fn apply_action(&mut self, action: BrowserArtistSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.artist_search_browser
                });
            }
            _ => warn!(
                "Received action {:?} but artist search browser not active",
                action
            ),
        };
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserArtistsAction> for Browser {
    fn apply_action(&mut self, action: BrowserArtistsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.artist_search_browser
                });
            }
            _ => warn!(
                "Received action {:?} but artist search browser not active",
                action
            ),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserSongsAction> for Browser {
    fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Song => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.song_search_browser
                });
            }
            _ => warn!(
                "Received action {:?} but song search browser not active",
                action
            ),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserPlaylistsAction> for Browser {
    fn apply_action(&mut self, action: BrowserPlaylistsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Playlist => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.playlist_search_browser
                });
            }
            _ => warn!(
                "Received action {:?} but playlist search browser not active",
                action
            ),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserPlaylistSongsAction> for Browser {
    fn apply_action(
        &mut self,
        action: BrowserPlaylistSongsAction,
    ) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Playlist => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.playlist_search_browser
                });
            }
            _ => warn!(
                "Received action {:?} but playlist search browser not active",
                action
            ),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserAction> for Browser {
    fn apply_action(&mut self, action: BrowserAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            BrowserAction::Left => self.left(),
            BrowserAction::Right => self.right(),
            BrowserAction::ViewPlaylist => {
                return (
                    AsyncTask::new_no_op(),
                    Some(AppCallback::ChangeContext(WindowContext::Playlist)),
                );
            }
            BrowserAction::Search => self.handle_toggle_search(),
            BrowserAction::ChangeSearchType => self.handle_change_search_type(),
        }
        (AsyncTask::new_no_op(), None)
    }
}
impl ActionHandler<FilterAction> for Browser {
    fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => self
                .artist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::Song => self
                .song_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.song_search_browser),
            BrowserVariant::Playlist => self
                .playlist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.playlist_search_browser),
        }
    }
}
impl ActionHandler<SortAction> for Browser {
    fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => self
                .artist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::Song => self
                .song_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.song_search_browser),
            BrowserVariant::Playlist => self
                .playlist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.playlist_search_browser),
        }
    }
}
impl TextHandler for Browser {
    fn is_text_handling(&self) -> bool {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.is_text_handling(),
            BrowserVariant::Song => self.song_search_browser.is_text_handling(),
            BrowserVariant::Playlist => self.playlist_search_browser.is_text_handling(),
        }
    }
    fn get_text(&self) -> std::option::Option<&str> {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.get_text(),
            BrowserVariant::Song => self.song_search_browser.get_text(),
            BrowserVariant::Playlist => self.playlist_search_browser.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.replace_text(text),
            BrowserVariant::Song => self.song_search_browser.replace_text(text),
            BrowserVariant::Playlist => self.playlist_search_browser.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.clear_text(),
            BrowserVariant::Song => self.song_search_browser.clear_text(),
            BrowserVariant::Playlist => self.playlist_search_browser.clear_text(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.variant {
            BrowserVariant::Artist => self
                .artist_search_browser
                .handle_text_event_impl(event)
                .map(|effect| {
                    effect.map_frontend(|this: &mut Self| &mut this.artist_search_browser)
                }),
            BrowserVariant::Song => self
                .song_search_browser
                .handle_text_event_impl(event)
                .map(|effect| effect.map_frontend(|this: &mut Self| &mut this.song_search_browser)),
            BrowserVariant::Playlist => self
                .playlist_search_browser
                .handle_text_event_impl(event)
                .map(|effect| {
                    effect.map_frontend(|this: &mut Self| &mut this.playlist_search_browser)
                }),
        }
    }
}

impl DrawableMut for Browser {
    fn draw_mut_chunk(
        &mut self,
        f: &mut ratatui::Frame,
        chunk: ratatui::prelude::Rect,
        selected: bool,
        cur_tick: u64,
    ) {
        draw_browser(f, self, chunk, selected, cur_tick);
    }
}
impl HasTabs for Browser {
    fn tabs_block_title(&'_ self) -> Cow<'_, str> {
        "Browser".into()
    }
    fn tab_items(&'_ self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> + '_ {
        ["Artists", "Songs", "Playlists"]
    }
    fn selected_tab_idx(&self) -> usize {
        // Cast won't panic - rust compiler allows casting enums without data to
        // integers, and won't compile if we later add data to the enum.
        self.variant as usize
    }
}
impl KeyRouter<AppAction> for Browser {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        // TODO: Verify if I want to show sort/filter keybinds even when not selected.
        [
            &config.keybinds.browser,
            &config.keybinds.browser_search,
            &config.keybinds.filter,
            &config.keybinds.sort,
        ]
        .into_iter()
        .chain(self.artist_search_browser.get_all_keybinds(config))
        .chain(self.song_search_browser.get_all_keybinds(config))
        .chain(self.playlist_search_browser.get_all_keybinds(config))
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        if self.dominant_keybinds_active() {
            return Either::Left(self.get_dominant_keybinds(config));
        }
        // Need to handle search keybinds? Filter/search are handled as they are
        // dominant.
        Either::Right(
            match self.variant {
                BrowserVariant::Song => Either::Left(Either::Left(
                    self.song_search_browser.get_active_keybinds(config),
                )),
                BrowserVariant::Artist => Either::Left(Either::Right(
                    self.artist_search_browser.get_active_keybinds(config),
                )),
                BrowserVariant::Playlist => {
                    Either::Right(self.playlist_search_browser.get_active_keybinds(config))
                }
            }
            .chain(std::iter::once(&config.keybinds.browser)),
        )
    }
}
impl DominantKeyRouter<AppAction> for Browser {
    fn dominant_keybinds_active(&self) -> bool {
        match self.variant {
            BrowserVariant::Song => {
                self.song_search_browser.sort.shown || self.song_search_browser.filter.shown
            }
            BrowserVariant::Artist => {
                self.artist_search_browser.album_songs_panel.sort.shown
                    || self.artist_search_browser.album_songs_panel.filter.shown
            }
            BrowserVariant::Playlist => {
                self.playlist_search_browser.playlist_songs_panel.sort.shown
                    || self
                        .playlist_search_browser
                        .playlist_songs_panel
                        .filter
                        .shown
            }
        }
    }
    fn get_dominant_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.variant {
            BrowserVariant::Artist => match self.artist_search_browser.album_songs_panel.route {
                artistsearch::songs_panel::AlbumSongsInputRouting::List => {
                    Either::Left(std::iter::empty())
                }
                artistsearch::songs_panel::AlbumSongsInputRouting::Sort => {
                    Either::Right(std::iter::once(&config.keybinds.sort))
                }
                artistsearch::songs_panel::AlbumSongsInputRouting::Filter => {
                    Either::Right(std::iter::once(&config.keybinds.filter))
                }
            },
            BrowserVariant::Playlist => {
                match self.playlist_search_browser.playlist_songs_panel.route {
                    playlistsearch::songs_panel::PlaylistSongsInputRouting::List => {
                        Either::Left(std::iter::empty())
                    }
                    playlistsearch::songs_panel::PlaylistSongsInputRouting::Sort => {
                        Either::Right(std::iter::once(&config.keybinds.sort))
                    }
                    playlistsearch::songs_panel::PlaylistSongsInputRouting::Filter => {
                        Either::Right(std::iter::once(&config.keybinds.filter))
                    }
                }
            }
            BrowserVariant::Song => match self.song_search_browser.input_routing {
                songsearch::InputRouting::List | songsearch::InputRouting::Search => {
                    Either::Left(std::iter::empty())
                }
                songsearch::InputRouting::Filter => {
                    Either::Right(std::iter::once(&config.keybinds.filter))
                }
                songsearch::InputRouting::Sort => {
                    Either::Right(std::iter::once(&config.keybinds.sort))
                }
            },
        }
    }
}

impl Browser {
    pub fn new() -> Self {
        Self {
            variant: Default::default(),
            artist_search_browser: ArtistSearchBrowser::new(),
            song_search_browser: SongSearchBrowser::new(),
            playlist_search_browser: PlaylistSearchBrowser::new(),
        }
    }
    pub fn left(&mut self) {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.left(),
            BrowserVariant::Playlist => self.playlist_search_browser.left(),
            BrowserVariant::Song => (),
        }
    }
    pub fn right(&mut self) {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.right(),
            BrowserVariant::Playlist => self.playlist_search_browser.right(),
            BrowserVariant::Song => (),
        }
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        match self.variant {
            BrowserVariant::Artist => self
                .artist_search_browser
                .handle_text_entry_action(action)
                .map_frontend(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::Song => self
                .song_search_browser
                .handle_text_entry_action(action)
                .map_frontend(|this: &mut Self| &mut this.song_search_browser),
            BrowserVariant::Playlist => self
                .playlist_search_browser
                .handle_text_entry_action(action)
                .map_frontend(|this: &mut Self| &mut this.playlist_search_browser),
        }
    }
    pub fn handle_toggle_search(&mut self) {
        match self.variant {
            BrowserVariant::Artist => self.artist_search_browser.handle_toggle_search(),
            BrowserVariant::Song => self.song_search_browser.handle_toggle_search(),
            BrowserVariant::Playlist => self.playlist_search_browser.handle_toggle_search(),
        }
    }
    pub fn handle_change_search_type(&mut self) {
        match self.variant {
            BrowserVariant::Artist => self.variant = BrowserVariant::Song,
            BrowserVariant::Song => self.variant = BrowserVariant::Playlist,
            BrowserVariant::Playlist => self.variant = BrowserVariant::Artist,
        }
    }
}

pub fn get_sort_keybinds(config: &Config) -> impl Iterator<Item = &Keymap<AppAction>> + '_ {
    [&config.keybinds.sort, &config.keybinds.list].into_iter()
}

#[cfg(test)]
mod tests {
    use super::Browser;
    use super::artistsearch::songs_panel::BrowserArtistSongsAction;
    use crate::app::component::actionhandler::{ActionHandler, KeyRouter};
    use crate::app::ui::action::AppAction;
    use crate::app::ui::browser::BrowserAction;
    use crate::app::ui::browser::shared_components::BrowserSearchAction;
    use crate::config::Config;
    use crate::config::keymap::KeyActionTree;
    use crate::keyaction::KeyActionVisibility;
    use crate::keybind::Keybind;
    use itertools::Itertools;
    #[tokio::test]
    async fn toggle_search_opens_popup() {
        let mut b = Browser::new();
        b.apply_action(BrowserArtistSongsAction::Filter);
        assert!(b.artist_search_browser.album_songs_panel.filter.shown);
    }
    #[tokio::test]
    async fn artist_search_panel_search_suggestions_has_correct_keybinds() {
        let cfg = Config::default();
        let b = Browser::new();
        let actual_kb = b.get_active_keybinds(&cfg);
        let expected_kb = (
            &Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            &KeyActionTree::new_key(AppAction::BrowserSearch(
                BrowserSearchAction::NextSearchSuggestion,
            )),
        );
        let kb_found = actual_kb
            .inspect(|kb| println!("{kb:#?}"))
            .any(|km| km.iter().contains(&expected_kb));
        assert!(kb_found);
    }
    #[tokio::test]
    async fn songs_search_panel_search_suggestions_has_correct_keybinds() {
        let cfg = Config::default();
        let mut b = Browser::new();
        b.apply_action(BrowserAction::ChangeSearchType);
        let actual_kb = b.get_active_keybinds(&cfg);
        let expected_kb = (
            &Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            &KeyActionTree::new_key(AppAction::BrowserSearch(
                BrowserSearchAction::NextSearchSuggestion,
            )),
        );
        let kb_found = actual_kb
            .inspect(|kb| println!("{kb:#?}"))
            .any(|km| km.iter().contains(&expected_kb));
        assert!(kb_found);
    }
    #[tokio::test]
    async fn artist_songs_panel_has_correct_keybinds() {
        let cfg = Config::default();
        let mut b = Browser::new();
        b.apply_action(BrowserAction::Right);
        let actual_kb = b.get_active_keybinds(&cfg);
        let expected_kb = (
            &Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            &KeyActionTree::new_key_with_visibility(
                AppAction::BrowserArtistSongs(BrowserArtistSongsAction::Filter),
                KeyActionVisibility::Global,
            ),
        );
        let kb_found = actual_kb
            .inspect(|kb| println!("{kb:#?}"))
            .any(|km| km.iter().contains(&expected_kb));
        assert!(kb_found);
    }
}
