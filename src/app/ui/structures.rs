use std::borrow::Cow;
use std::sync::Arc;
use std::{path::PathBuf, rc::Rc};
use tracing::{info, warn};
use ytmapi_rs::common::youtuberesult::{ResultCore, YoutubeResult};
use ytmapi_rs::parse::SongResult;

use super::view::{Scrollable, TableItem};

#[derive(Clone)]
pub struct AlbumSongsList {
    pub state: ListStatus,
    pub list: Vec<ListSong>,
    pub next_id: ListSongID,
    pub cur_selected: Option<usize>,
    pub offset_commands: Vec<isize>,
}

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct ListSongID(usize);

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct Percentage(pub u8);

#[derive(Clone)]
pub struct ListSong {
    pub raw: SongResult,
    pub download_status: DownloadStatus,
    pub id: ListSongID,
    year: Rc<String>,
    artists: Vec<Rc<String>>,
    album: Rc<String>,
}
#[derive(Clone)]
pub enum ListStatus {
    New,
    Loading,
    InProgress,
    Loaded,
    Error,
}

#[derive(Clone)]
pub enum DownloadStatus {
    None,
    Queued,
    Downloading(Percentage),
    Downloaded(Arc<Vec<u8>>),
    Failed,
}

#[derive(Clone, Debug)]
pub enum PlayState {
    NotPlaying,
    Playing(ListSongID),
    Transitioning,
    Paused(ListSongID),
    Stopped(ListSongID),
    Buffering(ListSongID),
}

impl PlayState {
    pub fn transition_to_paused(self) -> Self {
        match self {
            Self::NotPlaying => Self::NotPlaying,
            Self::Stopped(id) => Self::Stopped(id),
            Self::Playing(id) => Self::Paused(id),
            Self::Paused(id) => Self::Paused(id),
            Self::Buffering(id) => Self::Paused(id),
            Self::Transitioning => {
                tracing::error!("Tried to transition from transitioning state, unhandled.");
                Self::Transitioning
            }
        }
    }
    pub fn transition_to_stopped(self) -> Self {
        match self {
            Self::NotPlaying => Self::NotPlaying,
            Self::Stopped(id) => Self::Stopped(id),
            Self::Playing(id) => Self::Stopped(id),
            Self::Buffering(id) => Self::Stopped(id),
            Self::Paused(id) => {
                warn!("Stopping from Paused status - seems unusual");
                Self::Stopped(id)
            }
            Self::Transitioning => {
                tracing::error!("Tried to transition from transitioning state, unhandled.");
                Self::Transitioning
            }
        }
    }
    pub fn take_whilst_transitioning(&mut self) -> Self {
        let temp = Self::Transitioning;
        std::mem::replace(self, temp)
    }
    pub fn list_icon(&self) -> char {
        match self {
            PlayState::Buffering(_) => '',
            PlayState::NotPlaying => '',
            PlayState::Playing(_) => '',
            PlayState::Transitioning => '',
            PlayState::Paused(_) => '',
            PlayState::Stopped(_) => '',
        }
    }
}

impl DownloadStatus {
    pub fn list_icon(&self) -> char {
        match self {
            Self::Failed => '',
            Self::Queued => '',
            Self::None => ' ',
            Self::Downloading(_) => '',
            Self::Downloaded(_) => '',
        }
    }
}

impl ListSong {
    fn set_year(&mut self, year: Rc<String>) {
        self.year = year;
    }
    fn set_album(&mut self, album: Rc<String>) {
        self.album = album;
    }
    pub fn get_year(&self) -> &String {
        &self.year
    }
    fn set_artists(&mut self, artists: Vec<Rc<String>>) {
        self.artists = artists;
    }
    pub fn get_artists(&self) -> &Vec<Rc<String>> {
        &self.artists
    }
    pub fn get_album(&self) -> &String {
        &self.album
    }
    pub fn get_track_no(&self) -> usize {
        self.raw.get_track_no()
    }
}

