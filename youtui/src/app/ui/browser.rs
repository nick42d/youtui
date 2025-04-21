use self::draw::draw_browser;
use super::{
    action::{AppAction, TextEntryAction},
    AppCallback, WindowContext,
};
use crate::config::Config;
use crate::{
    app::{
        component::actionhandler::{
            Action, ActionHandler, Component, ComponentEffect, DelegateScrollable,
            DominantKeyRouter, KeyRouter, Scrollable, TextHandler, YoutuiEffect,
        },
        server::{ArcServer, TaskMetadata},
        view::DrawableMut,
    },
    config::keymap::Keymap,
};
use artistsearch::{
    search_panel::BrowserArtistsAction, songs_panel::BrowserArtistSongsAction, ArtistSearchBrowser,
};
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
    browser_keybinds: Keymap<AppAction>,
    sort_keybings: Keymap<AppAction>,
    filter_keybings: Keymap<AppAction>,
    search_keybinds: Keymap<AppAction>,
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
    async fn apply_action(&mut self, action: BrowserSearchAction) -> impl Into<YoutuiEffect<Self>> {
        // This is not as simple as mapping the action to either type of state, since an
        // action has a component it works on.
        match self.variant {
            BrowserVariant::ArtistSearch => {
                ActionHandler::<BrowserArtistsAction>::apply_action_mapped(
                    self,
                    action,
                    |this: &mut Self| &mut this.artist_search_browser,
                )
                .await
            }
            BrowserVariant::SongSearch => {
                ActionHandler::<BrowserArtistsAction>::apply_action_mapped(
                    self,
                    action,
                    |this: &mut Self| &mut this.song_search_browser,
                )
                .await
            }
        }
    }
}
impl ActionHandler<BrowserArtistSongsAction> for Browser {
    async fn apply_action(
        &mut self,
        action: BrowserArtistSongsAction,
    ) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                return ActionHandler::<BrowserArtistsAction>::apply_action_mapped(
                    self,
                    action,
                    |this: &mut Self| &mut this.artist_search_browser,
                )
                .await
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
    async fn apply_action(
        &mut self,
        action: BrowserArtistsAction,
    ) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                return ActionHandler::<BrowserArtistsAction>::apply_action_mapped(
                    self,
                    action,
                    |this: &mut Self| &mut this.artist_search_browser,
                )
                .await
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
    async fn apply_action(&mut self, action: BrowserSongsAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::SongSearch => {
                return ActionHandler::<BrowserSongsAction>::apply_action_mapped(
                    self,
                    action,
                    |this: &mut Self| &mut this.song_search_browser,
                )
                .await
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
    async fn apply_action(&mut self, action: BrowserAction) -> impl Into<YoutuiEffect<Self>> {
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
    async fn apply_action(&mut self, action: FilterAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .apply_action(action)
                .await
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .apply_action(action)
                .await
                .into()
                .map(|this: &mut Self| &mut this.song_search_browser),
        }
    }
}
impl ActionHandler<SortAction> for Browser {
    async fn apply_action(&mut self, action: SortAction) -> impl Into<YoutuiEffect<Self>> {
        match self.variant {
            BrowserVariant::ArtistSearch => self
                .artist_search_browser
                .apply_action(action)
                .await
                .into()
                .map(|this: &mut Self| &mut this.artist_search_browser),
            BrowserVariant::SongSearch => self
                .song_search_browser
                .apply_action(action)
                .await
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
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        std::iter::once(&self.browser_keybinds)
            .chain(self.artist_search_browser.get_all_keybinds())
            .chain(self.song_search_browser.get_all_keybinds())
            // TODO: Verify if I want to show sort/filter keybinds even when not selected.
            .chain(std::iter::once(&self.search_keybinds))
            .chain(std::iter::once(&self.filter_keybings))
            .chain(std::iter::once(&self.sort_keybings))
    }
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        if self.dominant_keybinds_active() {
            return Either::Left(self.get_dominant_keybinds());
        }
        // Need to handle search keybinds? Filter/search are handled as they are
        // dominant.
        Either::Right(
            match self.variant {
                BrowserVariant::SongSearch => {
                    Either::Left(self.song_search_browser.get_active_keybinds())
                }
                BrowserVariant::ArtistSearch => {
                    Either::Right(self.artist_search_browser.get_active_keybinds())
                }
            }
            .chain(std::iter::once(&self.browser_keybinds)),
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
    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &Keymap<AppAction>> {
        match self.variant {
            BrowserVariant::ArtistSearch => {
                match self.artist_search_browser.album_songs_panel.route {
                    artistsearch::songs_panel::AlbumSongsInputRouting::List => {
                        Either::Left(std::iter::empty())
                    }
                    artistsearch::songs_panel::AlbumSongsInputRouting::Sort => {
                        Either::Right(std::iter::once(&self.sort_keybings))
                    }
                    artistsearch::songs_panel::AlbumSongsInputRouting::Filter => {
                        Either::Right(std::iter::once(&self.filter_keybings))
                    }
                }
            }
            BrowserVariant::SongSearch => match self.song_search_browser.input_routing {
                songsearch::InputRouting::List => Either::Left(std::iter::empty()),
                songsearch::InputRouting::Search => Either::Left(std::iter::empty()),
                songsearch::InputRouting::Filter => {
                    Either::Right(std::iter::once(&self.filter_keybings))
                }
                songsearch::InputRouting::Sort => {
                    Either::Right(std::iter::once(&self.sort_keybings))
                }
            },
        }
    }
}

impl Browser {
    pub fn new(config: &Config) -> Self {
        Self {
            browser_keybinds: config.keybinds.browser.clone(),
            variant: Default::default(),
            artist_search_browser: ArtistSearchBrowser::new(config),
            song_search_browser: SongSearchBrowser::new(config),
            sort_keybings: config.keybinds.sort.clone(),
            filter_keybings: config.keybinds.filter.clone(),
            search_keybinds: config.keybinds.browser_search.clone(),
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

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::{artistsearch::songs_panel::BrowserArtistSongsAction, Browser};
    use crate::{
        app::{
            component::actionhandler::{ActionHandler, KeyRouter},
            ui::{action::AppAction, browser::BrowserAction},
        },
        config::{keymap::KeyActionTree, Config},
        keyaction::KeyActionVisibility,
        keybind::Keybind,
    };

    #[tokio::test]
    async fn toggle_search_opens_popup() {
        let mut b = Browser::new(&Config::default());
        b.apply_action(BrowserArtistSongsAction::Filter).await;
        assert!(b.artist_search_browser.album_songs_panel.filter.shown);
    }
    #[tokio::test]
    async fn artist_songs_panel_has_correct_keybinds() {
        let mut b = Browser::new(&Config::default());
        b.apply_action(BrowserAction::Right).await;
        let actual_kb = b.get_active_keybinds();
        let expected_kb = (
            &Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            &KeyActionTree::new_key_with_visibility(
                AppAction::BrowserArtistSongs(BrowserArtistSongsAction::Filter),
                KeyActionVisibility::Global,
            ),
        );
        let kb_found = actual_kb
            .inspect(|kb| println!("{:#?}", kb))
            .any(|km| km.iter().contains(&expected_kb));
        assert!(kb_found);
    }
}
