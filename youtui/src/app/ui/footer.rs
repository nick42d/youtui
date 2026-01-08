use crate::app::structures::{AlbumArtState, PlayState};
use crate::drawutils::{
    BUTTON_BG_COLOUR, BUTTON_FG_COLOUR, PROGRESS_BG_COLOUR, PROGRESS_FG_COLOUR, middle_of_rect,
};
use itertools::Itertools;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui_image::Image;
use ratatui_image::picker::Picker;
use std::time::Duration;

pub const ALBUM_ART_WIDTH: u16 = 7;

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
        format!("{hours}:{rem_mins:02}:{rem_secs:02}")
    } else {
        format!("{rem_mins:02}:{rem_secs:02}")
    }
}

pub fn draw_footer(
    f: &mut Frame,
    w: &mut super::YoutuiWindow,
    chunk: Rect,
    terminal_image_capabilities: &Picker,
) {
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
    let bar_str = format!("{progress_str}/{duration_str}");

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
    let album_art = cur_active_song.map(|s| &s.album_art);
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
        Line::from(Span::raw(format!("{vol:>3}"))),
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
    let [album_art_and_progress_bar_chunk, vol_bar_chunk] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .areas(block_inner);
    let get_progress_bar_and_text_layout = |r: Rect| {
        let [song_text_chunk, progress_bar_chunk] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Max(1)])
            .areas(r);
        (
            song_text_chunk,
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Max(4), Constraint::Min(1), Constraint::Max(4)])
                .areas(progress_bar_chunk),
        )
    };
    let (song_text_chunk, [left_arrow_chunk, progress_bar_chunk, right_arrow_chunk]) =
        match album_art {
            Some(AlbumArtState::None) | None => {
                get_progress_bar_and_text_layout(album_art_and_progress_bar_chunk)
            }
            Some(album_art) => {
                let [album_art_chunk, _, progress_bar_chunk] = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(ALBUM_ART_WIDTH),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .areas(album_art_and_progress_bar_chunk);
                match album_art {
                    AlbumArtState::Downloaded(album_art) => {
                        // TODO: consider hoist ability to panic here up to album_art_downloader
                        // server call.
                        // Since album art is fixed size, no
                        // need to use resize protocol so this might be acceptable.
                        // Drawback: This would mean relying on the server to provide a correctly
                        // sized image.
                        // Benefit: This includes an encoding step and so would be good to do that
                        // on the backend.
                        let image = terminal_image_capabilities
                            .new_protocol(
                                album_art.in_mem_image.clone(),
                                Rect {
                                    x: 0,
                                    y: 0,
                                    width: ALBUM_ART_WIDTH,
                                    height: ALBUM_ART_WIDTH,
                                },
                                ratatui_image::Resize::Fit(None),
                            )
                            .unwrap();
                    }
                    AlbumArtState::Error => {
                        let fallback_album_widget = Paragraph::new("").centered();
                    }
                    AlbumArtState::Init => {
                        let fallback_album_widget = Paragraph::new("").centered();
                    }
                    AlbumArtState::None => {
                        unreachable!("This arm is covered by the earlier match statement")
                    }
                };
                get_progress_bar_and_text_layout(progress_bar_chunk)
            }
        };
    f.render_widget(bar, progress_bar_chunk);
    f.render_widget(left_arrow, left_arrow_chunk);
    f.render_widget(right_arrow, right_arrow_chunk);
    //f.render_widget(block, chunk);
    f.render_widget(footer, song_text_chunk);
    f.render_widget(vol_bar, vol_bar_chunk);
}