impl<'a> TableItem for ListSong {
    fn get_field(&self, index: usize) -> Option<Cow<'_, str>> {
        match index {
            0 => Some(
                match self.download_status {
                    DownloadStatus::Downloading(p) => {
                        format!("{}[{}]%", self.download_status.list_icon(), p.0)
                    }
                    _ => self.download_status.list_icon().to_string(),
                }
                .into(),
            ),
            1 => Some(self.get_track_no().to_string().into()),
            2 => Some(
                // TODO: Remove allocation
                self.get_artists()
                    .get(0)
                    .map(|a| a.to_string())
                    .unwrap_or_default()
                    .into(),
            ),
            3 => Some(self.get_album().into()),
            4 => Some(self.get_title().into()),
            5 => self.get_duration().as_ref().map(|s| s.into()),
            6 => Some(self.get_year().into()),
            _ => None,
        }
    }
    fn len(&self) -> usize {
        6
    }
}

impl YoutubeResult for ListSong {
    fn get_core(&self) -> &ResultCore {
        self.raw.get_core()
    }
}

impl Scrollable for AlbumSongsList {
    fn get_selected_item(&self) -> usize {
        self.cur_selected.unwrap_or(0)
    }
    fn increment_list(&mut self, amount: isize) {
        // Naive
        self.cur_selected = Some(
            self.cur_selected
                .unwrap_or(0)
                .checked_add_signed(amount)
                .unwrap_or(0)
                .min(self.list.len().checked_add_signed(-1).unwrap_or(0)),
        );
        // TODO: Don't clear when we get to the bottom, instead should be vec![self.list.len()-1]
        if self.cur_selected == Some(0) || self.cur_selected == Some(self.list.len() - 1) {
            self.offset_commands.clear();
            return;
        }
        if let Some(n) = self.offset_commands.pop() {
            if n.signum() == amount.signum() {
                self.offset_commands.push(n + amount);
            } else {
                self.offset_commands.push(n);
                self.offset_commands.push(amount);
            }
        } else {
            self.offset_commands.push(amount);
        }
    }
    /// Compute the offset using the offset commands.
    // Seems to work how I was expecting, however offset doesn't work as expected.
    // TODO: investigate.
    fn get_offset(&self, height: usize) -> usize {
        info!("cmds: {:#?}", &self.offset_commands);
        let offset: usize = self
            .offset_commands
            .iter()
            .fold(0, |acc, e| (acc.saturating_add_signed(*e).min(height)));
        info!("offset: {offset}");
        offset
    }
}

impl Default for AlbumSongsList {
    fn default() -> Self {
        AlbumSongsList {
            state: ListStatus::New,
            list: Vec::new(),
            next_id: ListSongID::default(),
            cur_selected: None,
            offset_commands: Default::default(),
        }
    }
}

impl AlbumSongsList {
    // Naive implementation
    pub fn append_raw_songs(
        &mut self,
        raw_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
    ) {
        // The album is shared by all the songs.
        // So no need to clone/allocate for eache one.
        // Instead we'll share ownership via Rc.
        let album = Rc::new(album);
        let year = Rc::new(year);
        let artist = Rc::new(artist);
        for song in raw_list {
            self.add_raw_song(song, album.clone(), year.clone(), artist.clone());
        }
    }
    pub fn add_raw_song(
        &mut self,
        song: SongResult,
        album: Rc<String>,
        year: Rc<String>,
        artist: Rc<String>,
    ) -> ListSongID {
        let id = self.create_next_id();
        self.list.push(ListSong {
            raw: song,
            download_status: DownloadStatus::None,
            id,
            year,
            artists: vec![artist],
            album,
        });
        id
    }
    // Returns the ID of the first song added.
    pub fn push_song_list(&mut self, mut song_list: Vec<ListSong>) -> ListSongID {
        let first_id = self.create_next_id();
        song_list.first_mut().map(|song| song.id = first_id);
        // XXX: Below panics - consider a better option.
        self.list.push(song_list.remove(0));
        for mut song in song_list {
            song.id = self.create_next_id();
            self.list.push(song);
        }
        first_id
    }
    pub fn push_clone_listsong(&mut self, song: &ListSong) -> ListSongID {
        let mut cloned_song = song.clone();
        let id = self.create_next_id();
        cloned_song.id = id;
        self.list.push(cloned_song);
        id
    }
    pub fn create_next_id(&mut self) -> ListSongID {
        self.next_id.0 += 1;
        self.next_id
    }
}
