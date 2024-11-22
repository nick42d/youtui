use std::time::Duration;

use crate::{
    app::structures::PlayState,
    drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR, PROGRESS_BG_COLOUR, PROGRESS_FG_COLOUR},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub fn parse_simple_time_to_secs<S: AsRef<str>>(time_string: S) -> usize {
    time_string
        .as_ref()
        .rsplit(':')
        .flat_map(|n| n.parse::<usize>().ok())
        .zip([1, 60, 3600])
        .fold(0, |acc, (time, multiplier)| acc + time * multiplier)
}

pub fn secs_to_time_string(secs: usize) -> String {
    // Naive implementation
    let hours = secs / 3600;
    let rem_mins = (secs - (hours * 3600)) / 60;
    let rem_secs = secs - (hours * 3600 + rem_mins * 60);
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, rem_mins, rem_secs)
    } else {
        format!("{:02}:{:02}", rem_mins, rem_secs)
    }
}

pub fn draw_footer(f: &mut Frame, w: &super::YoutuiWindow, chunk: Rect) {
    let cur = &w.playlist.play_status;
    let mut duration = 0;
    let mut progress = Duration::default();
    let play_ratio = match cur {
        PlayState::Playing(id) | PlayState::Paused(id) => {
            duration = w
                .playlist
                .get_song_from_id(*id)
                .map(|s| &s.raw.duration)
                .map(parse_simple_time_to_secs)
                .unwrap_or(0);
            progress = w.playlist.cur_played_dur.unwrap_or_default();
            (progress.as_secs_f64() / duration as f64).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    let progress_str = secs_to_time_string(progress.as_secs() as usize);
    let duration_str = secs_to_time_string(duration);
    let bar_str = format!("{}/{}", progress_str, duration_str);
    let song_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .map(|s| s.raw.title.to_owned())
            .unwrap_or("No title".to_string()),
        PlayState::NotPlaying => "Not playing".to_string(),
        PlayState::Stopped => "Not playing".to_string(),
    };
    let album_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            .map(|s| s.get_album().to_owned())
            .unwrap_or("".to_string()),
        PlayState::NotPlaying => "".to_string(),
        PlayState::Stopped => "".to_string(),
    };
    let artist_title = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w
            .playlist
            .get_song_from_id(id)
            // TODO: tidy this up as ListSong only contains one artist currently.
            // TODO: Remove allocation
            .map(|s| {
                s.get_artists()
                    .clone()
                    .first()
                    .map(|a| a.to_string())
                    .unwrap_or_default()
            })
            .unwrap_or("".to_string()),
        PlayState::NotPlaying => "".to_string(),
        PlayState::Stopped => "".to_string(),
    };
    let song_title_string = match w.playlist.play_status {
        PlayState::Error(_)
        | PlayState::Playing(_)
        | PlayState::Paused(_)
        | PlayState::Buffering(_) => format!(
            "{} {song_title} - {artist_title}",
            w.playlist.play_status.list_icon()
        ),
        PlayState::NotPlaying => "".to_string(),
        PlayState::Stopped => "".to_string(),
    };
    let footer = Paragraph::new(vec![Line::from(song_title_string), Line::from(album_title)]);
    let block = Block::default()
        .title("Status")
        .title(Line::from("Youtui").right_aligned())
        .borders(Borders::ALL);
    let block_inner = block.inner(chunk);
    let song_vol = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .split(block_inner);
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Max(1)])
        .split(song_vol[0]);
    let progress_bar_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Max(4), Constraint::Min(1), Constraint::Max(4)])
        .split(vertical_layout[1]);
    let bar = Gauge::default()
        .label(bar_str)
        .gauge_style(
            Style::default()
                .fg(PROGRESS_FG_COLOUR)
                .bg(PROGRESS_BG_COLOUR),
        )
        .ratio(play_ratio);
    let left_arrow = Paragraph::new(Line::from(vec![
        Span::styled(
            "< [",
            Style::new()
                .fg(BUTTON_FG_COLOUR)
                .bg(BUTTON_BG_COLOUR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]));
    let right_arrow = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "] >",
            Style::new()
                .fg(BUTTON_FG_COLOUR)
                .bg(BUTTON_BG_COLOUR)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    let vol = w.playlist.volume.0;
    let vol_bar_spans = vec![
        Line::from(Span::styled(
            " + ",
            Style::new()
                .fg(BUTTON_FG_COLOUR)
                .bg(BUTTON_BG_COLOUR)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(format!("{:>3}", vol))),
        Line::from(Span::styled(
            " - ",
            Style::new()
                .fg(BUTTON_FG_COLOUR)
                .bg(BUTTON_BG_COLOUR)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let vol_bar = Paragraph::new(vol_bar_spans).alignment(Alignment::Right);
    f.render_widget(block, chunk);
    f.render_widget(footer, vertical_layout[0]);
    f.render_widget(bar, progress_bar_row[1]);
    f.render_widget(left_arrow, progress_bar_row[0]);
    f.render_widget(right_arrow, progress_bar_row[2]);
    f.render_widget(vol_bar, song_vol[1]);
}
