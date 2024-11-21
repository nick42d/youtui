use crate::app::keycommand::CommandVisibility;
use crate::app::keycommand::Keybind;
use crate::get_config_dir;
use crate::Result;
use clap::Command;
use clap::ValueEnum;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ytmapi_rs::auth::OAuthToken;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize)]
pub enum ApiKey {
    OAuthToken(OAuthToken),
    // BrowserToken takes the cookie, not the BrowserToken itself. This is because to obtain the
    // BrowserToken you must make a web request, and we want to obtain it as lazily as possible.
    BrowserToken(String),
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKey::OAuthToken(_) => write!(f, "OAuthToken(/* private fields */"),
            ApiKey::BrowserToken(_) => write!(f, "BrowserToken(/* private fields */"),
        }
    }
}

pub struct KeybindBasic {
    sequence: Vec<Keybind>,
    // Consider - can there be multiple actions?
    // Consider - can an action access global commands? Or commands from another component?
    action: String,
    // Eg header, standard, hidden.
    visibility: u8,
}
pub struct ModeBasic {
    sequence: Vec<Keybind>,
    name: String,
}

pub struct Keybinds {
    binds: Vec<Keybind>,
    mode_names: Vec<ModeBasic>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth_type: AuthType,
    // Consider - should keybinds be per module, e.g keybinds.playlist, keybinds.browser.
    pub keybinds: HashMap<String, String>,
    pub mode_names: HashMap<String, String>,
}

enum AppAction {
    VolUp(usize),
    VolDown(usize),
    NextSong,
    PrevSong,
    SeekForwardS(usize),
    SeekBackS(usize),
    ToggleHelp,
    Quit,
    ViewLogs,
    Pause,
    Playlist(PlaylistAction),
    Browser(BrowserAction),
    Filter(FilterAction),
    Sort(SortAction),
    Help(HelpAction),
    BrowserArtists(BrowserArtistsAction),
    BrowserSearch(BrowserSearchAction),
    BrowserSongs(BrowserSongsAction),
    Log(LogAction),
}

enum PlaylistAction {
    ViewBrowser,
    Left,
    Right,
    PlaySelected,
    DeleteSelected,
    DeleteAll,
}

enum BrowserAction {
    ViewPlaylist,
    Search,
    Left,
    Right,
}

enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

enum BrowserSearchAction {
    SearchArtist,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}

enum BrowserSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
}

enum HelpAction {
    CloseHelp,
}

enum FilterAction {
    CloseFilter,
    ClearFilter,
}

enum SortAction {
    CloseSort,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}

enum TextEntryAction {
    Submit,
    Left,
    Right,
    Backspace,
}

enum LogAction {}

enum KeysEnum {
    Key(KeyEnum),
    Subcomponent,
}

enum KeyEnum {
    Key {
        action: String,
        value: usize,
        visibility: CommandVisibility,
    },
    Mode(HashMap<Key, KeyEnum>),
}

enum ModeNameEnum {
    Name(String),
    Submode(HashMap<Key, ModeNameEnum>),
}

pub struct Key;

pub struct YoutuiKeymap {
    global: HashMap<Keybind, KeyEnum>,
    playlist: HashMap<Keybind, KeyEnum>,
    browser: HashMap<Keybind, KeyEnum>,
    browser_artists: HashMap<Keybind, KeyEnum>,
    browser_search: HashMap<Keybind, KeyEnum>,
    browser_songs: HashMap<Keybind, KeyEnum>,
    help: HashMap<Keybind, KeyEnum>,
    sort: HashMap<Keybind, KeyEnum>,
    filter: HashMap<Keybind, KeyEnum>,
    text_entry: HashMap<Keybind, KeyEnum>,
    list: HashMap<Keybind, KeyEnum>,
}

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
}

#[derive(ValueEnum, Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[value(name = "oauth")]
    OAuth,
    #[default]
    Browser,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_dir = get_config_dir()?;
        if let Ok(config_file) = std::fs::read_to_string(config_dir.join(CONFIG_FILE_NAME)) {
            Ok(toml::from_str(&config_file)?)
        } else {
            Ok(Self::default())
        }
    }
}
