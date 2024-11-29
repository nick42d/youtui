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
        playlist::PlaylistAction::{DeleteSelected, PlaySelected, ViewBrowser},
    },
};
use crossterm::event::KeyModifiers;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct YoutuiKeymap {
    pub global: HashMap<Keybind, KeyEnum<AppAction>>,
    pub playlist: HashMap<Keybind, KeyEnum<AppAction>>,
    pub browser: HashMap<Keybind, KeyEnum<AppAction>>,
    pub browser_artists: HashMap<Keybind, KeyEnum<AppAction>>,
    pub browser_search: HashMap<Keybind, KeyEnum<AppAction>>,
    pub browser_songs: HashMap<Keybind, KeyEnum<AppAction>>,
    pub help: HashMap<Keybind, KeyEnum<AppAction>>,
    pub sort: HashMap<Keybind, KeyEnum<AppAction>>,
    pub filter: HashMap<Keybind, KeyEnum<AppAction>>,
    pub text_entry: HashMap<Keybind, KeyEnum<AppAction>>,
    pub list: HashMap<Keybind, KeyEnum<AppAction>>,
    pub log: HashMap<Keybind, KeyEnum<AppAction>>,
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
    pub global: HashMap<Keybind, KeyEnumString>,
    pub playlist: HashMap<Keybind, KeyEnumString>,
    pub browser: HashMap<Keybind, KeyEnumString>,
    pub browser_artists: HashMap<Keybind, KeyEnumString>,
    pub browser_search: HashMap<Keybind, KeyEnumString>,
    pub browser_songs: HashMap<Keybind, KeyEnumString>,
    pub help: HashMap<Keybind, KeyEnumString>,
    pub sort: HashMap<Keybind, KeyEnumString>,
    pub filter: HashMap<Keybind, KeyEnumString>,
    pub text_entry: HashMap<Keybind, KeyEnumString>,
    pub list: HashMap<Keybind, KeyEnumString>,
    pub log: HashMap<Keybind, KeyEnumString>,
}

