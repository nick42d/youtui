//! Wrapper for souvlaki::MediaControls that performs diffing to ensure OS calls
//! are made at a minimum (in line with immediate mode architecture principle)
use futures::Stream;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::{borrow::Cow, fs::Metadata, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use super::{structures::PlayState, ui::YoutuiWindow};

pub struct MediaController {
    inner: MediaControls,
    status: MediaControlsStatus,
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    cover_url: Option<String>,
    duration: Option<Duration>,
    progress: Option<Duration>,
    // TODO: Volume
}

/// CoW version of souvlaki::MediaMetadata
pub struct CowMediaMetadata<'a> {
    pub title: Option<Cow<'a, str>>,
    pub album: Option<Cow<'a, str>>,
    pub artist: Option<Cow<'a, str>>,
    pub cover_url: Option<Cow<'a, str>>,
    pub duration: Option<Duration>,
}

impl<'a> From<&'a CowMediaMetadata<'a>> for MediaMetadata<'a> {
    fn from(value: &'a CowMediaMetadata<'a>) -> Self {
        let CowMediaMetadata {
            title,
            album,
            artist,
            cover_url,
            duration,
        } = value;
        MediaMetadata {
            title: title.as_deref(),
            album: album.as_deref(),
            artist: artist.as_deref(),
            cover_url: cover_url.as_deref(),
            duration: duration.to_owned(),
        }
    }
}

#[derive(PartialEq, Eq)]
enum MediaControlsStatus {
    Stopped,
    Paused,
    Playing,
}

impl MediaController {
    pub fn new() -> (Self, impl Stream<Item = MediaControlEvent>) {
        let (tx, rx) = mpsc::channel(super::EVENT_CHANNEL_SIZE);
        let config = PlatformConfig {
            display_name: "Youtui",
            dbus_name: "youtui",
            // TODO: hwnd for windows
            hwnd: None,
        };
        let mut controls = MediaControls::new(config).unwrap();
        // Assumption - event handler runs in another thread, and blocking send is
        // acceptable.
        controls
            .attach(move |event| {
                tx.blocking_send(event).unwrap();
            })
            .unwrap();
        (
            MediaController {
                inner: controls,
                status: MediaControlsStatus::Stopped,
                title: None,
                album: None,
                artist: None,
                cover_url: None,
                duration: None,
                progress: None,
            },
            ReceiverStream::new(rx),
        )
    }
    pub fn update_controls(
        &mut self,
        (playback_status, playback_metadata): (MediaPlayback, CowMediaMetadata<'_>),
    ) {
        // TODO: Change to just in time conversion.
        let playback_metadata: MediaMetadata = (&playback_metadata).into();
        let mut redraw_playback = false;
        let mut redraw_metadata = false;
        match playback_status {
            MediaPlayback::Stopped => {
                if self.status != MediaControlsStatus::Stopped {
                    self.status = MediaControlsStatus::Stopped;
                    redraw_playback = true;
                }
            }
            MediaPlayback::Paused { progress } if self.status != MediaControlsStatus::Paused => {
                info!("Changed to paused");
                redraw_playback = true;
                self.progress = progress.map(|d| d.0);
                self.status = MediaControlsStatus::Paused;
            }
            // Fallback - already paused
            MediaPlayback::Paused { progress } => {
                if self.progress != progress.map(|d| d.0) {
                    self.progress = progress.map(|d| d.0);
                    redraw_playback = true;
                }
            }
            MediaPlayback::Playing { progress } if self.status != MediaControlsStatus::Playing => {
                info!("Changed to playing");
                redraw_playback = true;
                self.progress = progress.map(|d| d.0);
                self.status = MediaControlsStatus::Playing;
            }
            // Fallback - already playing
            MediaPlayback::Playing { progress } => {
                if self.progress != progress.map(|d| d.0) {
                    self.progress = progress.map(|d| d.0);
                    redraw_playback = true;
                }
            }
        }
        if self.title.as_deref() != playback_metadata.title {
            redraw_metadata = true;
            self.title = playback_metadata.title.map(ToOwned::to_owned);
        }
        if self.album.as_deref() != playback_metadata.album {
            redraw_metadata = true;
            self.album = playback_metadata.album.map(ToOwned::to_owned);
        }
        if self.artist.as_deref() != playback_metadata.artist {
            redraw_metadata = true;
            self.artist = playback_metadata.artist.map(ToOwned::to_owned);
        }
        if self.cover_url.as_deref() != playback_metadata.cover_url {
            redraw_metadata = true;
            self.cover_url = playback_metadata.cover_url.map(ToOwned::to_owned);
        }
        if self.duration != playback_metadata.duration {
            redraw_metadata = true;
            self.duration = playback_metadata.duration;
        }
        if redraw_playback {
            let new_playback = match self.status {
                MediaControlsStatus::Stopped => MediaPlayback::Stopped,
                MediaControlsStatus::Paused => MediaPlayback::Paused {
                    progress: self.progress.map(souvlaki::MediaPosition),
                },
                MediaControlsStatus::Playing => MediaPlayback::Playing {
                    progress: self.progress.map(souvlaki::MediaPosition),
                },
            };
            self.inner.set_playback(new_playback).unwrap();
        }
        if redraw_metadata {
            let new_metadata = MediaMetadata {
                title: self.title.as_deref(),
                album: self.album.as_deref(),
                artist: self.artist.as_deref(),
                cover_url: self.cover_url.as_deref(),
                duration: self.duration,
            };
            info!("new_metadata: {:?}", new_metadata);
            self.inner.set_metadata(new_metadata).unwrap();
        }
    }
}
