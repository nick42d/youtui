use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Row, Table};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    terminal::Frame,
};

use super::{footer, header, WindowContext, YoutuiWindow};
use crate::app::component::actionhandler::{Action, DisplayableKeyRouter};
use crate::app::view::Drawable;
use crate::drawutils::left_bottom_corner_rect;

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
        WindowContext::Browser => w.browser.draw_chunk(f, base_layout[1]),
        WindowContext::Logs => w.logger.draw_chunk(f, base_layout[1]),
        WindowContext::Playlist => w.playlist.draw_chunk(f, base_layout[1]),
    }
    if w.help_shown {
        draw_help(f, w, base_layout[1]);
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

fn draw_help<B: Backend, D: DisplayableKeyRouter + ?Sized>(
    f: &mut Frame<B>,
    context: &D,
    chunk: Rect,
) {
    // Collect to a Vec so we can create more iterators. Dynamically dispatched Iterator can't be cloned.
    let commands: Vec<_> = context.get_all_keybinds_as_readable_iter().collect();
    // Get the maximum length of each element in the tuple vector created above.
    let (mut s_len, mut c_len, mut d_len) = commands
        .iter()
        .map(|(s, c, d)| (s.len(), c.len(), d.len()))
        .fold((0, 0, 0), |(smax, cmax, dmax), (s, c, d)| {
            (smax.max(s), cmax.max(c), dmax.max(d))
        });
    // Ensure the width of each column is at least as wide as header.
    (s_len, c_len, d_len) = (s_len.max(3), c_len.max(7), d_len.max(7));
    // Total block width required, including padding and borders.
    let width = s_len + c_len + d_len + 4;
    // Total block height required, including header and borders.
    let height = commands.len() + 3;
    // Naive implementation
    let commands_table = commands.iter().map(|(s, c, d)| {
        Row::new(vec![s.as_ref(), c.as_ref(), d.as_ref()]).style(Style::new().fg(Color::White))
    });
    let table_constraints = [
        Constraint::Min(s_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(c_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(d_len.try_into().unwrap_or(u16::MAX)),
    ];
    let block = Table::new(commands_table)
        .header(Row::new(vec!["Key", "Context", "Command"]))
        .style(Style::new().fg(Color::White))
        .block(
            Block::default()
                // TODO: Remove borrow.
                .title("Help")
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
