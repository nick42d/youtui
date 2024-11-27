use crate::app::component::actionhandler::Action;
use crate::app::keycommand::CommandVisibility;
use crate::app::keycommand::Keybind;
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
use itertools::Itertools;
use serde::de;
use serde::de::MapAccess;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::time::Duration;
use tracing::info;
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
pub struct ConfigIR {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymapIR,
    pub mode_names: YoutuiModeNames,
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
    #[serde(deserialize_with = "string_or_struct")]
    Key(KeyEnumKey<String>),
    Mode(HashMap<Keybind, KeyEnumString>),
}

#[derive(Debug)]
pub enum KeyEnum<A: Default> {
    Key(KeyEnumKey<A>),
    Mode(HashMap<Keybind, KeyEnum<A>>),
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

fn string_or_struct<'de, T, D>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Infallible>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);
    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Infallible>,
    {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }
        fn visit_str<E>(self, value: &str) -> std::result::Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }
        fn visit_map<M>(self, map: M) -> std::result::Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum ModeNameEnum {
    Submode(HashMap<Keybind, ModeNameEnum>),
    #[serde(untagged)]
    Name(String),
}

impl Action for AppAction {
    type State = YoutuiWindow;
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            AppAction::VolUp
            | AppAction::VolDown
            | AppAction::NextSong
            | AppAction::PrevSong
            | AppAction::SeekForwardS
            | AppAction::SeekBackS
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
            AppAction::VolUp => format!("Vol Up 5").into(),
            AppAction::VolDown => format!("Vol Down 5").into(),
            AppAction::ToggleHelp => "Toggle Help".into(),
            AppAction::ViewLogs => "View Logs".into(),
            AppAction::SeekForwardS => format!("Seek Forward 5s").into(),
            AppAction::SeekBackS => format!("Seek Back 5s").into(),
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
            AppAction::VolUp => return state.handle_increase_volume(5).await,
            AppAction::VolDown => return state.handle_increase_volume(-5).await,
            AppAction::NextSong => return state.handle_next(),
            AppAction::PrevSong => return state.handle_prev(),
            AppAction::SeekForwardS => {
                return state.handle_seek(Duration::from_secs(5 as u64), SeekDirection::Forward)
            }
            AppAction::SeekBackS => {
                return state.handle_seek(Duration::from_secs(5 as u64), SeekDirection::Back)
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
                    .map(|this: &mut Self::State| &mut this.browser)
                    .apply(state)
                    .await
            }
            AppAction::Sort(a) => {
                return a
                    .map(|this: &mut Self::State| &mut this.browser)
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
                    .map(|this: &mut Self::State| &mut this.browser)
                    .apply(state)
                    .await
            }
        };
        AsyncTask::new_no_op()
    }
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppAction {
    #[default]
    Quit,
    VolUp,
    VolDown,
    NextSong,
    PrevSong,
    SeekForwardS,
    SeekBackS,
    ToggleHelp,
    ViewLogs,
    Pause,
    Browser(BrowserAction),
    Filter(FilterAction),
    Sort(SortAction),
    Help(HelpAction),
    BrowserArtists(BrowserArtistsAction),
    BrowserSearch(BrowserSearchAction),
    BrowserSongs(BrowserSongsAction),
    Log(LoggerAction),
    Playlist(PlaylistAction),
}
impl From<String> for AppAction {
    fn from(value: String) -> Self {
        let mut vec = value
            .split('.')
            .take(3)
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        if vec.len() >= 3 {
            // return Err(format!(
            //     "Action {value} had too many subscripts, expected 1 max"
            // ));
            panic!("Action {value} had too many subscripts, expected 1 max")
        };
        if vec.len() == 0 {
            // return Err(format!("Action was empty!"));
            panic!("Action was empty!")
        };
        let back = vec.pop().expect("Length checked above");
        let front = vec.pop();
        if let Some(tag) = front {
            match tag.as_str() {
                "playlist" => return AppAction::Playlist(PlaylistAction::ViewBrowser),
                _ => todo!(),
            }
        } else {
            Deserialize::deserialize(de::value::StringDeserializer::<serde_json::Error>::new(
                back,
            ))
            .unwrap()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaylistAction {
    #[serde(rename = "playlist.view_browser")]
    ViewBrowser,
    #[serde(rename = "playlist.play_selected")]
    PlaySelected,
    #[serde(rename = "playlist.delete_selected")]
    DeleteSelected,
    #[serde(rename = "playlist.delete_all")]
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
                ListAction::Up(n) => "Up",
                ListAction::Down(n) => "Down",
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
                ListAction::Up(n) => state.album_songs_list.increment_list(-1),
                ListAction::Down(n) => state.album_songs_list.increment_list(1),
            },
            BrowserSongsAction::Sort => state.album_songs_list.handle_pop_sort(),
            BrowserSongsAction::Filter => state.album_songs_list.toggle_filter(),
        }
        AsyncTask::new_no_op()
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
    Close,
    Clear,
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
            FilterAction::Clear => "Clear filter",
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
            FilterAction::Clear => state.album_songs_list.clear_filter(),
        };
        AsyncTask::new_no_op()
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
    type State = Browser;
    fn context(&self) -> std::borrow::Cow<str> {
        "Filter".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            SortAction::CloseSort => "Close sort",
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
            SortAction::CloseSort => state.album_songs_list.close_sort(),
            SortAction::ClearSort => state.album_songs_list.handle_clear_sort(),
        }
        AsyncTask::new_no_op()
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
        config::{ConfigIR, CONFIG_FILE_NAME},
        get_config_dir,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    const CFG_TST: &str = r#"[global]
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

[playlist]
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
    }
    #[tokio::test]
    async fn test_deserialize_config_special_enums() {
        let x: ConfigIR = toml::from_str(CFG_TST).unwrap();
        eprintln!("{:#?}", x);
    }
}
