use self::draw::draw_browser;
use super::action::{AppAction, TextEntryAction};
use super::{AppCallback, WindowContext};
use crate::app::component::actionhandler::{
    apply_action_mapped, Action, ActionHandler, ComponentEffect, DelegateScrollable,
    DominantKeyRouter, KeyRouter, Scrollable, TextHandler, YoutuiEffect,
};
use crate::app::view::DrawableMut;
use crate::config::keymap::Keymap;
use crate::config::Config;
use artistsearch::search_panel::BrowserArtistsAction;
use artistsearch::songs_panel::BrowserArtistSongsAction;
use artistsearch::ArtistSearchBrowser;
use async_callback_manager::AsyncTask;
use itertools::Either;
use serde::{Deserialize, Serialize};
use shared_components::{BrowserSearchAction, FilterAction, SortAction};
use songsearch::{BrowserSongsAction, SongSearchBrowser};
use std::iter::Iterator;
use tracing::warn;

pub mod artistsearch;
mod draw;
pub mod shared_components;
pub mod songsearch;

#[derive(Default)]
enum BrowserVariant {
    #[default]
    ArtistSearch,
    SongSearch,
}

pub struct Browser {
    variant: BrowserVariant,
    artist_search_browser: ArtistSearchBrowser,
    song_search_browser: SongSearchBrowser,
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
    fn context(&self) -> std::borrow::Cow<str> {
        "Browser".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
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
            BrowserVariant::ArtistSearch => &mut self.artist_search_browser as &mut dyn Scrollable,
            BrowserVariant::SongSearch => &mut self.song_search_browser as &mut dyn Scrollable,
        }
    }
    fn delegate_ref(&self) -> &dyn Scrollable {
        match self.variant {
            BrowserVariant::ArtistSearch => &self.artist_search_browser as &dyn Scrollable,
            BrowserVariant::SongSearch => &self.song_search_browser as &dyn Scrollable,
        }
    }
}
impl ActionHandler<BrowserSearchAction> for Browser {
    fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => apply_action_mapped(self, action, |this: &mut Self| {
                &mut this.artist_search_browser
            }),
            BrowserVariant::SongSearch => apply_action_mapped(self, action, |this: &mut Self| {
                &mut this.song_search_browser
            }),
        }
    }
}
impl ActionHandler<BrowserArtistSongsAction> for Browser {
    fn apply_action(&mut self, action: BrowserArtistSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.artist_search_browser
                })
            }
            BrowserVariant::SongSearch => warn!(
                "Received action {:?} but song artist search browser not active",
                action
            ),
        };
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserArtistsAction> for Browser {
    fn apply_action(&mut self, action: BrowserArtistsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.artist_search_browser
                })
            }
            BrowserVariant::SongSearch => warn!(
                "Received action {:?} but song artist search browser not active",
                action
            ),
        }
        YoutuiEffect::new_no_op()
    }
}
impl ActionHandler<BrowserSongsAction> for Browser {
    fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::SongSearch => {
                return apply_action_mapped(self, action, |this: &mut Self| {
                    &mut this.song_search_browser
                })
            }
            BrowserVariant::ArtistSearch => warn!(
                "Received action {:?} but song search browser not active",
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
                )
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
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.song_search_browser),
        }
    }
}
impl ActionHandler<SortAction> for Browser {
    fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .apply_action(action)
                .into()
                .map(|this: &mut Self| &mut this.song_search_browser),
        }
    }
}
impl TextHandler for Browser {
    fn is_text_handling(&self) -> bool {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.is_text_handling(),
            BrowserVariant::SongSearch => self.song_search_browser.is_text_handling(),
        }
    }
    fn get_text(&self) -> &str {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.get_text(),
            BrowserVariant::SongSearch => self.song_search_browser.get_text(),
        }
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.replace_text(text),
            BrowserVariant::SongSearch => self.song_search_browser.replace_text(text),
        }
    }
    fn clear_text(&mut self) -> bool {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.clear_text(),
            BrowserVariant::SongSearch => self.song_search_browser.clear_text(),
        }
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.artist_search_browser)),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .handle_text_event_impl(event)
                .map(|effect| effect.map(|this: &mut Self| &mut this.song_search_browser)),
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
                BrowserVariant::SongSearch => {
                    Either::Left(self.song_search_browser.get_active_keybinds(config))
                }
                BrowserVariant::ArtistSearch => {
                    Either::Right(self.artist_search_browser.get_active_keybinds(config))
                }
            }
            .chain(std::iter::once(&config.keybinds.browser)),
        )
    }
}
impl DominantKeyRouter<AppAction> for Browser {
    fn dominant_keybinds_active(&self) -> bool {
        match self.variant {
            BrowserVariant::SongSearch => {
                self.song_search_browser.sort.shown || self.song_search_browser.filter.shown
            }
            BrowserVariant::ArtistSearch => {
                self.artist_search_browser.album_songs_panel.sort.shown
                    || self.artist_search_browser.album_songs_panel.filter.shown
            }
        }
    }
    fn get_dominant_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                match self.artist_search_browser.album_songs_panel.route {
                    artistsearch::songs_panel::AlbumSongsInputRouting::List => {
                        Either::Left(std::iter::empty())
                    }
                    artistsearch::songs_panel::AlbumSongsInputRouting::Sort => {
                        Either::Right(std::iter::once(&config.keybinds.sort))
                    }
                    artistsearch::songs_panel::AlbumSongsInputRouting::Filter => {
                        Either::Right(std::iter::once(&config.keybinds.filter))
                    }
                }
            }
            BrowserVariant::SongSearch => match self.song_search_browser.input_routing {
                songsearch::InputRouting::List => Either::Left(std::iter::empty()),
                songsearch::InputRouting::Search => Either::Left(std::iter::empty()),
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
        }
    }
    pub fn left(&mut self) {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.left(),
            BrowserVariant::SongSearch => (),
        }
    }
    pub fn right(&mut self) {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.right(),
            BrowserVariant::SongSearch => (),
        }
    }
    pub fn handle_text_entry_action(&mut self, action: TextEntryAction) -> ComponentEffect<Self> {
        match self.variant {
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .handle_text_entry_action(action)
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .handle_text_entry_action(action)
                .map(|this: &mut Self| &mut this.song_search_browser),
        }
    }
    pub fn handle_toggle_search(&mut self) {
        match self.variant {
            BrowserVariant::ArtistSearch => self.artist_search_browser.handle_toggle_search(),
            BrowserVariant::SongSearch => self.song_search_browser.handle_toggle_search(),
        }
    }
    pub fn handle_change_search_type(&mut self) {
        match self.variant {
            BrowserVariant::ArtistSearch => self.variant = BrowserVariant::SongSearch,
            BrowserVariant::SongSearch => self.variant = BrowserVariant::ArtistSearch,
        }
    }
}

pub fn get_sort_keybinds(config: &Config) -> impl Iterator<Item = &Keymap<AppAction>> + '_ {
    [&config.keybinds.sort, &config.keybinds.list].into_iter()
}

#[cfg(test)]
mod tests {
    use super::artistsearch::songs_panel::BrowserArtistSongsAction;
    use super::Browser;
    use crate::app::component::actionhandler::{ActionHandler, KeyRouter};
    use crate::app::ui::action::AppAction;
    use crate::app::ui::browser::shared_components::BrowserSearchAction;
    use crate::app::ui::browser::BrowserAction;
    use crate::config::keymap::KeyActionTree;
    use crate::config::Config;
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
