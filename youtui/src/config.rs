use crate::app::component::actionhandler::Action;
use crate::app::keycommand::CommandVisibility;
use crate::app::keycommand::Keybind;
use crate::app::ui::action::AppAction;
use crate::app::ui::browser::Browser;
use crate::app::ui::HelpMenu;
use crate::app::ui::WindowContext;
use crate::app::view::Scrollable;
use crate::app::AppCallback;
use crate::core::send_or_error;
use crate::get_config_dir;
use crate::Result;
use async_callback_manager::AsyncTask;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::str::FromStr;
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

#[derive(Debug)]
pub struct Config {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymap,
    pub mode_names: YoutuiModeNames,
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct ConfigIR {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymapIR,
    pub mode_names: YoutuiModeNames,
}

impl TryFrom<ConfigIR> for Config {
    type Error = String;
    fn try_from(value: ConfigIR) -> std::result::Result<Self, Self::Error> {
        let ConfigIR {
            auth_type,
            keybinds,
            mode_names,
        } = value;
        Ok(Config {
            auth_type,
            keybinds: keybinds.try_into()?,
            mode_names,
        })
    }
}

#[derive(ValueEnum, Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[value(name = "oauth")]
    OAuth,
    #[default]
    Browser,
}

#[derive(Debug)]
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

#[derive(Default, Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyEnumString {
    #[serde(deserialize_with = "super::core::string_or_struct")]
    Key(KeyEnumKey<String>),
    Mode(HashMap<Keybind, KeyEnumString>),
}

