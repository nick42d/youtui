use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Row, Table};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    terminal::Frame,
};

use super::contextpane::ContextPane;
use super::{footer, header, WindowContext, YoutuiWindow};

pub fn draw_app<B>(f: &mut Frame<B>, w: &YoutuiWindow)
where
    B: Backend,
{
    let base_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.size());
    header::draw_header(f, w, base_layout[0]);
    match w.context {
        WindowContext::Browser => w.browser.draw_context_chunk(f, base_layout[1]),
        WindowContext::Logs => w.logger.draw_context_chunk(f, base_layout[1]),
        WindowContext::Playlist => w.playlist.draw_context_chunk(f, base_layout[1]),
    }
    if w.key_pending() {
        draw_popup(f, w, base_layout[1]);
    }
    footer::draw_footer(f, w, base_layout[2]);
}
fn draw_popup<B: Backend>(f: &mut Frame<B>, w: &YoutuiWindow, chunk: Rect) {
    let title = "test";
    let commands = w.get_cur_mode();
    // TODO: Remove unwrap, although we shouldn't be drawing popup if no Map.
    let shortcuts_descriptions = commands.unwrap().collect::<Vec<_>>();
    // Cloning here only clones iterators, so it's low-cost.
    // let shortcut_len = shortcuts.clone().map(|s| s.len()).max().unwrap_or_default();
    // let description_len = descriptions
    //     .clone()
    //     .map(|d| d.len())
    //     .max()
    //     .unwrap_or_default();
    // XXX: temporary
    let (shortcut_len, description_len) = shortcuts_descriptions
        .iter()
        .fold((0, 0), |(acc1, acc2), (s, c)| {
            (s.len().max(acc1), c.len().max(acc2))
        });
    let width = shortcut_len + description_len + 3;
    // let height = commands.len() + 2;
    // XXX: temporary
    let height = shortcuts_descriptions.len() + 2;
    let mut commands_vec = Vec::new();
    for (s, d) in shortcuts_descriptions {
        commands_vec.push(
            Row::new(vec![format!("{}", s), format!("{}", d)]).style(Style::new().fg(Color::White)),
        );
    }
    let table_constraints = [
        Constraint::Min(shortcut_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(description_len.try_into().unwrap_or(u16::MAX)),
    ];
    // let table_constraints = [
    //     Constraint::Length(shortcut_width + 10),
    //     Constraint::Length(description_width),
    // ];
    let block = Table::new(commands_vec)
        .style(Style::new().fg(Color::White))
        .block(
            Block::default()
                .title(title.as_ref())
                .borders(Borders::ALL)
                .style(Style::new().fg(Color::Cyan)),
        )
        .widths(&table_constraints);
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}
/// Helper function to create a popup at bottom corner of chunk.
pub fn left_bottom_corner_rect(height: u16, width: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(height)].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(width)].as_ref())
        .split(popup_layout[1])[1]
}
