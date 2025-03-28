use super::server::downloader::InMemSong;
use super::view::{SortDirection, TableItem};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use ytmapi_rs::common::AlbumID;
use ytmapi_rs::parse::AlbumSong;

pub trait SongListComponent {
    fn get_song_from_idx(&self, idx: usize) -> Option<&ListSong>;
}

#[derive(Clone)]
pub struct AlbumSongsList {
    pub state: ListStatus,
    list: Vec<ListSong>,
    pub next_id: ListSongID,
}

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct ListSongID(usize);

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct Percentage(pub u8);

#[derive(Clone, Debug)]
pub struct ListSong {
    pub raw: AlbumSong,
    pub download_status: DownloadStatus,
    pub id: ListSongID,
    pub actual_duration: Option<Duration>,
    pub year: Rc<String>,
    pub artists: Vec<Rc<String>>,
    pub album: Rc<String>,
    pub album_id: Rc<AlbumID<'static>>,
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
    Downloaded(Arc<InMemSong>),
    Failed,
    Retrying { times_retried: usize },
}

#[derive(Clone, Debug)]
pub enum PlayState {
    NotPlaying,
    Playing(ListSongID),
    Paused(ListSongID),
    // May be the same as NotPlaying?
    Stopped,
    Error(ListSongID),
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
            PlayState::Error(_) => '',
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
            Self::Retrying { .. } => '',
        }
    }
}

impl ListSong {
    pub fn get_track_no(&self) -> usize {
        self.raw.track_no
    }
    pub fn get_fields_iter(&self) -> TableItem {
        Box::new(
            [
                // Type annotation to help rust compiler
                Cow::from(match self.download_status {
                    DownloadStatus::Downloading(p) => {
                        format!("{}[{}]%", self.download_status.list_icon(), p.0)
                    }
                    DownloadStatus::Retrying { times_retried } => {
                        format!("{}[x{}]", self.download_status.list_icon(), times_retried)
                    }
                    _ => self.download_status.list_icon().to_string(),
                }),
                self.get_track_no().to_string().into(),
                // TODO: Remove allocation
                self.artists
                    .first()
                    .map(|a| a.to_string())
                    .unwrap_or_default()
                    .into(),
                self.album.as_ref().into(),
                (&self.raw.title).into(),
                // TODO: Remove allocation
                (&self.raw.duration).into(),
                self.year.as_ref().into(),
            ]
            .into_iter(),
        )
    }
}

impl Default for AlbumSongsList {
    fn default() -> Self {
        AlbumSongsList {
            state: ListStatus::New,
            list: Vec::new(),
            next_id: ListSongID::default(),
        }
    }
}

impl AlbumSongsList {
    pub fn get_list_iter(&self) -> std::slice::Iter<ListSong> {
        self.list.iter()
    }
    pub fn get_list_iter_mut(&mut self) -> std::slice::IterMut<ListSong> {
        self.list.iter_mut()
    }
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
    }
    // Naive implementation
    pub fn append_raw_songs(
        &mut self,
        raw_list: Vec<AlbumSong>,
        album: String,
        album_id: AlbumID<'static>,
        year: String,
        artist: String,
    ) {
        // The album is shared by all the songs.
        // So no need to clone/allocate for eache one.
        // Instead we'll share ownership via Rc.
        let album = Rc::new(album);
        let album_id = Rc::new(album_id);
        let year = Rc::new(year);
        let artist = Rc::new(artist);
        for song in raw_list {
            self.add_raw_song(
                song,
                album.clone(),
                album_id.clone(),
                year.clone(),
                artist.clone(),
            );
        }
    }
    pub fn add_raw_song(
        &mut self,
        song: AlbumSong,
        album: Rc<String>,
        album_id: Rc<AlbumID<'static>>,
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
            album_id,
            actual_duration: None,
        });
        id
    }
    // Returns the ID of the first song added.
    pub fn push_song_list(&mut self, mut song_list: Vec<ListSong>) -> ListSongID {
        let first_id = self.create_next_id();
        if let Some(song) = song_list.first_mut() {
            song.id = first_id;
        };
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
        Some(self.list.remove(idx))
    }
    pub fn create_next_id(&mut self) -> ListSongID {
        self.next_id.0 += 1;
        self.next_id
    }
}
