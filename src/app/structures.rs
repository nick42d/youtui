use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;
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
    Failed, // Should keep track of times failed
}

#[derive(Clone, Debug)]
pub enum PlayState {
    NotPlaying,
    Playing(ListSongID),
    Transitioning,
    Paused(ListSongID),
    Stopped,
    Buffering(ListSongID),
}

impl PlayState {
    pub fn transition_to_paused(self) -> Self {
        match self {
            Self::NotPlaying => Self::NotPlaying,
            Self::Stopped => Self::Stopped,
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
            Self::Stopped => Self::Stopped,
            Self::Playing(id) => Self::Stopped,
            Self::Buffering(id) => Self::Stopped,
            Self::Paused(id) => {
                warn!("Stopping from Paused status - seems unusual");
                Self::Stopped
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
            PlayState::Stopped => '',
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
    pub fn get_fields_iter(&self) -> TableItem {
        Box::new(
            [
                // Type annotation to help rust compiler
                Cow::from(match self.download_status {
                    DownloadStatus::Downloading(p) => {
                        format!("{}[{}]%", self.download_status.list_icon(), p.0)
                    }
                    _ => self.download_status.list_icon().to_string(),
                }),
                self.get_track_no().to_string().into(),
                // TODO: Remove allocation
                self.get_artists()
                    .get(0)
                    .map(|a| a.to_string())
                    .unwrap_or_default()
                    .into(),
                self.get_album().into(),
                self.get_title().into(),
                // TODO: Remove allocation
                self.get_duration()
                    .as_ref()
                    .map(|d| d.as_str())
                    .unwrap_or("")
                    .into(),
                self.get_year().into(),
            ]
            .into_iter(),
        )
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
        if self.cur_selected == Some(0) || self.cur_selected == Some(self.list.len() - 1) {
            self.offset_commands.clear();
            // Safe to unwrap, checked above.
            self.offset_commands
                .push(self.cur_selected.unwrap() as isize);
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
    // TODO: Docs and tests.
    fn get_offset(&self, height: usize) -> usize {
        let (offset, _): (usize, usize) = self
            .offset_commands
            .iter()
            // XXX: cursor is stored in self if we want to avoid using fold state for it also.
            .fold((0, 0), |(offset, cursor), e| {
                let new_cur = cursor.saturating_add_signed(*e);
                let new_offset = if new_cur.saturating_sub(offset) <= 0 {
                    new_cur
                } else if new_cur.saturating_sub(offset) > height {
                    new_cur.saturating_sub(height)
                } else {
                    offset
                };

                (new_offset, new_cur)
            });
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