#[derive(Debug)]
pub enum KeyEnum<A: Default> {
    Key(KeyEnumKey<A>),
    Mode(HashMap<Keybind, KeyEnum<A>>),
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    ViewPlaylist,
    Search,
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
            BrowserAction::Search => "Toggle Search",
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
            BrowserAction::Search => state.handle_toggle_search(),
        }
        AsyncTask::new_no_op()
    }
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
    List(ListAction),
}
impl Action for BrowserSongsAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Songs Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match &self {
            BrowserSongsAction::PlaySong => "Play song",
            BrowserSongsAction::PlaySongs => "Play songs",
            BrowserSongsAction::PlayAlbum => "Play album",
            BrowserSongsAction::AddSongToPlaylist => "Add song to playlist",
            BrowserSongsAction::AddSongsToPlaylist => "Add songs to playlist",
            BrowserSongsAction::AddAlbumToPlaylist => "Add album to playlist",
            BrowserSongsAction::List(a) => match a {
                ListAction::Up => "Up",
                ListAction::Down => "Down",
            },
            BrowserSongsAction::Sort => "Sort",
            BrowserSongsAction::Filter => "Filter",
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
            BrowserSongsAction::PlayAlbum => state.play_album().await,
            BrowserSongsAction::PlaySong => state.play_song().await,
            BrowserSongsAction::PlaySongs => state.play_songs().await,
            BrowserSongsAction::AddAlbumToPlaylist => state.add_album_to_playlist().await,
            BrowserSongsAction::AddSongToPlaylist => state.add_song_to_playlist().await,
            BrowserSongsAction::AddSongsToPlaylist => state.add_songs_to_playlist().await,
            BrowserSongsAction::List(a) => match a {
                ListAction::Up => state.album_songs_list.increment_list(-1),
                ListAction::Down => state.album_songs_list.increment_list(1),
            },
            BrowserSongsAction::Sort => state.album_songs_list.handle_pop_sort(),
            BrowserSongsAction::Filter => state.album_songs_list.toggle_filter(),
        }
        AsyncTask::new_no_op()
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelpAction {
    Close,
    ListAction(ListAction),
}
impl Action for HelpAction {
    type State = HelpMenu;
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::Close => "Help".into(),
            HelpAction::ListAction(a) => match a {
                ListAction::Up => "Help".into(),
                ListAction::Down => "Help".into(),
            },
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::Close => "Close Help".into(),
            HelpAction::ListAction(a) => match a {
                ListAction::Up => format!("Up 1").into(),
                ListAction::Down => format!("Down 1").into(),
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
            HelpAction::Close => state.shown = false,
            HelpAction::ListAction(a) => match a {
                ListAction::Up => state.increment_list(1),
                ListAction::Down => state.increment_list(-1),
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterAction {
    Close,
    ClearFilter,
    Apply,
}
impl Action for FilterAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Filter".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            FilterAction::Close => "Close Filter",
            FilterAction::Apply => "Apply filter",
            FilterAction::ClearFilter => "Clear filter",
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
            FilterAction::Close => state.album_songs_list.toggle_filter(),
            FilterAction::Apply => state.album_songs_list.apply_filter(),
            FilterAction::ClearFilter => state.album_songs_list.clear_filter(),
        };
        AsyncTask::new_no_op()
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortAction {
    Close,
    ClearSort,
    SortSelectedAsc,
    SortSelectedDesc,
}
impl Action for SortAction {
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Filter".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            SortAction::Close => "Close sort",
            SortAction::ClearSort => "Clear sort",
            SortAction::SortSelectedAsc => "Sort ascending",
            SortAction::SortSelectedDesc => "Sort descending",
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
            SortAction::SortSelectedAsc => state.album_songs_list.handle_sort_cur_asc(),
            SortAction::SortSelectedDesc => state.album_songs_list.handle_sort_cur_desc(),
            SortAction::Close => state.album_songs_list.close_sort(),
            SortAction::ClearSort => state.album_songs_list.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
    }
}

// SPECIAL CASES

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListAction {
    Up,
    Down,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEntryAction {
    Submit,
    Left,
    Right,
    Backspace,
}

impl Config {
    pub async fn new(debug: bool) -> Result<Self> {
        let config_dir = get_config_dir()?;
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        if let Ok(config_file) = tokio::fs::read_to_string(&config_file_location).await {
            // NOTE: This happens before logging / app is initialised.
            if debug {
                println!(
                    "Loading config from {}",
                    config_file_location.to_string_lossy()
                );
            }
            todo!()
            // Ok(toml::from_str(&config_file)?)
        } else {
            if debug {
                println!(
                    "Config file not found in {}, using defaults",
                    config_file_location.to_string_lossy()
                );
            }
            todo!()
            // Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppAction, KeyEnum, ModeNameEnum};
    use crate::{
        app::keycommand::Keybind,
        config::{Config, ConfigIR, CONFIG_FILE_NAME},
        get_config_dir,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    const CFG_TST: &str = r#"[keybinds.global]
"+" = {action = "vol_up", value = 5}
"-" = {action = "vol_down", value = 5}
">" = "next_song"
"<" = "prev_song"
"]" = {action = "seek_forward_s", value = 5}
"[" = {action = "seek_back_s", value = 5}
F1 = {action = "toggle_help", visibility = "global"}
F10 = {action = "quit", visibility = "global"}
F12 = {action = "view_logs", visibility = "global"}
space = "pause"
C-c = "quit"

[keybinds.playlist]
F5 = {action = "playlist.view_browser", visibility = "global"}
enter.enter = "playlist.play_selected"
enter.d = "playlist.delete_selected"
enter.D = "playlist.delete_all""#;
    #[tokio::test]
    async fn test_deserialize_config_basic() {
        let config_dir = get_config_dir().unwrap();
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        let config_file = tokio::fs::read_to_string(&config_file_location)
            .await
            .unwrap();
        let x: ConfigIR = toml::from_str(&config_file).unwrap();
        let cfg = Config::try_from(x).unwrap();
        eprintln!("{:#?}", cfg)
    }
    #[tokio::test]
    async fn test_deserialize_config_special_enums() {
        let text = "browser.view_playlist";
        let expected = AppAction::Browser(crate::config::BrowserAction::ViewPlaylist);
        let actual = AppAction::try_from(text.to_string()).unwrap();
        assert_eq!(expected, actual)
    }
}
