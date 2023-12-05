use super::{footer, header, WindowContext, YoutuiWindow};
use crate::app::component::actionhandler::DisplayableKeyRouter;
use crate::app::view::{Drawable, DrawableMut};
use crate::drawutils::{
    highlight_style, left_bottom_corner_rect, SELECTED_BORDER_COLOUR, TEXT_COLOUR,
};
use ratatui::prelude::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Row, Table};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    terminal::Frame,
};

pub fn draw_app(f: &mut Frame, w: &mut YoutuiWindow) {
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
        WindowContext::Browser => w
            .browser
            .draw_mut_chunk(f, base_layout[1], &mut w.mutable_state),
        WindowContext::Logs => w.logger.draw_chunk(f, base_layout[1]),
        WindowContext::Playlist => {
            w.playlist
                .draw_mut_chunk(f, base_layout[1], &mut w.mutable_state)
        }
    }
    if w.help.shown {
        draw_help(f, w, base_layout[1]);
    }
    if w.key_pending() {
        draw_popup(f, w, base_layout[1]);
    }
    footer::draw_footer(f, w, base_layout[2]);
}
fn draw_popup(f: &mut Frame, w: &YoutuiWindow, chunk: Rect) {
    // NOTE: if there are more commands than we can fit on the screen, some will be cut off.
    // If there are no commands, no need to draw anything.
    let Some(title) = w.get_cur_mode_description() else {
        return;
    };
    // If there are no commands, no need to draw anything.
    let Some(commands) = w.get_cur_mode() else {
        return;
    };
    let shortcuts_descriptions = commands.collect::<Vec<_>>();
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
            Row::new(vec![format!("{}", s), format!("{}", d)]).style(Style::new().fg(TEXT_COLOUR)),
        );
    }
    let table_constraints = [
        Constraint::Min(shortcut_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(description_len.try_into().unwrap_or(u16::MAX)),
    ];
    let block = Table::new(commands_vec)
        .block(
            Block::default()
                .title(title.as_ref())
                .borders(Borders::ALL)
                .style(Style::new().fg(SELECTED_BORDER_COLOUR)),
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

fn draw_help(f: &mut Frame, w: &mut YoutuiWindow, chunk: Rect) {
    // NOTE: if there are more commands than we can fit on the screen, some will be cut off.
    // Set up mutable state
    w.mutable_state.help_state.select(Some(w.help.cur));
    let commands = w.get_all_visible_keybinds_as_readable_iter();
    // Get the maximum length of each element in the tuple vector created above, as well as the number of items.
    let (mut s_len, mut c_len, mut d_len, items) = commands
        .map(|(s, c, d)| (s.len(), c.len(), d.len()))
        .fold((0, 0, 0, 0), |(smax, cmax, dmax, n), (s, c, d)| {
            (smax.max(s), cmax.max(c), dmax.max(d), n + 1)
        });
    // Ensure the width of each column is at least as wide as header.
    (s_len, c_len, d_len) = (s_len.max(3), c_len.max(7), d_len.max(7));
    // Total block width required, including padding and borders.
    let width = s_len + c_len + d_len + 4;
    // Total block height required, including header and borders.
    let height = items + 3;
    // Naive implementation
    let commands_table: Vec<_> = w
        .get_all_visible_keybinds_as_readable_iter()
        .map(|(s, c, d)| {
            // Allocate to avoid collision with the mutable state used below.
            // TODO: Remove allocation (Store mutable state outside of YoutuiWindow?)
            Row::new(vec![s.to_string(), c.to_string(), d.to_string()])
                .style(Style::new().fg(TEXT_COLOUR))
        })
        .collect();
    let table_constraints = [
        Constraint::Min(s_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(c_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(d_len.try_into().unwrap_or(u16::MAX)),
    ];
    let block = Table::new(commands_table)
        .highlight_style(highlight_style())
        .header(Row::new(vec!["Key", "Context", "Command"]))
        .block(
            Block::default()
                // TODO: Remove borrow.
                .title("Help")
                .borders(Borders::ALL)
                .style(Style::new().fg(SELECTED_BORDER_COLOUR)),
        )
        .widths(&table_constraints);
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_stateful_widget(block, area, &mut w.mutable_state.help_state);
}
