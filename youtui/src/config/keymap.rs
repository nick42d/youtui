use crate::app::component::actionhandler::Action;
use crate::app::ui::action::{AppAction, HelpAction, ListAction, TextEntryAction};
use crate::app::ui::browser::BrowserAction;
use crate::app::ui::browser::artistsearch::search_panel::BrowserArtistsAction;
use crate::app::ui::browser::artistsearch::songs_panel::BrowserArtistSongsAction;
use crate::app::ui::browser::playlistsearch::search_panel::BrowserPlaylistsAction;
use crate::app::ui::browser::playlistsearch::songs_panel::BrowserPlaylistSongsAction;
use crate::app::ui::browser::shared_components::{BrowserSearchAction, FilterAction, SortAction};
use crate::app::ui::browser::songsearch::BrowserSongsAction;
use crate::app::ui::logger::LoggerAction;
use crate::app::ui::playlist::PlaylistAction::{self, ViewBrowser};
use crate::keyaction::{KeyAction, KeyActionVisibility};
use crate::keybind::Keybind;
use anyhow::{Context, Error, Result};
use crossterm::event::KeyModifiers;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::convert::Infallible;
use std::str::FromStr;

/// Convenience type alias
pub type Keymap<A> = BTreeMap<Keybind, KeyActionTree<A>>;

