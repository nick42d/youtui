use super::HelpMenu;
use super::browser::BrowserAction;
use super::browser::artistsearch::search_panel::BrowserArtistsAction;
use super::browser::artistsearch::songs_panel::BrowserArtistSongsAction;
use super::browser::shared_components::{BrowserSearchAction, FilterAction, SortAction};
use super::browser::songsearch::BrowserSongsAction;
use super::logger::LoggerAction;
use super::playlist::PlaylistAction;
use crate::app::component::actionhandler::{Action, ActionHandler, YoutuiEffect};
use crate::app::ui::browser::playlistsearch::search_panel::BrowserPlaylistsAction;
use crate::app::ui::browser::playlistsearch::songs_panel::BrowserPlaylistSongsAction;
use anyhow::bail;
use async_callback_manager::AsyncTask;
use serde::de::{self};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const VOL_TICK: i8 = 5;
pub const SEEK_AMOUNT: Duration = Duration::from_secs(5);
pub const PAGE_KEY_LINES: isize = 10;

#[derive(Clone, Copy, PartialEq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppAction {
    #[default]
    Quit,
    VolUp,
    VolDown,
    NextSong,
    PrevSong,
    SeekForward,
    SeekBack,
    ToggleHelp,
    ViewLogs,
    PlayPause,
    NoOp,
    Browser(BrowserAction),
    Filter(FilterAction),
    Sort(SortAction),
    Help(HelpAction),
    BrowserArtists(BrowserArtistsAction),
    BrowserPlaylists(BrowserPlaylistsAction),
    BrowserSearch(BrowserSearchAction),
    BrowserArtistSongs(BrowserArtistSongsAction),
    BrowserPlaylistSongs(BrowserPlaylistSongsAction),
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
    PageUp,
    PageDown,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEntryAction {
    Submit,
    Left,
    Right,
    Backspace,
}

impl Action for TextEntryAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        "Global".into()
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            TextEntryAction::Submit => "Submit".into(),
            TextEntryAction::Left => "Left".into(),
            TextEntryAction::Right => "Right".into(),
            TextEntryAction::Backspace => "Backspace".into(),
        }
    }
}
impl Action for ListAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        "Global".into()
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            ListAction::Up => "List Up".into(),
            ListAction::Down => "List Down".into(),
            ListAction::PageUp => "List PageUp".into(),
            ListAction::PageDown => "List PageDown".into(),
        }
    }
}

impl Action for AppAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        match self {
            AppAction::VolUp
            | AppAction::VolDown
            | AppAction::NextSong
            | AppAction::PrevSong
            | AppAction::SeekForward
            | AppAction::SeekBack
            | AppAction::ToggleHelp
            | AppAction::Quit
            | AppAction::ViewLogs
            | AppAction::NoOp
            | AppAction::PlayPause => "Global".into(),
            AppAction::Log(a) => a.context(),
            AppAction::Playlist(a) => a.context(),
            AppAction::Browser(a) => a.context(),
            AppAction::Filter(a) => a.context(),
            AppAction::Sort(a) => a.context(),
            AppAction::Help(a) => a.context(),
            AppAction::BrowserArtists(a) => a.context(),
            AppAction::BrowserSearch(a) => a.context(),
            AppAction::BrowserArtistSongs(a) => a.context(),
            AppAction::TextEntry(a) => a.context(),
            AppAction::List(a) => a.context(),
            AppAction::BrowserSongs(a) => a.context(),
            AppAction::BrowserPlaylists(browser_playlists_action) => todo!(),
            AppAction::BrowserPlaylistSongs(browser_playlist_songs_action) => todo!(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            AppAction::Quit => "Quit".into(),
            AppAction::PrevSong => "Prev Song".into(),
            AppAction::NextSong => "Next Song".into(),
            AppAction::PlayPause => "Pause".into(),
            AppAction::VolUp => format!("Vol Up {VOL_TICK}").into(),
            AppAction::VolDown => format!("Vol Down {VOL_TICK}").into(),
            AppAction::ToggleHelp => "Toggle Help".into(),
            AppAction::ViewLogs => "View Logs".into(),
            AppAction::SeekForward => format!("Seek Forward {}s", SEEK_AMOUNT.as_secs()).into(),
            AppAction::SeekBack => format!("Seek Back {}s", SEEK_AMOUNT.as_secs()).into(),
            AppAction::NoOp => "No Operation".into(),
            AppAction::Log(a) => a.describe(),
            AppAction::Playlist(a) => a.describe(),
            AppAction::Browser(a) => a.describe(),
            AppAction::Filter(a) => a.describe(),
            AppAction::Sort(a) => a.describe(),
            AppAction::Help(a) => a.describe(),
            AppAction::BrowserArtists(a) => a.describe(),
            AppAction::BrowserSearch(a) => a.describe(),
            AppAction::BrowserArtistSongs(a) => a.describe(),
            AppAction::TextEntry(a) => a.describe(),
            AppAction::List(a) => a.describe(),
            AppAction::BrowserSongs(a) => a.describe(),
            AppAction::BrowserPlaylists(browser_playlists_action) => todo!(),
            AppAction::BrowserPlaylistSongs(browser_playlist_songs_action) => todo!(),
        }
    }
}

impl TryFrom<String> for AppAction {
    type Error = anyhow::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let mut vec = value
            .split('.')
            .take(3)
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        if vec.len() >= 3 {
            bail!(format!(
                "Action {value} had too many subscripts, expected 1 max"
            ));
        };
        if vec.is_empty() {
            bail!("Action was empty!");
        };
        let back = vec.pop().expect("Length checked above");
        let front = vec.pop();
        if let Some(tag) = front {
            // Neat hack to turn tag.back into any of the nested enum variants.
            let json = serde_json::json!({tag : back});
            Ok(serde_json::from_value(json)?)
        } else {
            // Neat hack to turn back into any of the non-nested enum variants.
            Ok(Deserialize::deserialize(de::value::StringDeserializer::<
                serde_json::Error,
            >::new(back))?)
        }
    }
}

impl Action for HelpAction {
    fn context(&self) -> std::borrow::Cow<'_, str> {
        match self {
            HelpAction::Close => "Help".into(),
        }
    }
    fn describe(&self) -> std::borrow::Cow<'_, str> {
        match self {
            HelpAction::Close => "Close Help".into(),
        }
    }
}
impl ActionHandler<HelpAction> for HelpMenu {
    fn apply_action(&mut self, action: HelpAction) -> impl Into<YoutuiEffect<Self>> {
        match action {
            HelpAction::Close => self.shown = false,
        }
        AsyncTask::new_no_op()
    }
}
