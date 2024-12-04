use crate::app::component::actionhandler::{Action, ActionHandler};
use async_callback_manager::AsyncTask;
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Serialize,
};

use super::{
    browser::{
        artistalbums::{
            albumsongs::{BrowserSongsAction, FilterAction, SortAction},
            artistsearch::{BrowserArtistsAction, BrowserSearchAction},
        },
        BrowserAction,
    },
    logger::LoggerAction,
    playlist::PlaylistAction,
    HelpMenu, YoutuiWindow,
};

#[derive(Clone, Copy, PartialEq, Default, Debug, Serialize, Deserialize)]
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
    TextEntry(TextEntryAction),
    List(ListAction),
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelpAction {
    Close,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListAction {
    Up,
    Down,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEntryAction {
    Submit,
    Left,
    Right,
    Backspace,
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
            AppAction::TextEntry(_) => todo!(),
            AppAction::List(_) => todo!(),
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
            AppAction::TextEntry(_) => todo!(),
            AppAction::List(_) => todo!(),
        }
    }
}

impl TryFrom<String> for AppAction {
    type Error = String;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        fn deserialize_enum<T: DeserializeOwned>(s: String) -> std::result::Result<T, String> {
            Deserialize::deserialize(de::value::StringDeserializer::<serde_json::Error>::new(s))
                .map_err(|e| e.to_string())
        }
        let mut vec = value
            .split('.')
            .take(3)
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        if vec.len() >= 3 {
            return Err(format!(
                "Action {value} had too many subscripts, expected 1 max"
            ));
        };
        if vec.is_empty() {
            return Err("Action was empty!".to_string());
        };
        let back = vec.pop().expect("Length checked above");
        let front = vec.pop();
        if let Some(tag) = front {
            match tag.as_str() {
                "playlist" => Ok(AppAction::Playlist(deserialize_enum(back)?)),
                "browser" => Ok(AppAction::Browser(deserialize_enum(back)?)),
                "browser_artists" => Ok(AppAction::BrowserArtists(deserialize_enum(back)?)),
                "browser_songs" => Ok(AppAction::BrowserSongs(deserialize_enum(back)?)),
                "browser_search" => Ok(AppAction::BrowserSearch(deserialize_enum(back)?)),
                "log" => Ok(AppAction::Log(deserialize_enum(back)?)),
                "help" => Ok(AppAction::Help(deserialize_enum(back)?)),
                "filter" => Ok(AppAction::Filter(deserialize_enum(back)?)),
                "sort" => Ok(AppAction::Sort(deserialize_enum(back)?)),
                "text_entry" => Ok(AppAction::TextEntry(deserialize_enum(back)?)),
                "list" => Ok(AppAction::List(deserialize_enum(back)?)),
                _ => Err(format!("Unrecognised action category {tag}")),
            }
        } else {
            deserialize_enum(back)
        }
    }
}

impl Action for HelpAction {
    type State = HelpMenu;
    fn context(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::Close => "Help".into(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match self {
            HelpAction::Close => "Close Help".into(),
        }
    }
}
impl ActionHandler<HelpAction> for HelpMenu {
    async fn apply_action(
        &mut self,
        action: HelpAction,
    ) -> crate::app::component::actionhandler::ComponentEffect<Self> {
        match action {
            HelpAction::Close => self.shown = false,
        }
        AsyncTask::new_no_op()
    }
}