/// Merge `other` into `this` leaving `this` empty and returning the merged
/// keymap. This recurively handles modes, merging them also.
fn merge_keymaps<A: Action>(this: &mut Keymap<A>, other: Keymap<A>) {
    for (other_key, other_tree) in other {
        let entry = this.entry(other_key);
        match entry {
            Entry::Vacant(e) => {
                e.insert(other_tree);
            }
            Entry::Occupied(mut e) => {
                let this_tree = e.get_mut();
                this_tree.merge(other_tree);
            }
        }
    }
}
/// If self is a key with action `action`, return None.
/// If self is a mode, remove any actions `action` from it, and if there are
/// none left, also return None.
fn remove_action_from_keymap<A: Action + PartialEq>(this: &mut Keymap<A>, action: &A) {
    this.retain(|_, v| match v {
        KeyActionTree::Key(ka) => &ka.action != action,
        KeyActionTree::Mode { keys, .. } => {
            remove_action_from_keymap(keys, action);
            !keys.is_empty()
        }
    })
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyStringTree {
    #[serde(deserialize_with = "crate::core::string_or_struct")]
    Key(KeyAction<String>),
    Mode(BTreeMap<Keybind, KeyStringTree>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyActionTree<A> {
    Key(KeyAction<A>),
    Mode {
        name: Option<String>,
        keys: Keymap<A>,
    },
}

#[derive(Debug, PartialEq)]
pub struct YoutuiKeymap {
    pub global: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub playlist: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_artists: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_playlists: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_search: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_songs: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_artist_songs: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_playlist_songs: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub help: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub sort: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub filter: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub text_entry: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub list: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub log: BTreeMap<Keybind, KeyActionTree<AppAction>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct YoutuiKeymapIR {
    pub global: BTreeMap<Keybind, KeyStringTree>,
    pub playlist: BTreeMap<Keybind, KeyStringTree>,
    pub browser: BTreeMap<Keybind, KeyStringTree>,
    pub browser_artists: BTreeMap<Keybind, KeyStringTree>,
    pub browser_playlists: BTreeMap<Keybind, KeyStringTree>,
    pub browser_search: BTreeMap<Keybind, KeyStringTree>,
    pub browser_songs: BTreeMap<Keybind, KeyStringTree>,
    pub browser_artist_songs: BTreeMap<Keybind, KeyStringTree>,
    pub browser_playlist_songs: BTreeMap<Keybind, KeyStringTree>,
    pub help: BTreeMap<Keybind, KeyStringTree>,
    pub sort: BTreeMap<Keybind, KeyStringTree>,
    pub filter: BTreeMap<Keybind, KeyStringTree>,
    pub text_entry: BTreeMap<Keybind, KeyStringTree>,
    pub list: BTreeMap<Keybind, KeyStringTree>,
    pub log: BTreeMap<Keybind, KeyStringTree>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
// TODO: Mode visibility
pub struct YoutuiModeNamesIR {
    global: BTreeMap<Keybind, ModeNameEnum>,
    playlist: BTreeMap<Keybind, ModeNameEnum>,
    browser: BTreeMap<Keybind, ModeNameEnum>,
    browser_artists: BTreeMap<Keybind, ModeNameEnum>,
    browser_playlists: BTreeMap<Keybind, ModeNameEnum>,
    browser_search: BTreeMap<Keybind, ModeNameEnum>,
    browser_songs: BTreeMap<Keybind, ModeNameEnum>,
    browser_artist_songs: BTreeMap<Keybind, ModeNameEnum>,
    browser_playlist_songs: BTreeMap<Keybind, ModeNameEnum>,
    help: BTreeMap<Keybind, ModeNameEnum>,
    sort: BTreeMap<Keybind, ModeNameEnum>,
    filter: BTreeMap<Keybind, ModeNameEnum>,
    text_entry: BTreeMap<Keybind, ModeNameEnum>,
    list: BTreeMap<Keybind, ModeNameEnum>,
    log: BTreeMap<Keybind, ModeNameEnum>,
}

impl Default for YoutuiKeymap {
    fn default() -> Self {
        Self {
            global: default_global_keybinds(),
            playlist: default_playlist_keybinds(),
            browser: default_browser_keybinds(),
            browser_artists: default_browser_artists_keybinds(),
            browser_search: default_browser_search_keybinds(),
            browser_songs: default_browser_songs_keybinds(),
            browser_artist_songs: default_browser_artist_songs_keybinds(),
            help: default_help_keybinds(),
            sort: default_sort_keybinds(),
            filter: default_filter_keybinds(),
            text_entry: default_text_entry_keybinds(),
            list: default_list_keybinds(),
            log: default_log_keybinds(),
            browser_playlists: default_browser_playlists_keybinds(),
            browser_playlist_songs: default_browser_playlist_songs_keybinds(),
        }
    }
}

impl YoutuiKeymap {
    pub fn try_from_stringy(keys: YoutuiKeymapIR, mode_names: YoutuiModeNamesIR) -> Result<Self> {
        let YoutuiKeymapIR {
            global,
            playlist,
            browser,
            browser_artists,
            browser_search,
            browser_songs,
            help,
            sort,
            filter,
            text_entry,
            list,
            log,
            browser_artist_songs,
            browser_playlists,
            browser_playlist_songs,
        } = keys;
        let YoutuiModeNamesIR {
            global: mut global_mode_names,
            playlist: mut playlist_mode_names,
            browser: mut browser_mode_names,
            browser_artists: mut browser_artists_mode_names,
            browser_search: mut browser_search_mode_names,
            browser_songs: mut browser_songs_mode_names,
            help: mut help_mode_names,
            sort: mut sort_mode_names,
            filter: mut filter_mode_names,
            text_entry: mut text_entry_mode_names,
            list: mut list_mode_names,
            log: mut log_mode_names,
            browser_artist_songs: mut browser_artist_songs_mode_names,
            browser_playlists: mut browser_playlists_mode_names,
            browser_playlist_songs: mut browser_playlist_songs_mode_names,
        } = mode_names;

        let global = global
            .into_iter()
            .map(move |(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut global_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Global keybinds parse failed")?;
        let playlist = playlist
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut playlist_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Playlist keybinds parse failed")?;
        let browser = browser
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser keybinds parse failed")?;
        let browser_artists = browser_artists
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_artists_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser artists keybinds parse failed")?;
        let browser_playlists = browser_playlists
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_playlists_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser playlists keybinds parse failed")?;
        let browser_search = browser_search
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_search_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser search keybinds parse failed")?;
        let browser_songs = browser_songs
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_songs_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser songs keybinds parse failed")?;
        let browser_artist_songs = browser_artist_songs
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_artist_songs_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser artist songs keybinds parse failed")?;
        let browser_playlist_songs = browser_playlist_songs
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_playlist_songs_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser playlist songs keybinds parse failed")?;
        let text_entry = text_entry
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut text_entry_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Text entry keybinds parse failed")?;
        let help = help
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut help_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Help keybinds parse failed")?;
        let sort = sort
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut sort_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Sort keybinds parse failed")?;
        let filter = filter
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut filter_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Filter keybinds parse failed")?;
        let list = list
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut list_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("List keybinds parse failed")?;
        let log = log
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut log_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Log keybinds parse failed")?;
        let mut keymap = YoutuiKeymap::default();
        merge_keymaps(&mut keymap.global, global);
        merge_keymaps(&mut keymap.playlist, playlist);
        merge_keymaps(&mut keymap.browser, browser);
        merge_keymaps(&mut keymap.browser_artists, browser_artists);
        merge_keymaps(&mut keymap.browser_playlists, browser_playlists);
        merge_keymaps(&mut keymap.browser_search, browser_search);
        merge_keymaps(&mut keymap.browser_songs, browser_songs);
        merge_keymaps(&mut keymap.browser_artist_songs, browser_artist_songs);
        merge_keymaps(&mut keymap.browser_playlist_songs, browser_playlist_songs);
        merge_keymaps(&mut keymap.text_entry, text_entry);
        merge_keymaps(&mut keymap.help, help);
        merge_keymaps(&mut keymap.sort, sort);
        merge_keymaps(&mut keymap.filter, filter);
        merge_keymaps(&mut keymap.list, list);
        merge_keymaps(&mut keymap.log, log);
        remove_action_from_keymap(&mut keymap.global, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.playlist, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_artists, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_playlists, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_search, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_songs, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_artist_songs, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.browser_playlist_songs, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.text_entry, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.help, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.sort, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.filter, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.list, &AppAction::NoOp);
        remove_action_from_keymap(&mut keymap.log, &AppAction::NoOp);
        Ok(keymap)
    }
    #[cfg(test)]
    /// The regular try_from_stringy function merges the IR with the default
    /// config - ie it's additive to the default keybinds. This version
    /// replaces the default config
    pub fn try_from_stringy_exact(
        keys: YoutuiKeymapIR,
        mode_names: YoutuiModeNamesIR,
    ) -> Result<Self> {
        let YoutuiKeymapIR {
            global,
            playlist,
            browser,
            browser_artists,
            browser_search,
            browser_songs,
            help,
            sort,
            filter,
            text_entry,
            list,
            log,
            browser_artist_songs,
            browser_playlists,
            browser_playlist_songs,
        } = keys;
        let YoutuiModeNamesIR {
            global: mut global_mode_names,
            playlist: mut playlist_mode_names,
            browser: mut browser_mode_names,
            browser_artists: mut browser_artists_mode_names,
            browser_search: mut browser_search_mode_names,
            browser_songs: mut browser_songs_mode_names,
            help: mut help_mode_names,
            sort: mut sort_mode_names,
            filter: mut filter_mode_names,
            text_entry: mut text_entry_mode_names,
            list: mut list_mode_names,
            log: mut log_mode_names,
            browser_artist_songs: mut browser_artist_songs_mode_names,
            browser_playlists: mut browser_playlists_mode_names,
            browser_playlist_songs: mut browser_playlist_songs_mode_names,
        } = mode_names;

        let global = global
            .into_iter()
            .map(move |(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut global_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Global keybinds parse failed")?;
        let playlist = playlist
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut playlist_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Playlist keybinds parse failed")?;
        let browser = browser
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser keybinds parse failed")?;
        let browser_artists = browser_artists
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_artists_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser artists keybinds parse failed")?;
        let browser_playlists = browser_playlists
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_playlists_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser playlists keybinds parse failed")?;
        let browser_search = browser_search
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_search_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser search keybinds parse failed")?;
        let browser_songs = browser_songs
            .into_iter()
            .map(|(k, v)| {
                let v =
                    KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_songs_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser songs keybinds parse failed")?;
        let browser_artist_songs = browser_artist_songs
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_artist_songs_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser artist songs keybinds parse failed")?;
        let browser_playlist_songs = browser_playlist_songs
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(
                    &k,
                    v,
                    Some(&mut browser_playlist_songs_mode_names),
                )?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Browser playlist songs keybinds parse failed")?;
        let text_entry = text_entry
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut text_entry_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Text entry keybinds parse failed")?;
        let help = help
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut help_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Help keybinds parse failed")?;
        let sort = sort
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut sort_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Sort keybinds parse failed")?;
        let filter = filter
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut filter_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Filter keybinds parse failed")?;
        let list = list
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut list_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("List keybinds parse failed")?;
        let log = log
            .into_iter()
            .map(|(k, v)| {
                let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut log_mode_names))?;
                Ok((k, v))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .context("Log keybinds parse failed")?;
        Ok(YoutuiKeymap {
            global,
            playlist,
            browser,
            browser_artists,
            browser_search,
            browser_songs,
            browser_artist_songs,
            help,
            sort,
            filter,
            text_entry,
            list,
            log,
            browser_playlists,
            browser_playlist_songs,
        })
    }
}

impl<A: Action> KeyActionTree<A> {
    pub fn new_key(action: A) -> Self {
        Self::Key(KeyAction {
            action,
            visibility: Default::default(),
        })
    }
    pub fn new_key_with_visibility(action: A, visibility: KeyActionVisibility) -> Self {
        Self::Key(KeyAction { action, visibility })
    }
    pub fn new_mode<I>(binds: I, name: String) -> Self
    where
        I: IntoIterator<Item = (Keybind, KeyActionTree<A>)>,
    {
        Self::Mode {
            keys: FromIterator::from_iter(binds),
            name: Some(name),
        }
    }
    /// Merge this KeyActionTree with another.
    fn merge(&mut self, other: KeyActionTree<A>) {
        match self {
            KeyActionTree::Key(_) => *self = other,
            KeyActionTree::Mode {
                name: this_name,
                keys: keys_this,
            } => match other {
                KeyActionTree::Key(key_action) => *self = KeyActionTree::Key(key_action),
                KeyActionTree::Mode {
                    name: other_name,
                    keys: keys_other,
                } => {
                    if other_name.is_some() {
                        *this_name = other_name;
                    }
                    merge_keymaps(keys_this, keys_other);
                }
            },
        }
    }
    /// Try to create a KeyActionTree from a KeyStringTree.
    fn try_from_stringy(
        key: &Keybind,
        stringy: KeyStringTree,
        mode_names: Option<&mut BTreeMap<Keybind, ModeNameEnum>>,
    ) -> Result<Self>
    where
        A: TryFrom<String, Error = anyhow::Error>,
    {
        let new: KeyActionTree<A> = match stringy {
            KeyStringTree::Key(k) => KeyActionTree::Key(k.try_map(TryInto::try_into)?),
            KeyStringTree::Mode(m) => {
                let mode_name_enum = mode_names.and_then(|m| m.remove(key));
                let (mut next_modes, cur_mode_name) = match mode_name_enum {
                    Some(ModeNameEnum::Submode { name, keys }) => (Some(keys), name),
                    Some(ModeNameEnum::Name(name)) => (None, Some(name)),
                    None => (None, None),
                };
                KeyActionTree::Mode {
                    keys: m
                        .into_iter()
                        .map(|(k, a)| {
                            let v = KeyActionTree::try_from_stringy(&k, a, next_modes.as_mut())?;
                            Ok::<_, Error>((k, v))
                        })
                        .collect::<std::result::Result<_, _>>()?,
                    name: cur_mode_name,
                }
            }
        };
        Ok(new)
    }
    /// # Note
    /// Currently, visibility for a mode can't be set in config, so it is set to
    /// the default.
    pub fn get_visibility(&self) -> KeyActionVisibility {
        match self {
            KeyActionTree::Key(k) => k.visibility,
            KeyActionTree::Mode { .. } => KeyActionVisibility::default(),
        }
    }
    /// If a key, get the context of the key's action.
    /// If a mode, recursively get the context of the first key's keyactiontree.
    /// Returns String::default() if no keys in the mode.
    pub fn get_context(&self) -> Cow<'_, str> {
        match self {
            KeyActionTree::Key(k) => k.action.context(),
            KeyActionTree::Mode { keys, .. } => keys
                .iter()
                .next()
                .map(|(_, kt)| kt.get_context())
                .unwrap_or_default(),
        }
    }
}

impl<A> KeyAction<A> {
    fn try_map<U, E>(
        self,
        f: impl FnOnce(A) -> std::result::Result<U, E>,
    ) -> std::result::Result<KeyAction<U>, E> {
        let Self { action, visibility } = self;
        Ok(KeyAction {
            action: f(action)?,
            visibility,
        })
    }
}

impl FromStr for KeyAction<String> {
    type Err = Infallible;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(KeyAction {
            action: s.to_string(),
            visibility: Default::default(),
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum ModeNameEnum {
    Submode {
        name: Option<String>,
        keys: BTreeMap<Keybind, ModeNameEnum>,
    },
    #[serde(untagged)]
    Name(String),
}

fn default_global_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('+')),
            KeyActionTree::new_key(AppAction::VolUp),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('-')),
            KeyActionTree::new_key(AppAction::VolDown),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('>')),
            KeyActionTree::new_key(AppAction::NextSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('<')),
            KeyActionTree::new_key(AppAction::PrevSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(']')),
            KeyActionTree::new_key(AppAction::SeekForward),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('[')),
            KeyActionTree::new_key(AppAction::SeekBack),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(1)),
            KeyActionTree::new_key_with_visibility(
                AppAction::ToggleHelp,
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(10)),
            KeyActionTree::new_key_with_visibility(AppAction::Quit, KeyActionVisibility::Global),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(12)),
            KeyActionTree::new_key_with_visibility(
                AppAction::ViewLogs,
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
            KeyActionTree::new_key_with_visibility(
                AppAction::PlayPause,
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Char('c'), KeyModifiers::CONTROL),
            KeyActionTree::new_key(AppAction::Quit),
        ),
    ])
}
fn default_playlist_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Playlist(ViewBrowser),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('S')),
            KeyActionTree::new_key_with_visibility(
                AppAction::Playlist(PlaylistAction::ToggleShuffle),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key(AppAction::Playlist(PlaylistAction::PlaySelected)),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('d')),
                        KeyActionTree::new_key(AppAction::Playlist(PlaylistAction::DeleteSelected)),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('D')),
                        KeyActionTree::new_key(AppAction::Playlist(PlaylistAction::DeleteAll)),
                    ),
                ],
                "Playlist Action".into(),
            ),
        ),
    ])
}
fn default_browser_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Browser(BrowserAction::ViewPlaylist),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(2)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Browser(BrowserAction::Search),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key(AppAction::Browser(BrowserAction::Left)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(6)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Browser(BrowserAction::ChangeSearchType),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key(AppAction::Browser(BrowserAction::Right)),
        ),
    ])
}
fn default_browser_artists_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        KeyActionTree::new_key(AppAction::BrowserArtists(
            BrowserArtistsAction::DisplaySelectedArtistAlbums,
        )),
    )])
}
fn default_browser_playlists_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        KeyActionTree::new_key(AppAction::BrowserPlaylists(
            BrowserPlaylistsAction::DisplaySelectedPlaylist,
        )),
    )])
}
fn default_browser_search_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key(AppAction::BrowserSearch(
                BrowserSearchAction::NextSearchSuggestion,
            )),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key(AppAction::BrowserSearch(
                BrowserSearchAction::PrevSearchSuggestion,
            )),
        ),
    ])
}
fn default_browser_artist_songs_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserArtistSongs(BrowserArtistSongsAction::Filter),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserArtistSongs(BrowserArtistSongsAction::Sort),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::AddSongToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('p')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::PlaySongs,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('a')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::PlayAlbum,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::PlaySong,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('P')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::AddSongsToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('A')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::AddAlbumToPlaylist,
                        )),
                    ),
                ],
                "Play".into(),
            ),
        ),
    ])
}
fn default_browser_playlist_songs_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserPlaylistSongs(BrowserPlaylistSongsAction::Filter),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserPlaylistSongs(BrowserPlaylistSongsAction::Sort),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                        KeyActionTree::new_key(AppAction::BrowserPlaylistSongs(
                            BrowserPlaylistSongsAction::AddSongToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('p')),
                        KeyActionTree::new_key(AppAction::BrowserPlaylistSongs(
                            BrowserPlaylistSongsAction::PlaySongs,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key(AppAction::BrowserPlaylistSongs(
                            BrowserPlaylistSongsAction::PlaySong,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('P')),
                        KeyActionTree::new_key(AppAction::BrowserPlaylistSongs(
                            BrowserPlaylistSongsAction::AddSongsToPlaylist,
                        )),
                    ),
                ],
                "Play".into(),
            ),
        ),
    ])
}
fn default_browser_songs_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserSongs(BrowserSongsAction::Filter),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserSongs(BrowserSongsAction::Sort),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                        KeyActionTree::new_key(AppAction::BrowserSongs(
                            BrowserSongsAction::AddSongToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('p')),
                        KeyActionTree::new_key(AppAction::BrowserSongs(
                            BrowserSongsAction::PlaySongs,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key(AppAction::BrowserSongs(
                            BrowserSongsAction::PlaySong,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('P')),
                        KeyActionTree::new_key(AppAction::BrowserSongs(
                            BrowserSongsAction::AddSongsToPlaylist,
                        )),
                    ),
                ],
                "Play".into(),
            ),
        ),
    ])
}
fn default_help_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key_with_visibility(
                AppAction::Help(HelpAction::Close),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(1)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Help(HelpAction::Close),
                KeyActionVisibility::Global,
            ),
        ),
    ])
}
fn default_sort_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::SortSelectedAsc),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Enter, KeyModifiers::ALT),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::SortSelectedDesc),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::F(4), KeyModifiers::ALT),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::ClearSort),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::Close),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::Close),
                KeyActionVisibility::Global,
            ),
        ),
    ])
}
fn default_filter_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::Close),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::Close),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::Apply),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::F(3), KeyModifiers::ALT),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::ClearFilter),
                KeyActionVisibility::Global,
            ),
        ),
    ])
}
fn default_text_entry_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key_with_visibility(
                AppAction::TextEntry(TextEntryAction::Submit),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key_with_visibility(
                AppAction::TextEntry(TextEntryAction::Left),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key_with_visibility(
                AppAction::TextEntry(TextEntryAction::Right),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Backspace),
            KeyActionTree::new_key_with_visibility(
                AppAction::TextEntry(TextEntryAction::Backspace),
                KeyActionVisibility::Hidden,
            ),
        ),
    ])
}
fn default_log_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Log(LoggerAction::ViewBrowser),
                KeyActionVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Left, KeyModifiers::SHIFT),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ReduceCaptured)),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Right, KeyModifiers::SHIFT),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::IncreaseCaptured)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ReduceShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::IncreaseShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::Up)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::Down)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::PageUp)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::PageDown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('t')),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ToggleHideFiltered)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ExitPageMode)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('f')),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ToggleTargetFocus)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('h')),
            KeyActionTree::new_key(AppAction::Log(LoggerAction::ToggleTargetSelector)),
        ),
    ])
}
fn default_list_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key_with_visibility(
                AppAction::List(ListAction::Up),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key_with_visibility(
                AppAction::List(ListAction::Down),
                KeyActionVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyActionTree::new_key(AppAction::List(ListAction::PageUp)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyActionTree::new_key(AppAction::List(ListAction::PageDown)),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::{KeyActionTree, merge_keymaps};
    use crate::app::ui::action::AppAction;
    use crate::app::ui::browser::artistsearch::search_panel::BrowserArtistsAction;
    use crate::app::ui::browser::artistsearch::songs_panel::BrowserArtistSongsAction;
    use crate::config::keymap::{Keymap, remove_action_from_keymap};
    use crate::keybind::Keybind;

    #[test]
    fn test_add_key() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::BrowserArtists(
                BrowserArtistsAction::DisplaySelectedArtistAlbums,
            )),
        )]);
        let to_add = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key(AppAction::Quit),
        )]);
        merge_keymaps(&mut keys, to_add);
        let expected = FromIterator::from_iter([
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                KeyActionTree::new_key(AppAction::BrowserArtists(
                    BrowserArtistsAction::DisplaySelectedArtistAlbums,
                )),
            ),
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                KeyActionTree::new_key(AppAction::Quit),
            ),
        ]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_add_key_overrides_old() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::Quit),
        )]);
        let to_add = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::NoOp),
        )]);
        merge_keymaps(&mut keys, to_add);
        let expected = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::NoOp),
        )]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_add_mode() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::BrowserArtists(
                BrowserArtistsAction::DisplaySelectedArtistAlbums,
            )),
        )]);
        let to_add = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_mode(
                [(
                    Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                    KeyActionTree::new_key(AppAction::Quit),
                )],
                "New Modename".into(),
            ),
        )]);
        merge_keymaps(&mut keys, to_add);
        let expected = Keymap::from_iter([
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                KeyActionTree::new_key(AppAction::BrowserArtists(
                    BrowserArtistsAction::DisplaySelectedArtistAlbums,
                )),
            ),
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                KeyActionTree::new_mode(
                    [(
                        Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                        KeyActionTree::new_key(AppAction::Quit),
                    )],
                    "New Modename".into(),
                ),
            ),
        ]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_add_key_to_mode() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [(
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                    KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                        BrowserArtistSongsAction::AddSongToPlaylist,
                    )),
                )],
                "Play".into(),
            ),
        )]);
        let to_add = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [(
                    Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                    KeyActionTree::new_key(AppAction::Quit),
                )],
                "New Modename".into(),
            ),
        )]);
        merge_keymaps(&mut keys, to_add);
        let expected = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                        KeyActionTree::new_key(AppAction::BrowserArtistSongs(
                            BrowserArtistSongsAction::AddSongToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                        KeyActionTree::new_key(AppAction::Quit),
                    ),
                ],
                "New Modename".into(),
            ),
        )]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_remove_action() {
        let mut keys = Keymap::from_iter([
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                KeyActionTree::new_key(AppAction::BrowserArtists(
                    BrowserArtistsAction::DisplaySelectedArtistAlbums,
                )),
            ),
            (
                Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                KeyActionTree::new_key(AppAction::Quit),
            ),
        ]);
        remove_action_from_keymap(&mut keys, &AppAction::Quit);
        let expected = FromIterator::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key(AppAction::BrowserArtists(
                BrowserArtistsAction::DisplaySelectedArtistAlbums,
            )),
        )]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_remove_action_from_mode() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                        KeyActionTree::new_key(AppAction::Quit),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key(AppAction::BrowserArtists(
                            BrowserArtistsAction::DisplaySelectedArtistAlbums,
                        )),
                    ),
                ],
                "New Modename".into(),
            ),
        )]);
        remove_action_from_keymap(&mut keys, &AppAction::Quit);
        let expected = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_mode(
                [(
                    Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                    KeyActionTree::new_key(AppAction::BrowserArtists(
                        BrowserArtistsAction::DisplaySelectedArtistAlbums,
                    )),
                )],
                "New Modename".into(),
            ),
        )]);
        pretty_assertions::assert_eq!(keys, expected);
    }
    #[test]
    fn test_remove_action_removes_mode() {
        let mut keys = Keymap::from_iter([(
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_mode(
                [(
                    Keybind::new_unmodified(crossterm::event::KeyCode::Up),
                    KeyActionTree::new_key(AppAction::Quit),
                )],
                "New Modename".into(),
            ),
        )]);
        remove_action_from_keymap(&mut keys, &AppAction::Quit);
        let expected = Keymap::from_iter([]);
        pretty_assertions::assert_eq!(keys, expected);
    }
}
