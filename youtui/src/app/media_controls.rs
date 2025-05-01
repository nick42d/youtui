//! Wrapper for souvlaki::MediaControls that performs diffing to ensure OS calls
//! are made at a minimum (in line with immediate mode architecture principle)
use souvlaki::MediaControls;
use tokio::sync::mpsc;

struct MediaController {
    inner: MediaControls,
    prev_state: Option<MediaControlsState>,
}

struct MediaControlsState {}

impl MediaController {
    pub fn new() -> (Self, mpsc::Receiver<()>) {}
    pub fn update_controls(state: MediaControlsState) {}
}
