use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;
use ytmapi_rs::common::youtuberesult::{ResultCore, YoutubeResult};
use ytmapi_rs::parse::SongResult;

use super::view::{Scrollable, SortDirection, TableItem, TableSortCommand};

#[derive(Clone)]
pub struct AlbumSongsList {
    pub state: ListStatus,
    pub list: Vec<ListSong>,
    pub next_id: ListSongID,
    pub cur_selected: Option<usize>,
}

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct ListSongID(usize);

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct Percentage(pub u8);

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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
    Paused(ListSongID),
    // May be the same as NotPlaying?
    Stopped,
    Buffering(ListSongID),
}

impl PlayState {
    pub fn list_icon(&self) -> char {
        match self {
            PlayState::Buffering(_) => '',
            PlayState::NotPlaying => '',
            PlayState::Playing(_) => '',
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
    fn _set_year(&mut self, year: Rc<String>) {
        self.year = year;
    }
    fn _set_album(&mut self, album: Rc<String>) {
        self.album = album;
    }
    pub fn get_year(&self) -> &String {
        &self.year
    }
    fn _set_artists(&mut self, artists: Vec<Rc<String>>) {
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
    fn increment_list(&mut self, amount: isize) {
        // Naive
        self.cur_selected = Some(
            self.cur_selected
                .unwrap_or(0)
                .checked_add_signed(amount)
                .unwrap_or(0)
                .min(self.list.len().checked_add_signed(-1).unwrap_or(0)),
        );
    }
    fn get_selected_item(&self) -> usize {
        self.cur_selected.unwrap_or_default()
    }
}

impl Default for AlbumSongsList {
    fn default() -> Self {
        AlbumSongsList {
            state: ListStatus::New,
            cur_selected: None,
            list: Vec::new(),
            next_id: ListSongID::default(),
        }
    }
}

impl AlbumSongsList {
    pub fn sort(&mut self, column: usize, direction: SortDirection) {
        self.list.sort_by(|a, b| match direction {
            SortDirection::Asc => a
                .get_fields_iter()
                .nth(column)
                .partial_cmp(&b.get_fields_iter().nth(column))
                .unwrap_or(std::cmp::Ordering::Equal),
            SortDirection::Desc => b
                .get_fields_iter()
                .nth(column)
                .partial_cmp(&a.get_fields_iter().nth(column))
                .unwrap_or(std::cmp::Ordering::Equal),
        });
    }
    pub fn clear(&mut self) {
        // We can't reset the ID, so it's left out and we'll keep incrementing.
        self.state = ListStatus::New;
        self.list.clear();
        self.cur_selected = None;
    }
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
        // Set a current selected index if we haven't already got one
        // so that we can start using commands right away.
        if !song_list.is_empty() && self.cur_selected.is_none() {
            self.cur_selected = Some(0);
        }
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
    /// Safely deletes the song at index if it exists, and returns it.
    pub fn remove_song_index(&mut self, idx: usize) -> Option<ListSong> {
        // Guard against index out of bounds
        if self.list.len() <= idx {
            return None;
        }
        // If we are removing a song at a position less than current index, decrement current index.
        if let Some(cur_idx) = self.cur_selected {
            // NOTE: Ok to simply take, if list only had one element.
            if cur_idx >= idx && idx != 0 {
                // Safe, as checked above that cur_idx >= 0
                self.cur_selected = Some(cur_idx - 1);
            }
        }
        Some(self.list.remove(idx))
    }
    pub fn create_next_id(&mut self) -> ListSongID {
        self.next_id.0 += 1;
        self.next_id
    }
}
