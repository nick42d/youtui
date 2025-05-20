//! Wrapper for souvlaki::MediaControls that performs diffing to ensure OS calls
//! are made at a minimum (in line with immediate mode architecture principle)
use super::structures::Percentage;
use futures::Stream;
use souvlaki::{MediaControlEvent, MediaMetadata, MediaPosition, PlatformConfig};
use std::{borrow::Cow, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub struct MediaController {
    inner: souvlaki::MediaControls,
    status: souvlaki::MediaPlayback,
    volume: MediaControlsVolume,
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    cover_url: Option<String>,
    duration: Option<Duration>,
}

pub struct MediaControlsUpdate<'a> {
    pub title: Option<Cow<'a, str>>,
    pub album: Option<Cow<'a, str>>,
    pub artist: Option<Cow<'a, str>>,
    pub cover_url: Option<Cow<'a, str>>,
    pub duration: Option<Duration>,
    pub playback_status: MediaControlsStatus,
    pub volume: MediaControlsVolume,
}

#[derive(Default)]
pub enum MediaControlsStatus {
    #[default]
    Stopped,
    Paused {
        progress: Duration,
    },
    Playing {
        progress: Duration,
    },
}

#[derive(Copy, Clone, PartialEq)]
pub struct MediaControlsVolume(f64);

impl Default for MediaControlsVolume {
    // Default copied from app::ui::playlist
    fn default() -> Self {
        Self(0.5)
    }
}

impl MediaControlsVolume {
    pub fn from_percentage_clamped(Percentage(p): Percentage) -> Self {
        let raw = (p as f64) / 100.0;
        Self(raw.clamp(0.0, 1.0))
    }
}

impl MediaController {
    pub fn new() -> anyhow::Result<(Self, impl Stream<Item = MediaControlEvent>)> {
        let (tx, rx) = mpsc::channel(super::EVENT_CHANNEL_SIZE);

        // On windows, a hwnd window handle is required, so we create a non-visible
        // window using winit. See souvlaki docs for more information.
        #[cfg(target_os = "windows")]
        let raw_window_handle::RawWindowHandle::Win32(raw_win32_handle) =
            winit::window::WindowBuilder::new()
                .with_visible(false)
                .build(&EventLoop::<()>::new_any_thread())?
                .window_handle()?
                .as_raw()
        else {
            anyhow::bail!("Expected to get a Win32WindowHandle but we did not!")
        };

        let config = PlatformConfig {
            display_name: "Youtui",
            dbus_name: "youtui",
            #[cfg(not(target_os = "windows"))]
            hwnd: None,
            #[cfg(target_os = "windows")]
            hwnd: raw_win32_handle.hwnd,
        };

        let mut controls = souvlaki::MediaControls::new(config).unwrap();
        // Assumption - event handler runs in another thread, and blocking send is
        // acceptable.
        controls
            .attach(move |event| {
                tx.blocking_send(event).unwrap();
            })
            .unwrap();
        Ok((
            MediaController {
                inner: controls,
                status: souvlaki::MediaPlayback::Stopped,
                title: None,
                album: None,
                artist: None,
                cover_url: None,
                duration: None,
                volume: Default::default(),
            },
            ReceiverStream::new(rx),
        ))
    }
    pub fn update_controls(&mut self, update: MediaControlsUpdate<'_>) {
        let MediaControlsUpdate {
            title,
            album,
            artist,
            cover_url,
            duration,
            playback_status,
            volume,
        } = update;
        self.update_metadata(title, album, artist, cover_url, duration);
        self.update_playback(playback_status);
        self.update_volume(volume);
    }
    fn update_volume(&mut self, volume: MediaControlsVolume) {
        if self.volume != volume {
            self.volume = volume;
            self.inner.set_volume(volume.0);
        }
    }
    fn update_metadata(
        &mut self,
        title: Option<Cow<'_, str>>,
        album: Option<Cow<'_, str>>,
        artist: Option<Cow<'_, str>>,
        cover_url: Option<Cow<'_, str>>,
        duration: Option<Duration>,
    ) {
        let mut redraw = false;
        if self.title.as_deref() != title.as_deref() {
            redraw = true;
            self.title = title.map(|title| title.to_string());
        }
        if self.album.as_deref() != album.as_deref() {
            redraw = true;
            self.album = album.map(|album| album.to_string());
        }
        if self.artist.as_deref() != artist.as_deref() {
            redraw = true;
            self.artist = artist.map(|artist| artist.to_string());
        }
        if self.cover_url.as_deref() != cover_url.as_deref() {
            redraw = true;
            self.cover_url = cover_url.map(|cover_url| cover_url.to_string());
        }
        if self.duration != duration {
            redraw = true;
            self.duration = duration;
        }
        if redraw {
            let new_metadata = MediaMetadata {
                title: self.title.as_deref(),
                album: self.album.as_deref(),
                artist: self.artist.as_deref(),
                cover_url: self.cover_url.as_deref(),
                duration: self.duration,
            };
            self.inner.set_metadata(new_metadata).unwrap();
        }
    }
    fn update_playback(&mut self, playback_status: MediaControlsStatus) {
        let mut redraw = false;
        match playback_status {
            MediaControlsStatus::Stopped => {
                if self.status != souvlaki::MediaPlayback::Stopped {
                    self.status = souvlaki::MediaPlayback::Stopped;
                    redraw = true;
                }
            }
            MediaControlsStatus::Paused {
                progress: new_progress,
            } => {
                if !matches!(self.status, souvlaki::MediaPlayback::Paused { .. }) {
                    redraw = true;
                    self.status = souvlaki::MediaPlayback::Paused {
                        progress: Some(MediaPosition(new_progress)),
                    };
                }
                if let souvlaki::MediaPlayback::Paused { progress } = self.status {
                    if progress == Some(MediaPosition(new_progress)) {
                        redraw = true;
                        self.status = souvlaki::MediaPlayback::Paused {
                            progress: Some(MediaPosition(new_progress)),
                        };
                    }
                }
            }
            MediaControlsStatus::Playing {
                progress: new_progress,
            } => {
                if !matches!(self.status, souvlaki::MediaPlayback::Playing { .. }) {
                    redraw = true;
                    self.status = souvlaki::MediaPlayback::Playing {
                        progress: Some(MediaPosition(new_progress)),
                    };
                }
                if let souvlaki::MediaPlayback::Playing { progress } = self.status {
                    if progress == Some(MediaPosition(new_progress)) {
                        redraw = true;
                        self.status = souvlaki::MediaPlayback::Playing {
                            progress: Some(MediaPosition(new_progress)),
                        };
                    }
                }
            }
        }
        if redraw {
            self.inner.set_playback(self.status.clone()).unwrap();
        }
    }
}
