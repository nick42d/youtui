use super::server::album_art_downloader::AlbumArt;
use super::server::song_downloader::InMemSong;
use super::view::SortDirection;
use itertools::Itertools;
use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use ytmapi_rs::common::{Explicit, Thumbnail, VideoID};
use ytmapi_rs::parse::{AlbumSong, ParsedSongAlbum, ParsedSongArtist, SearchResultSong};

pub trait SongListComponent {
    fn get_song_from_idx(&self, idx: usize) -> Option<&ListSong>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaybeRc<T> {
    Rc(Rc<T>),
    Owned(T),
}
impl<T> Deref for MaybeRc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            MaybeRc::Rc(rc) => rc.deref(),
            MaybeRc::Owned(t) => t,
        }
    }
}
impl<T> AsRef<T> for MaybeRc<T> {
    fn as_ref(&self) -> &T {
        match self {
            MaybeRc::Rc(rc) => rc,
            MaybeRc::Owned(t) => t,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlbumSongsList {
    pub state: ListStatus,
    list: Vec<ListSong>,
    pub next_id: ListSongID,
}

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, PartialOrd)]
pub struct ListSongID(#[cfg(test)] pub usize, #[cfg(not(test))] usize);

// As this is a simple wrapper type we implement Copy for ease of handling
#[derive(Clone, PartialEq, Copy, Debug, Default, PartialOrd)]
pub struct Percentage(pub u8);

#[derive(Clone, Debug, PartialEq)]
pub struct ListSong {
    pub video_id: VideoID<'static>,
    pub track_no: Option<usize>,
    pub plays: String,
    pub title: String,
    pub explicit: Explicit,
    pub download_status: DownloadStatus,
    pub id: ListSongID,
    pub duration_string: String,
    pub actual_duration: Option<Duration>,
    pub year: Option<Rc<String>>,
    pub album_art: Option<Rc<AlbumArt>>,
    pub artists: MaybeRc<Vec<ParsedSongArtist>>,
    pub thumbnails: MaybeRc<Vec<Thumbnail>>,
    pub album: Option<MaybeRc<ParsedSongAlbum>>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ListSongDisplayableField {
    DownloadStatus,
    TrackNo,
    Artists,
    Album,
    Song,
    Duration,
    Year,
    Plays,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ListStatus {
    New,
    Loading,
    InProgress,
    Loaded,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadStatus {
    None,
    Queued,
    Downloading(Percentage),
    Downloaded(Arc<InMemSong>),
    Failed,
    Retrying { times_retried: usize },
}

#[derive(Clone, Debug, PartialEq)]
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
    pub fn get_track_no(&self) -> Option<usize> {
        self.track_no
    }
    pub fn get_fields<const N: usize>(
        &self,
        fields: [ListSongDisplayableField; N],
    ) -> [Cow<'_, str>; N] {
        fields.map(|field| self.get_field(field))
    }
    pub fn get_field(&self, field: ListSongDisplayableField) -> Cow<'_, str> {
        match field {
            ListSongDisplayableField::DownloadStatus =>
            // Type annotation to help rust compiler
            {
                Cow::from(match self.download_status {
                    DownloadStatus::Downloading(p) => {
                        format!("{}[{}]%", self.download_status.list_icon(), p.0)
                    }
                    DownloadStatus::Retrying { times_retried } => {
                        format!("{}[x{}]", self.download_status.list_icon(), times_retried)
                    }
                    _ => self.download_status.list_icon().to_string(),
                })
            }
            ListSongDisplayableField::TrackNo => self
                .get_track_no()
                .map(|track_no| track_no.to_string())
                .unwrap_or_default()
                .into(),
            ListSongDisplayableField::Artists => Itertools::intersperse(
                self.artists
                    .as_ref()
                    .iter()
                    .map(|artist| artist.name.as_str()),
                ", ",
            )
            .collect::<String>()
            .into(),
            ListSongDisplayableField::Album => self
                .album
                .as_ref()
                .map(|album| album.as_ref().name.as_str())
                .unwrap_or_default()
                .into(),
            ListSongDisplayableField::Year => self
                .year
                .as_ref()
                .map(|year| year.as_str())
                .unwrap_or_default()
                .into(),
            ListSongDisplayableField::Song => self.title.as_str().into(),
            ListSongDisplayableField::Duration => self.duration_string.as_str().into(),
            ListSongDisplayableField::Plays => self.plays.as_str().into(),
        }
    }
}

impl Default for AlbumSongsList {
    fn default() -> Self {
        AlbumSongsList {
            state: ListStatus::New,
            list: Vec::new(),
            next_id: ListSongID(0),
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
    pub fn sort(&mut self, field: ListSongDisplayableField, direction: SortDirection) {
        self.list.sort_by(|a, b| match direction {
            SortDirection::Asc => a
                .get_field(field)
                .partial_cmp(&b.get_field(field))
                .unwrap_or(std::cmp::Ordering::Equal),
            SortDirection::Desc => b
                .get_field(field)
                .partial_cmp(&a.get_field(field))
                .unwrap_or(std::cmp::Ordering::Equal),
        });
    }
    pub fn clear(&mut self) {
        // We can't reset the ID, so it's left out and we'll keep incrementing.
        self.state = ListStatus::New;
        self.list.clear();
    }
    // Naive implementation
    pub fn append_raw_album_songs(
        &mut self,
        raw_list: Vec<AlbumSong>,
        album: ParsedSongAlbum,
        year: String,
        artists: Vec<ParsedSongArtist>,
        thumbnails: Vec<Thumbnail>,
    ) {
        // The album is shared by all the songs.
        // So no need to clone/allocate for eache one.
        // Instead we'll share ownership via Rc.
        let album = Rc::new(album);
        let year = Rc::new(year);
        let artists = Rc::new(artists);
        let thumbnails = Rc::new(thumbnails);
        for song in raw_list {
            self.add_raw_album_song(
                song,
                album.clone(),
                year.clone(),
                artists.clone(),
                thumbnails.clone(),
            );
        }
    }
    // Naive implementation
    pub fn append_raw_search_result_songs(&mut self, raw_list: Vec<SearchResultSong>) {
        for song in raw_list {
            self.add_raw_search_result_song(song);
        }
    }
    pub fn add_raw_album_song(
        &mut self,
        song: AlbumSong,
        album: Rc<ParsedSongAlbum>,
        year: Rc<String>,
        artists: Rc<Vec<ParsedSongArtist>>,
        thumbnails: Rc<Vec<Thumbnail>>,
    ) -> ListSongID {
        let id = self.create_next_id();
        let AlbumSong {
            video_id,
            track_no,
            duration,
            plays,
            title,
            explicit,
            ..
        } = song;
        self.list.push(ListSong {
            download_status: DownloadStatus::None,
            id,
            year: Some(year),
            artists: MaybeRc::Rc(artists),
            album: Some(MaybeRc::Rc(album)),
            actual_duration: None,
            video_id,
            track_no: Some(track_no),
            plays,
            title,
            explicit,
            duration_string: duration,
            thumbnails: MaybeRc::Rc(thumbnails),
            album_art: None,
        });
        id
    }
    pub fn add_raw_search_result_song(&mut self, song: SearchResultSong) -> ListSongID {
        let id = self.create_next_id();
        let SearchResultSong {
            title,
            artist,
            album,
            duration,
            plays,
            explicit,
            video_id,
            thumbnails,
            ..
        } = song;
        self.list.push(ListSong {
            download_status: DownloadStatus::None,
            id,
            year: None,
            artists: MaybeRc::Owned(vec![ParsedSongArtist {
                name: artist,
                id: None,
            }]),
            album: album.map(MaybeRc::Owned),
            actual_duration: None,
            video_id,
            track_no: None,
            plays,
            title,
            explicit,
            duration_string: duration,
            thumbnails: MaybeRc::Owned(thumbnails),
            album_art: None,
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
        let id = self.next_id;
        self.next_id.0 += 1;
        id
    }
    pub fn update_album_art(&mut self, album_art: AlbumArt) {
        let shared = Rc::new(album_art);
        for song in &mut self.list {
            if song.album_art.is_none()
                && song
                    .album
                    .as_ref()
                    .is_some_and(|album| album.id == shared.album_id)
            {
                song.album_art = Some(shared.clone());
            }
            tracing::info!("Album art updated");
        }
    }
}