impl TryFrom<YoutuiKeymapIR> for YoutuiKeymap {
    type Error = String;
    fn try_from(value: YoutuiKeymapIR) -> std::result::Result<Self, Self::Error> {
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
        } = value;
        Ok(Self {
            global: global
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            playlist: playlist
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            browser: browser
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            browser_artists: browser_artists
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            browser_search: browser_search
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            browser_songs: browser_songs
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            help: help
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            sort: sort
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            filter: filter
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            text_entry: text_entry
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            list: list
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
            log: log
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<std::result::Result<HashMap<_, _>, String>>()?,
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct YoutuiModeNames {
    global: HashMap<Keybind, ModeNameEnum>,
    playlist: HashMap<Keybind, ModeNameEnum>,
    browser: HashMap<Keybind, ModeNameEnum>,
    browser_artists: HashMap<Keybind, ModeNameEnum>,
    browser_search: HashMap<Keybind, ModeNameEnum>,
    browser_songs: HashMap<Keybind, ModeNameEnum>,
    help: HashMap<Keybind, ModeNameEnum>,
    sort: HashMap<Keybind, ModeNameEnum>,
    filter: HashMap<Keybind, ModeNameEnum>,
    text_entry: HashMap<Keybind, ModeNameEnum>,
    list: HashMap<Keybind, ModeNameEnum>,
    log: HashMap<Keybind, ModeNameEnum>,
}

impl Default for YoutuiModeNames {
    fn default() -> Self {
        Self {
            global: default_global_mode_names(),
            playlist: default_playlist_mode_names(),
            browser: default_browser_mode_names(),
            browser_artists: default_browser_artists_mode_names(),
            browser_search: default_browser_search_mode_names(),
            browser_songs: default_browser_songs_mode_names(),
            help: default_help_mode_names(),
            sort: default_sort_mode_names(),
            filter: default_filter_mode_names(),
            text_entry: default_text_entry_mode_names(),
            list: default_list_mode_names(),
            log: default_log_mode_names(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyEnumString {
    #[serde(deserialize_with = "crate::core::string_or_struct")]
    Key(KeyEnumKey<String>),
    Mode(HashMap<Keybind, KeyEnumString>),
}

#[derive(Debug, PartialEq)]
pub enum KeyEnum<A: Default> {
    Key(KeyEnumKey<A>),
    Mode(HashMap<Keybind, KeyEnum<A>>),
}

impl<A: Default> KeyEnum<A> {
    fn new_key_defaulted(action: A) -> Self {
        Self::Key(KeyEnumKey {
            action,
            value: Default::default(),
            visibility: Default::default(),
        })
    }
    fn new_key(action: A, value: usize, visibility: CommandVisibility) -> Self {
        Self::Key(KeyEnumKey {
            action,
            value,
            visibility,
        })
    }
    fn new_mode<I>(binds: I) -> Self
    where
        I: IntoIterator<Item = (Keybind, KeyEnum<A>)>,
    {
        Self::Mode(FromIterator::from_iter(binds))
    }
}

impl TryFrom<KeyEnumString> for KeyEnum<AppAction> {
    type Error = String;
    fn try_from(value: KeyEnumString) -> std::result::Result<Self, Self::Error> {
        let new: KeyEnum<AppAction> = match value {
            KeyEnumString::Key(k) => KeyEnum::Key(k.try_map(TryInto::try_into)?),
            KeyEnumString::Mode(m) => KeyEnum::Mode(
                m.into_iter()
                    .map(|(k, a)| Ok::<_, String>((k, KeyEnum::<AppAction>::try_from(a)?)))
                    .collect::<std::result::Result<_, _>>()?,
            ),
        };
        Ok(new)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyEnumKey<A: Default> {
    // Consider - can there be multiple actions?
    // Consider - can an action access global commands? Or commands from another component?
    // Consider - case where component has list and help keybinds, but some keybinds share a
    // mode. What happens here.
    pub action: A,
    #[serde(default)]
    pub value: usize,
    #[serde(default)]
    pub visibility: CommandVisibility,
}

impl<A: Default> KeyEnumKey<A> {
    fn try_map<U: Default, E>(
        self,
        f: impl FnOnce(A) -> std::result::Result<U, E>,
    ) -> std::result::Result<KeyEnumKey<U>, E> {
        let Self {
            action,
            value,
            visibility,
        } = self;
        Ok(KeyEnumKey {
            action: f(action)?,
            value,
            visibility,
        })
    }
}

impl FromStr for KeyEnumKey<String> {
    type Err = Infallible;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(KeyEnumKey {
            action: s.to_string(),
            value: Default::default(),
            visibility: Default::default(),
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum ModeNameEnum {
    Submode(HashMap<Keybind, ModeNameEnum>),
    #[serde(untagged)]
    Name(String),
}

fn default_global_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('+')),
            KeyEnum::new_key(AppAction::VolUp, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('-')),
            KeyEnum::new_key(AppAction::VolDown, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('>')),
            KeyEnum::new_key_defaulted(AppAction::NextSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('<')),
            KeyEnum::new_key_defaulted(AppAction::PrevSong),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(']')),
            KeyEnum::new_key(AppAction::SeekForwardS, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('[')),
            KeyEnum::new_key(AppAction::SeekBackS, 5, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(1)),
            KeyEnum::new_key(AppAction::ToggleHelp, 1, CommandVisibility::Global),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(10)),
            KeyEnum::new_key(AppAction::Quit, 1, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(12)),
            KeyEnum::new_key(AppAction::ViewLogs, 1, CommandVisibility::Standard),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
            KeyEnum::new_key_defaulted(AppAction::Pause),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Char('c'), KeyModifiers::CONTROL),
            KeyEnum::new_key_defaulted(AppAction::Quit),
        ),
    ])
}
fn default_playlist_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyEnum::new_key_defaulted(AppAction::Playlist(ViewBrowser)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyEnum::new_mode([
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                    KeyEnum::new_key_defaulted(AppAction::Playlist(PlaySelected)),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('d')),
                    KeyEnum::new_key_defaulted(AppAction::Playlist(DeleteSelected)),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('D')),
                    KeyEnum::new_key_defaulted(AppAction::Playlist(DeleteSelected)),
                ),
            ]),
        ),
    ])
}
fn default_browser_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyEnum::new_key(
                AppAction::Browser(BrowserAction::ViewPlaylist),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(2)),
            KeyEnum::new_key(
                AppAction::Browser(BrowserAction::Search),
                1,
                CommandVisibility::Global,
            ),
        ),
    ])
}
fn default_browser_artists_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        KeyEnum::new_key_defaulted(AppAction::BrowserArtists(
            BrowserArtistsAction::DisplaySelectedArtistAlbums,
        )),
    )])
}
fn default_browser_search_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyEnum::new_key_defaulted(AppAction::BrowserSearch(
                BrowserSearchAction::NextSearchSuggestion,
            )),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyEnum::new_key_defaulted(AppAction::BrowserSearch(
                BrowserSearchAction::PrevSearchSuggestion,
            )),
        ),
    ])
}
fn default_browser_songs_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyEnum::new_key(
                AppAction::BrowserSongs(BrowserSongsAction::Filter),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyEnum::new_key(
                AppAction::BrowserSongs(BrowserSongsAction::Sort),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyEnum::new_mode([
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::AddSongToPlaylist,
                    )),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('p')),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::PlaySongs,
                    )),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('a')),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::PlayAlbum,
                    )),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::PlaySong,
                    )),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('P')),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::AddSongsToPlaylist,
                    )),
                ),
                (
                    Keybind::new_unmodified(crossterm::event::KeyCode::Char('A')),
                    KeyEnum::new_key_defaulted(AppAction::BrowserSongs(
                        BrowserSongsAction::AddAlbumToPlaylist,
                    )),
                ),
            ]),
        ),
    ])
}
fn default_help_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyEnum::new_key_defaulted(AppAction::Help(HelpAction::Close)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyEnum::new_key(
                AppAction::Help(HelpAction::Close),
                1,
                CommandVisibility::Global,
            ),
        ),
    ])
}
fn default_sort_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyEnum::new_key(
                AppAction::Sort(SortAction::SortSelectedAsc),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new(crossterm::event::KeyCode::Enter, KeyModifiers::ALT),
            KeyEnum::new_key(
                AppAction::Sort(SortAction::SortSelectedDesc),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('C')),
            KeyEnum::new_key(
                AppAction::Sort(SortAction::ClearSort),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyEnum::new_key(
                AppAction::Sort(SortAction::Close),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(4)),
            KeyEnum::new_key(
                AppAction::Sort(SortAction::Close),
                1,
                CommandVisibility::Global,
            ),
        ),
    ])
}
fn default_filter_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyEnum::new_key(
                AppAction::Filter(FilterAction::Close),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(3)),
            KeyEnum::new_key(
                AppAction::Filter(FilterAction::Close),
                1,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(6)),
            KeyEnum::new_key(
                AppAction::Filter(FilterAction::ClearFilter),
                1,
                CommandVisibility::Global,
            ),
        ),
    ])
}
fn default_text_entry_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
            KeyEnum::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Submit)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyEnum::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Left)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyEnum::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Right)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Backspace),
            KeyEnum::new_key_defaulted(AppAction::TextEntry(TextEntryAction::Backspace)),
        ),
    ])
}
fn default_log_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::F(5)),
            KeyEnum::new_key(
                AppAction::Log(LoggerAction::ViewBrowser),
                0,
                CommandVisibility::Global,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('[')),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ReduceCaptured)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(']')),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::IncreaseCaptured)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Left),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ReduceShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Right),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::IncreaseShown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::Up)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::Down)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::PageUp)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::PageDown)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char(' ')),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ToggleHideFiltered)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Esc),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ExitPageMode)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('h')),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ToggleTargetSelector)),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Char('f')),
            KeyEnum::new_key_defaulted(AppAction::Log(LoggerAction::ToggleTargetFocus)),
        ),
    ])
}
fn default_list_keybinds() -> HashMap<Keybind, KeyEnum<AppAction>> {
    FromIterator::from_iter([
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Up),
            KeyEnum::new_key(
                AppAction::List(ListAction::Up),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::Down),
            KeyEnum::new_key(
                AppAction::List(ListAction::Down),
                1,
                CommandVisibility::Hidden,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageUp),
            KeyEnum::new_key(
                AppAction::List(ListAction::Up),
                10,
                CommandVisibility::Standard,
            ),
        ),
        (
            Keybind::new_unmodified(crossterm::event::KeyCode::PageDown),
            KeyEnum::new_key(
                AppAction::List(ListAction::Down),
                10,
                CommandVisibility::Standard,
            ),
        ),
    ])
}

fn default_log_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_list_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_text_entry_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_filter_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_sort_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_help_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_browser_songs_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        ModeNameEnum::Name("Play".into()),
    )])
}
fn default_browser_artists_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_browser_search_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_browser_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
fn default_playlist_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    FromIterator::from_iter([(
        Keybind::new_unmodified(crossterm::event::KeyCode::Enter),
        ModeNameEnum::Name("Playlist Action".into()),
    )])
}
fn default_global_mode_names() -> HashMap<Keybind, ModeNameEnum> {
    Default::default()
}
