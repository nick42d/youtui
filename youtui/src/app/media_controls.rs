//! Wrapper for souvlaki::MediaControls that performs diffing to ensure OS calls
//! are made at a minimum (in line with immediate mode architecture principle)
use super::structures::Percentage;
use super::ui::playlist::DEFAULT_UI_VOLUME;
use crate::core::blocking_send_or_error;
use futures::Stream;
use souvlaki::{MediaControlEvent, MediaMetadata, MediaPosition, PlatformConfig};
use std::borrow::Cow;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Minimum change in playing position before triggering a redraw. This is to
/// reduce number of calls to the platform.
const POSITION_DIFFERENCE_REDRAW_THRESHOLD: Duration = Duration::from_secs(5);

// On some platforms, souvlaki::Error doesn't implement Error, so this is the
// workaround.
// TODO: Report upstream.
#[derive(Debug)]
struct MediaControlsError(souvlaki::Error);
impl std::error::Error for MediaControlsError {}
impl std::fmt::Display for MediaControlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(all(
            unix,
            not(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            ))
        )) {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

pub struct MediaController {
    inner: souvlaki::MediaControls,
    status: souvlaki::MediaPlayback,
    volume: MediaControlsVolume,
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    cover_url: Option<String>,
    duration: Option<Duration>,
    /// macos requires an active window handle
    #[cfg(target_os = "macos")]
    macos_window_handle: raw_window_handle::AppKitWindowHandle,
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
    fn default() -> Self {
        Self(DEFAULT_UI_VOLUME.0 as f64 / 100.0)
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
        use raw_window_handle::HasWindowHandle;
        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "windows")]
        let raw_window_handle::RawWindowHandle::Win32(raw_win32_handle) =
            winit::event_loop::EventLoop::builder()
                .with_any_thread(true)
                .build()?
                .create_window(winit::window::Window::default_attributes().with_visible(false))?
                .window_handle()?
                .as_raw()
        else {
            anyhow::bail!("Expected to get a Win32WindowHandle but we did not!")
        };
        #[cfg(target_os = "macos")]
        use raw_window_handle::HasWindowHandle;
        #[cfg(target_os = "macos")]
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        #[cfg(target_os = "macos")]
        let raw_window_handle::RawWindowHandle::AppKit(macos_window_handle) =
            winit::event_loop::EventLoop::builder()
                .build()?
                .create_window(winit::window::Window::default_attributes().with_visible(false))?
                .window_handle()?
                .as_raw()
        else {
            anyhow::bail!("Expected to get a AppKitWindowHandle but we did not!")
        };

        let config = PlatformConfig {
            display_name: "Youtui",
            dbus_name: "youtui",
            #[cfg(not(target_os = "windows"))]
            hwnd: None,
            #[cfg(target_os = "windows")]
            hwnd: Some(raw_win32_handle.hwnd.get() as *mut std::ffi::c_void),
        };

        let mut controls = souvlaki::MediaControls::new(config).map_err(MediaControlsError)?;
        // Assumption - event handler runs in another thread, and blocking send is
        // acceptable.
        controls
            .attach(move |event| {
                blocking_send_or_error(&tx, event);
            })
            .map_err(MediaControlsError)?;
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
                #[cfg(target_os = "macos")]
                macos_window_handle,
            },
            ReceiverStream::new(rx),
        ))
    }
    pub fn update_controls(&mut self, update: MediaControlsUpdate<'_>) -> anyhow::Result<()> {
        let MediaControlsUpdate {
            title,
            album,
            artist,
            cover_url,
            duration,
            playback_status,
            volume,
        } = update;
        self.update_metadata(title, album, artist, cover_url, duration)?;
        self.update_playback(playback_status)?;
        #[cfg(target_os = "linux")]
        self.update_volume(volume)?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    fn update_volume(&mut self, volume: MediaControlsVolume) -> anyhow::Result<()> {
        if self.volume != volume {
            self.volume = volume;
            self.inner.set_volume(volume.0)?;
        }
        Ok(())
    }
    fn update_metadata(
        &mut self,
        title: Option<Cow<'_, str>>,
        album: Option<Cow<'_, str>>,
        artist: Option<Cow<'_, str>>,
        cover_url: Option<Cow<'_, str>>,
        duration: Option<Duration>,
    ) -> anyhow::Result<()> {
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
            self.inner
                .set_metadata(new_metadata)
                .map_err(MediaControlsError)?;
        }
        Ok(())
    }
    fn update_playback(&mut self, playback_status: MediaControlsStatus) -> anyhow::Result<()> {
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
                    if let Some(progress) = progress {
                        if progress.0.abs_diff(new_progress) >= POSITION_DIFFERENCE_REDRAW_THRESHOLD
                        {
                            redraw = true;
                            self.status = souvlaki::MediaPlayback::Paused {
                                progress: Some(MediaPosition(new_progress)),
                            };
                        }
                    } else {
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
                    if let Some(progress) = progress {
                        if progress.0.abs_diff(new_progress) >= POSITION_DIFFERENCE_REDRAW_THRESHOLD
                        {
                            redraw = true;
                            self.status = souvlaki::MediaPlayback::Playing {
                                progress: Some(MediaPosition(new_progress)),
                            };
                        }
                    } else {
                        redraw = true;
                        self.status = souvlaki::MediaPlayback::Playing {
                            progress: Some(MediaPosition(new_progress)),
                        };
                    }
                }
            }
        }
        if redraw {
            self.inner
                .set_playback(self.status.clone())
                .map_err(MediaControlsError)?;
        }
        Ok(())
    }
}
