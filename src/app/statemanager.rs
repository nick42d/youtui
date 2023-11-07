use ytmapi_rs::{common::TextRun, parse::SongResult};

use super::{
    server::downloader::SongProgressUpdateType,
    structures::{ListSongID, Percentage},
    ui::YoutuiWindow,
};

// A message from the server to update state.
#[derive(Debug)]
pub enum StateUpdateMessage {
    SetSongProgress(SongProgressUpdateType, ListSongID),
    ReplaceArtistList(Vec<ytmapi_rs::parse::SearchResultArtist>),
    HandleSearchArtistError,
    ReplaceSearchSuggestions(Vec<Vec<TextRun>>, String),
    HandleSongListLoading,
    HandleSongListLoaded,
    HandleNoSongsFound,
    HandleSongsFound,
    AppendSongList {
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
    },
    HandleDonePlaying(ListSongID),
    SetToPaused(ListSongID),
    SetToPlaying(ListSongID),
    SetToStopped,
    SetVolume(Percentage),
}

pub async fn process_state_updates(
    state: &mut YoutuiWindow,
    state_updates: Vec<StateUpdateMessage>,
) {
    // Process all messages in queue from API on each tick.
    for msg in state_updates {
        tracing::debug!("Processing {:?}", msg);
        update_state(state, msg).await;
    }
}
pub async fn update_state(state: &mut YoutuiWindow, state_update_msg: StateUpdateMessage) {
    match state_update_msg {
        StateUpdateMessage::SetSongProgress(update, id) => {
            state.handle_song_progress_update(update, id).await
        }
        StateUpdateMessage::ReplaceArtistList(l) => state.handle_replace_artist_list(l).await,
        StateUpdateMessage::HandleSearchArtistError => state.handle_search_artist_error(),
        StateUpdateMessage::ReplaceSearchSuggestions(runs, query) => {
            state.handle_replace_search_suggestions(runs, query).await
        }
        StateUpdateMessage::HandleSongListLoading => state.handle_song_list_loading(),
        StateUpdateMessage::HandleSongListLoaded => state.handle_song_list_loaded(),
        StateUpdateMessage::HandleNoSongsFound => state.handle_no_songs_found(),
        StateUpdateMessage::HandleSongsFound => state.handle_songs_found(),
        StateUpdateMessage::AppendSongList {
            song_list,
            album,
            year,
            artist,
        } => state.handle_append_song_list(song_list, album, year, artist),
        StateUpdateMessage::HandleDonePlaying(id) => state.handle_done_playing(id).await,
        StateUpdateMessage::SetToPaused(id) => state.handle_set_to_paused(id).await,
        StateUpdateMessage::SetToPlaying(id) => state.handle_set_to_playing(id).await,
        StateUpdateMessage::SetToStopped => state.handle_set_to_stopped().await,
        StateUpdateMessage::SetVolume(p) => state.handle_set_volume(p),
    }
}
