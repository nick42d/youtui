use crate::app::component::actionhandler::Action;
use crate::app::keycommand::CommandVisibility;
use crate::app::keycommand::Keybind;
use crate::app::ui::logger::LoggerActio;
use crate::app::ui::YoutuiWindow;
use crate::get_config_dir;
use crate::Result;
use clap::ValueEnum;
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymap,
    pub mode_names: YoutuiModeNames,
}

#[derive(ValueEnum, Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[value(name = "oauth")]
    OAuth,
    #[default]
    Browser,
}

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub enum KeyEnum<Action> {
    Key {
        // Consider - can there be multiple actions?
        // Consider - can an action access global commands? Or commands from another component?
        // Consider - case where component has list and help keybinds, but some keybinds share a
        // mode. What happens here.
        action: Action,
        value: usize,
        visibility: CommandVisibility,
    },
    Mode(HashMap<Keybind, KeyEnum<Action>>),
}

#[derive(Debug, Serialize, Deserialize)]
enum ModeNameEnum {
    Name(String),
    Submode(HashMap<Keybind, ModeNameEnum>),
}

impl Action for AppAction {
    type State = YoutuiWindow;
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            AppAction::Log(a) => a.context(),
            _ => todo!(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            AppAction::Log(a) => a.describe(),
            _ => todo!(),
        }
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            AppAction::Log(a) => a
                .apply(&mut state.logger)
                .await
                .map(|this: &mut Self::State| &mut this.logger),
            _ => todo!(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppAction {
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
    Log(LoggerActio),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaylistAction {
    ViewBrowser,
    Left,
    Right,
    PlaySelected,
    DeleteSelected,
    DeleteAll,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserAction {
    ViewPlaylist,
    Search,
    Left,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserSearchAction {
    SearchArtist,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HelpAction {
    CloseHelp,
    ListAction(ListAction),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ListAction {
    Up(usize),
    Down(usize),
}

impl From<ListAction> for HelpAction {
    fn from(value: ListAction) -> Self {
        HelpAction::ListAction(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FilterAction {
    CloseFilter,
    ClearFilter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortAction {
    CloseSort,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TextEntryAction {
    Submit,
    Left,
    Right,
    Backspace,
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
