use crate::app::{
    keycommand::{CommandVisibility, Keybind},
    ui::{
        action::{AppAction, HelpAction, ListAction, TextEntryAction},
        browser::{
            artistalbums::{
                albumsongs::{BrowserSongsAction, FilterAction, SortAction},
                artistsearch::{BrowserArtistsAction, BrowserSearchAction},
            },
            BrowserAction,
        },
        logger::LoggerAction,
        playlist::PlaylistAction::{self, ViewBrowser},
    },
};
use crossterm::event::KeyModifiers;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, convert::Infallible, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct YoutuiKeymap {
    pub global: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub playlist: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_artists: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_search: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub browser_songs: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub help: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub sort: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub filter: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub text_entry: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub list: BTreeMap<Keybind, KeyActionTree<AppAction>>,
    pub log: BTreeMap<Keybind, KeyActionTree<AppAction>>,
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
            help: default_help_keybinds(),
            sort: default_sort_keybinds(),
            filter: default_filter_keybinds(),
            text_entry: default_text_entry_keybinds(),
            list: default_list_keybinds(),
            log: default_log_keybinds(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct YoutuiKeymapIR {
    pub global: BTreeMap<Keybind, KeyStringTree>,
    pub playlist: BTreeMap<Keybind, KeyStringTree>,
    pub browser: BTreeMap<Keybind, KeyStringTree>,
    pub browser_artists: BTreeMap<Keybind, KeyStringTree>,
    pub browser_search: BTreeMap<Keybind, KeyStringTree>,
    pub browser_songs: BTreeMap<Keybind, KeyStringTree>,
    pub help: BTreeMap<Keybind, KeyStringTree>,
    pub sort: BTreeMap<Keybind, KeyStringTree>,
    pub filter: BTreeMap<Keybind, KeyStringTree>,
    pub text_entry: BTreeMap<Keybind, KeyStringTree>,
    pub list: BTreeMap<Keybind, KeyStringTree>,
    pub log: BTreeMap<Keybind, KeyStringTree>,
}

impl YoutuiKeymap {
    pub fn try_from_stringy(
        keys: YoutuiKeymapIR,
        mode_names: YoutuiModeNamesIR,
    ) -> std::result::Result<Self, String> {
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
        } = mode_names;
        Ok(Self {
            global: global
                .into_iter()
                .map(move |(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut global_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            playlist: playlist
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut playlist_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            browser: browser
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut browser_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            browser_artists: browser_artists
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(
                        &k,
                        v,
                        Some(&mut browser_artists_mode_names),
                    )?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            browser_search: browser_search
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(
                        &k,
                        v,
                        Some(&mut browser_search_mode_names),
                    )?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            browser_songs: browser_songs
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(
                        &k,
                        v,
                        Some(&mut browser_songs_mode_names),
                    )?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            text_entry: text_entry
                .into_iter()
                .map(|(k, v)| {
                    let v =
                        KeyActionTree::try_from_stringy(&k, v, Some(&mut text_entry_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            help: help
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut help_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            sort: sort
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut sort_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            filter: filter
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut filter_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            list: list
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut list_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
            log: log
                .into_iter()
                .map(|(k, v)| {
                    let v = KeyActionTree::try_from_stringy(&k, v, Some(&mut log_mode_names))?;
                    Ok((k, v))
                })
                .collect::<std::result::Result<BTreeMap<_, _>, String>>()?,
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
// TODO: Mode visibility
pub struct YoutuiModeNamesIR {
    global: BTreeMap<Keybind, ModeNameEnum>,
    playlist: BTreeMap<Keybind, ModeNameEnum>,
    browser: BTreeMap<Keybind, ModeNameEnum>,
    browser_artists: BTreeMap<Keybind, ModeNameEnum>,
    browser_search: BTreeMap<Keybind, ModeNameEnum>,
    browser_songs: BTreeMap<Keybind, ModeNameEnum>,
    help: BTreeMap<Keybind, ModeNameEnum>,
    sort: BTreeMap<Keybind, ModeNameEnum>,
    filter: BTreeMap<Keybind, ModeNameEnum>,
    text_entry: BTreeMap<Keybind, ModeNameEnum>,
    list: BTreeMap<Keybind, ModeNameEnum>,
    log: BTreeMap<Keybind, ModeNameEnum>,
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
        keys: BTreeMap<Keybind, KeyActionTree<A>>,
    },
}

impl<A> KeyActionTree<A> {
    pub fn new_key_defaulted(action: A) -> Self {
        Self::Key(KeyAction {
            action,
            value: Default::default(),
            visibility: Default::default(),
        })
    }
    pub fn new_key_with_visibility(action: A, visibility: CommandVisibility) -> Self {
        Self::Key(KeyAction {
            action,
            value: Default::default(),
            visibility,
        })
    }
    pub fn new_key(action: A, value: usize, visibility: CommandVisibility) -> Self {
        Self::Key(KeyAction {
            action,
            value: Some(value),
            visibility,
        })
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
    fn try_from_stringy(
        key: &Keybind,
        stringy: KeyStringTree,
        mode_names: Option<&mut BTreeMap<Keybind, ModeNameEnum>>,
    ) -> std::result::Result<Self, String>
    where
        A: TryFrom<String, Error = String>,
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
                            Ok::<_, String>((k, v))
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
    pub fn get_visibility(&self) -> CommandVisibility {
        match self {
            KeyActionTree::Key(k) => k.visibility,
            KeyActionTree::Mode { .. } => CommandVisibility::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyAction<A> {
    // Consider - can there be multiple actions?
    // Consider - can an action access global commands? Or commands from another component?
    // Consider - case where component has list and help keybinds, but some keybinds share a
    // mode. What happens here.
    pub action: A,
    #[serde(default)]
    pub value: Option<usize>,
    #[serde(default)]
    pub visibility: CommandVisibility,
}

impl<A> KeyAction<A> {
    fn try_map<U, E>(
        self,
        f: impl FnOnce(A) -> std::result::Result<U, E>,
    ) -> std::result::Result<KeyAction<U>, E> {
        let Self {
            action,
            value,
            visibility,
        } = self;
        Ok(KeyAction {
            action: f(action)?,
            value,
            visibility,
        })
    }
}

impl FromStr for KeyAction<String> {
    type Err = Infallible;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(KeyAction {
            action: s.to_string(),
            value: Default::default(),
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
            KeyActionTree::new_key(AppAction::VolUp, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('-')),
            KeyActionTree::new_key(AppAction::VolDown, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('>')),
            KeyActionTree::new_key_defaulted(AppAction::NextSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('<')),
            KeyActionTree::new_key_defaulted(AppAction::PrevSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(']')),
            KeyActionTree::new_key(AppAction::SeekForwardS, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('[')),
            KeyActionTree::new_key(AppAction::SeekBackS, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(1)),
            KeyActionTree::new_key_with_visibility(
                AppAction::ToggleHelp,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(10)),
            KeyActionTree::new_key_with_visibility(AppAction::Quit, CommandVisibility::Global),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(12)),
            KeyActionTree::new_key_with_visibility(AppAction::ViewLogs, CommandVisibility::Global),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
            KeyActionTree::new_key_with_visibility(AppAction::Pause, CommandVisibility::Global),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Char('c'), KeyModifiers::CONTROL),
            KeyActionTree::new_key_defaulted(AppAction::Quit),
        ),
    ])
}
fn default_playlist_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Playlist(ViewBrowser),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key_defaulted(AppAction::Playlist(
                            PlaylistAction::PlaySelected,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('d')),
                        KeyActionTree::new_key_defaulted(AppAction::Playlist(
                            PlaylistAction::DeleteSelected,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('D')),
                        KeyActionTree::new_key_defaulted(AppAction::Playlist(
                            PlaylistAction::DeleteAll,
                        )),
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
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(2)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Browser(BrowserAction::Search),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key_defaulted(AppAction::Browser(BrowserAction::Left)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key_defaulted(AppAction::Browser(BrowserAction::Right)),
        ),
    ])
}
fn default_browser_artists_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        KeyActionTree::new_key_defaulted(AppAction::BrowserArtists(
            BrowserArtistsAction::DisplaySelectedArtistAlbums,
        )),
    )])
}
fn default_browser_search_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key_defaulted(AppAction::BrowserSearch(
                BrowserSearchAction::NextSearchSuggestion,
            )),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key_defaulted(AppAction::BrowserSearch(
                BrowserSearchAction::PrevSearchSuggestion,
            )),
        ),
    ])
}
fn default_browser_songs_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserSongs(BrowserSongsAction::Filter),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::BrowserSongs(BrowserSongsAction::Sort),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_mode(
                [
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::AddSongToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('p')),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::PlaySongs,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('a')),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::PlayAlbum,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::PlaySong,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('P')),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::AddSongsToPlaylist,
                        )),
                    ),
                    (
                        Keybind::new_unmodified(crossterm::event::KeyCode::Char('A')),
                        KeyActionTree::new_key_defaulted(AppAction::BrowserSongs(
                            BrowserSongsAction::AddAlbumToPlaylist,
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
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(1)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Help(HelpAction::Close),
                CommandVisibility::Global,
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
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Enter, KeyModifiers::ALT),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::SortSelectedDesc),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('C')),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::ClearSort),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::Close),
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Sort(SortAction::Close),
                CommandVisibility::Global,
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
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::Close),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(6)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Filter(FilterAction::ClearFilter),
                CommandVisibility::Global,
            ),
        ),
    ])
}
fn default_text_entry_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyActionTree::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Submit)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Left)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Right)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Backspace),
            KeyActionTree::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Backspace)),
        ),
    ])
}
fn default_log_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyActionTree::new_key_with_visibility(
                AppAction::Log(LoggerAction::ViewBrowser),
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('[')),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ReduceCaptured)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(']')),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::IncreaseCaptured)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ReduceShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::IncreaseShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::Up)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::Down)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::PageUp)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::PageDown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ToggleHideFiltered)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ExitPageMode)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('f')),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ToggleTargetFocus)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('h')),
            KeyActionTree::new_key_defaulted(AppAction::Log(LoggerAction::ToggleTargetSelector)),
        ),
    ])
}
fn default_list_keybinds() -> BTreeMap<Keybind, KeyActionTree<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyActionTree::new_key(
                AppAction::List(ListAction::Up),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyActionTree::new_key(
                AppAction::List(ListAction::Down),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyActionTree::new_key(
                AppAction::List(ListAction::Up),
                10,
                CommandVisibility::Standard,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyActionTree::new_key(
                AppAction::List(ListAction::Down),
                10,
                CommandVisibility::Standard,
            ),
        ),
    ])
}
