use crate::app::structures::PlayState;
use crate::drawutils::{
    middle_of_rect, BUTTON_BG_COLOUR, BUTTON_FG_COLOUR, PROGRESS_BG_COLOUR, PROGRESS_FG_COLOUR,
};
use itertools::Itertools;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Widget};
use ratatui::Frame;
use ratatui_image::{Image, StatefulImage};
use std::rc::Rc;
use std::time::Duration;
use ytmapi_rs::query::album;

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

pub fn draw_footer(f: &mut Frame, w: &mut super::YoutuiWindow, chunk: Rect) {
    let mut duration = 0;
    let mut progress = Duration::default();
    let play_ratio = match &w.playlist.play_status {
        PlayState::Playing(id) | PlayState::Paused(id) => {
            duration = w
                .playlist
                .get_song_from_id(*id)
                .map(|s| &s.duration_string)
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

    let cur_active_song = match w.playlist.play_status {
        PlayState::Error(id)
        | PlayState::Playing(id)
        | PlayState::Paused(id)
        | PlayState::Buffering(id) => w.playlist.get_song_from_id(id),
        PlayState::NotPlaying | PlayState::Stopped => None,
    };
    let song_and_artists_string = cur_active_song
        .map(|song| {
            let artists =
                Itertools::intersperse(song.artists.iter().map(|s| s.name.as_str()), ", ")
                    .collect::<String>();
            format!(
                "{} {} - {}",
                w.playlist.play_status.list_icon(),
                song.title,
                artists
            )
        })
        .unwrap_or_default();
    let album_title = cur_active_song
        .and_then(|s| s.album.as_ref())
        .map(|s| s.name.as_str())
        .unwrap_or_default();
    let album_art = cur_active_song
        .and_then(|s| s.album_art.as_deref())
        .map(|s| &s.in_mem_image);
    let footer = Paragraph::new(vec![
        Line::from(song_and_artists_string),
        Line::from(album_title),
    ]);
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
    let block = Block::default()
        .title("Status")
        .title(Line::from("Youtui").right_aligned())
        .borders(Borders::ALL);
    let vol_bar = Paragraph::new(vol_bar_spans).alignment(Alignment::Right);

    let block_inner = block.inner(chunk);
    let bar_and_vol = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .split(block_inner);
    let get_progress_bar_and_text_layout = |r: Rect| {
        let text_bar_vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Max(1)])
            .split(r);
        (
            text_bar_vertical_layout[0],
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Max(4), Constraint::Min(1), Constraint::Max(4)])
                .split(text_bar_vertical_layout[1]),
        )
    };
    let (song_text_rect, progress_bar_layout) = match cur_active_song {
        Some(_) => {
            let album_art_and_bar = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(7), Constraint::Min(0)])
                .split(bar_and_vol[0]);
            match album_art {
                Some(image) => {
                    w.footer_album_art_state = Some(
                        w.terminal_image_capabilities
                            .new_resize_protocol(image.clone()),
                    );
                    if let Some(state) = w.footer_album_art_state.as_mut() {
                        f.render_stateful_widget(
                            StatefulImage::default(),
                            album_art_and_bar[0],
                            state,
                        );
                    }
                }
                None => {
                    let fallback_album_widget = Paragraph::new("").centered();
                    f.render_widget(fallback_album_widget, middle_of_rect(album_art_and_bar[0]));
                }
            };
            get_progress_bar_and_text_layout(album_art_and_bar[1])
        }
        None => get_progress_bar_and_text_layout(bar_and_vol[0]),
    };
    f.render_widget(bar, progress_bar_layout[1]);
    f.render_widget(left_arrow, progress_bar_layout[0]);
    f.render_widget(right_arrow, progress_bar_layout[2]);
    f.render_widget(block, chunk);
    f.render_widget(footer, song_text_rect);
    f.render_widget(vol_bar, bar_and_vol[1]);
}
