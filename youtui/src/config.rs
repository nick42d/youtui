use crate::app::component::actionhandler::Action;
use crate::app::keycommand::CommandVisibility;
use crate::app::keycommand::Keybind;
use crate::app::ui::browser::artistalbums::albumsongs::AlbumSongsPanel;
use crate::app::ui::browser::artistalbums::albumsongs::FilterManager;
use crate::app::ui::browser::artistalbums::albumsongs::SortManager;
use crate::app::ui::browser::Browser;
use crate::app::ui::logger::LoggerAction;
use crate::app::ui::playlist::Playlist;
use crate::app::ui::HelpMenu;
use crate::app::ui::WindowContext;
use crate::app::ui::YoutuiWindow;
use crate::app::view::Scrollable;
use crate::app::AppCallback;
use crate::async_rodio_sink::SeekDirection;
use crate::core::send_or_error;
use crate::get_config_dir;
use crate::Result;
use async_callback_manager::AsyncTask;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
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
            AppAction::VolUp(_)
            | AppAction::VolDown(_)
            | AppAction::NextSong
            | AppAction::PrevSong
            | AppAction::SeekForwardS(_)
            | AppAction::SeekBackS(_)
            | AppAction::ToggleHelp
            | AppAction::Quit
            | AppAction::ViewLogs
            | AppAction::Pause => "Global".into(),
            AppAction::Log(a) => a.context(),
            AppAction::Playlist(a) => a.context(),
            AppAction::Browser(a) => a.context(),
            AppAction::Filter(a) => a.context(),
            AppAction::Sort(a) => a.context(),
            AppAction::Help(a) => a.context(),
            AppAction::BrowserArtists(a) => a.context(),
            AppAction::BrowserSearch(a) => a.context(),
            AppAction::BrowserSongs(a) => a.context(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            AppAction::Quit => "Quit".into(),
            AppAction::PrevSong => "Prev Song".into(),
            AppAction::NextSong => "Next Song".into(),
            AppAction::Pause => "Pause".into(),
            AppAction::VolUp(n) => format!("Vol Up {n}").into(),
            AppAction::VolDown(n) => format!("Vol Down {n}").into(),
            AppAction::ToggleHelp => "Toggle Help".into(),
            AppAction::ViewLogs => "View Logs".into(),
            AppAction::SeekForwardS(s) => format!("Seek Forward {s}s").into(),
            AppAction::SeekBackS(s) => format!("Seek Back {s}s").into(),
            AppAction::Log(a) => a.describe(),
            AppAction::Playlist(a) => a.describe(),
            AppAction::Browser(a) => a.describe(),
            AppAction::Filter(a) => a.describe(),
            AppAction::Sort(a) => a.describe(),
            AppAction::Help(a) => a.describe(),
            AppAction::BrowserArtists(a) => a.describe(),
            AppAction::BrowserSearch(a) => a.describe(),
            AppAction::BrowserSongs(a) => a.describe(),
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
            AppAction::VolUp(n) => return state.handle_increase_volume(n).await,
            AppAction::VolDown(n) => return state.handle_increase_volume(-n).await,
            AppAction::NextSong => return state.handle_next(),
            AppAction::PrevSong => return state.handle_prev(),
            AppAction::SeekForwardS(s) => {
                return state.handle_seek(Duration::from_secs(s as u64), SeekDirection::Forward)
            }
            AppAction::SeekBackS(s) => {
                return state.handle_seek(Duration::from_secs(s as u64), SeekDirection::Back)
            }
            AppAction::ToggleHelp => state.toggle_help(),
            AppAction::Quit => send_or_error(&state.callback_tx, AppCallback::Quit).await,
            AppAction::ViewLogs => state.handle_change_context(WindowContext::Logs),
            AppAction::Pause => return state.pauseplay(),
            AppAction::Log(a) => {
                return a
                    .apply(&mut state.logger)
                    .await
                    .map(|this: &mut Self::State| &mut this.logger)
            }
            AppAction::Playlist(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.playlist)
                    .apply(state)
                    .await
            }
            AppAction::Browser(a) => {
                return a
                    .apply(&mut state.browser)
                    .await
                    .map(|this: &mut Self::State| &mut this.browser)
            }
            AppAction::Filter(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser.album_songs_list.filter)
                    .apply(state)
                    .await
            }
            AppAction::Sort(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser.album_songs_list.sort)
                    .apply(state)
                    .await
            }
            AppAction::Help(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.help)
                    .apply(state)
                    .await
            }
            AppAction::BrowserArtists(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser)
                    .apply(state)
                    .await
            }
            AppAction::BrowserSearch(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser)
                    .apply(state)
                    .await
            }
            AppAction::BrowserSongs(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser.album_songs_list)
                    .apply(state)
                    .await
            }
        };
        AsyncTask::new_no_op()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppAction {
    VolUp(i8),
    VolDown(i8),
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
    Log(LoggerAction),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaylistAction {
    ViewBrowser,
    PlaySelected,
    DeleteSelected,
    DeleteAll,
    List(ListAction),
}
impl Action for PlaylistAction {
    type State = Playlist;
    fn context(&self) -> std::borrow::Cow<str> {
        "Playlist".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            PlaylistAction::ViewBrowser => "View Browser",
            PlaylistAction::PlaySelected => "Play Selected",
            PlaylistAction::DeleteSelected => "Delete Selected",
            PlaylistAction::DeleteAll => "Delete All",
            PlaylistAction::List(a) => todo!(),
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            PlaylistAction::ViewBrowser => state.view_browser().await,
            PlaylistAction::List(ListAction::Down(n)) => state.increment_list(n),
            PlaylistAction::List(ListAction::Up(n)) => state.increment_list(-n),
            PlaylistAction::PlaySelected => return state.play_selected(),
            PlaylistAction::DeleteSelected => return state.delete_selected(),
            PlaylistAction::DeleteAll => return state.delete_all(),
        }
        AsyncTask::new_no_op()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserAction {
    ViewPlaylist,
    ToggleSearch,
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
            BrowserAction::ToggleSearch => "Toggle Search",
            BrowserAction::Left => "Left",
            BrowserAction::Right => "Right",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            BrowserAction::Left => state.left(),
            BrowserAction::Right => state.right(),
            BrowserAction::ViewPlaylist => {
                send_or_error(
                    &state.callback_tx,
                    AppCallback::ChangeContext(WindowContext::Playlist),
                )
                .await
            }
            BrowserAction::ToggleSearch => state.handle_toggle_search(),
        }
        AsyncTask::new_no_op()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserArtistsAction {
    DisplaySelectedArtistAlbums,
}

impl Action for BrowserArtistsAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            Self::DisplaySelectedArtistAlbums => "Display albums for selected artist",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            BrowserArtistsAction::DisplaySelectedArtistAlbums => state.get_songs(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrowserSearchAction {
    SearchArtist,
    PrevSearchSuggestion,
    NextSearchSuggestion,
}
impl Action for BrowserSearchAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Search Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            BrowserSearchAction::SearchArtist => "Search",
            BrowserSearchAction::PrevSearchSuggestion => "Prev Search Suggestion",
            BrowserSearchAction::NextSearchSuggestion => "Next Search Suggestion",
        }
        .into()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        match self {
            BrowserSearchAction::SearchArtist => return state.search(),
            BrowserSearchAction::PrevSearchSuggestion => {
                state.artist_list.search.increment_list(-1)
            }
            BrowserSearchAction::NextSearchSuggestion => state.artist_list.search.increment_list(1),
        }
        AsyncTask::new_no_op()
    }
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
impl Action for BrowserSongsAction {
    type State = AlbumSongsPanel;
    fn context(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HelpAction {
    CloseHelp,
    ListAction(ListAction),
}
impl Action for HelpAction {
    type State = HelpMenu;
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::CloseHelp => "Help".into(),
            HelpAction::ListAction(a) => match a {
                ListAction::Up(_) => "Help".into(),
                ListAction::Down(_) => "Help".into(),
            },
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::CloseHelp => "Close Help".into(),
            HelpAction::ListAction(a) => match a {
                ListAction::Up(n) => format!("Up {n}").into(),
                ListAction::Down(n) => format!("Down {n}").into(),
            },
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
            HelpAction::CloseHelp => state.shown = false,
            HelpAction::ListAction(a) => match a {
                ListAction::Up(n) => state.increment_list(n),
                ListAction::Down(n) => state.increment_list(-n),
            },
        }
        AsyncTask::new_no_op()
    }
}
impl From<ListAction> for HelpAction {
    fn from(value: ListAction) -> Self {
        Self::ListAction(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FilterAction {
    CloseFilter,
    ClearFilter,
}
impl Action for FilterAction {
    type State = FilterManager;
    fn context(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortAction {
    CloseSort,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}
impl Action for SortAction {
    type State = SortManager;
    fn context(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        todo!()
    }
    async fn apply(
        self,
        state: &mut Self::State,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        todo!()
    }
}

// SPECIAL CASES

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ListAction {
    Up(isize),
    Down(isize),
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
