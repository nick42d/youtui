use super::{footer, header, WindowContext, YoutuiWindow};
use crate::app::component::actionhandler::KeyDisplayer;
use crate::app::keycommand::{DisplayableCommand, DisplayableMode};
use crate::app::view::draw::draw_panel;
use crate::app::view::{Drawable, DrawableMut};
use crate::app::YoutuiMutableState;
use crate::drawutils::{
    highlight_style, left_bottom_corner_rect, SELECTED_BORDER_COLOUR, TABLE_HEADINGS_COLOUR,
    TEXT_COLOUR,
};
use ratatui::prelude::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::{block, line};
use ratatui::widgets::{
    Block, Borders, Clear, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    terminal::Frame,
};
use std::borrow::Cow;

// Add tests to try and draw app with oddly sized windows.
pub fn draw_app(f: &mut Frame, w: &YoutuiWindow, m: &mut YoutuiMutableState) {
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
    let context_selected = !w.help.shown && !w.key_pending();
    match w.context {
        WindowContext::Browser => w
            .browser
            .draw_mut_chunk(f, base_layout[1], m, context_selected),
        WindowContext::Logs => w.logger.draw_chunk(f, base_layout[1], context_selected),
        WindowContext::Playlist => {
            w.playlist
                .draw_mut_chunk(f, base_layout[1], m, context_selected)
        }
    }
    if w.help.shown {
        draw_help(f, w, &mut m.help_state, base_layout[1]);
    }
    if w.key_pending() {
        draw_popup(f, w, base_layout[1]);
    }
    footer::draw_footer(f, w, base_layout[2]);
}
fn draw_popup(f: &mut Frame, w: &YoutuiWindow, chunk: Rect) {
    // NOTE: if there are more commands than we can fit on the screen, some will be
    // cut off. If there are no commands, no need to draw anything.
    let Some(DisplayableMode {
        displayable_commands: commands,
        description: title,
    }) = w.get_cur_displayable_mode()
    else {
        return;
    };
    let shortcuts_descriptions = commands.collect::<Vec<_>>();
    // TODO: Make commands_vec an iterator instead of a vec
    let (shortcut_len, description_len, commands_vec) = shortcuts_descriptions.iter().fold(
        (0, 0, Vec::new()),
        |(acc1, acc2, mut commands_vec),
         DisplayableCommand {
             keybinds,
             context: _,
             description,
         }| {
            commands_vec.push(
                Row::new(vec![format!("{}", keybinds), format!("{}", description)])
                    .style(Style::new().fg(TEXT_COLOUR)),
            );
            (
                keybinds.len().max(acc1),
                description.len().max(acc2),
                commands_vec,
            )
        },
    );
    let width = shortcut_len + description_len + 3;
    let height = commands_vec.len() + 2;
    let table_constraints = [
        Constraint::Min(shortcut_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(description_len.try_into().unwrap_or(u16::MAX)),
    ];
    let block = Table::new(commands_vec, table_constraints).block(
        Block::default()
            .title(title.as_ref())
            .borders(Borders::ALL)
            .style(Style::new().fg(SELECTED_BORDER_COLOUR)),
    );
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}

fn draw_help(f: &mut Frame, w: &YoutuiWindow, state: &mut TableState, chunk: Rect) {
    // NOTE: if there are more commands than we can fit on the screen, some will be
    // cut off.
    let commands = w.get_all_visible_keybinds_as_readable_iter();
    // Get the maximum length of each element in the tuple vector created above, as
    // well as the number of items. XXX: Probably don't need to map then fold,
    // just fold. XXX: Fold closure could be written as a function, then becomes
    // testable.
    let (mut s_len, mut c_len, mut d_len, items) = commands
        .map(
            |DisplayableCommand {
                 keybinds,
                 context,
                 description,
             }| (keybinds.len(), context.len(), description.len()),
        )
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
    // XXX: We're running get_all_visible_keybinds a second time here.
    // Better to move to the fold above.
    let commands_table = w.get_all_visible_keybinds_as_readable_iter().map(
        |DisplayableCommand {
             keybinds,
             context,
             description,
         }| {
            // TODO: Remove vec allocation?
            Row::new(vec![
                keybinds.to_string(),
                context.to_string(),
                description.to_string(),
            ])
            .style(Style::new().fg(TEXT_COLOUR))
        },
    );
    let table_constraints = [
        Constraint::Min(s_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(c_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(d_len.try_into().unwrap_or(u16::MAX)),
    ];
    let headings = ["Key", "Context", "Command"];
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    draw_generic_scrollable_table(
        f,
        commands_table,
        "Help".into(),
        w.help.cur,
        items,
        &table_constraints,
        &headings,
        area,
        state,
        true,
    );
}

fn draw_generic_scrollable_table<'a, T: IntoIterator<Item = Row<'a>>>(
    f: &mut Frame,
    table_items: T,
    title: Cow<str>,
    cur: usize,
    len: usize,
    layout: &[Constraint], // Can this be done better?
    headings: &[&'static str],
    chunk: Rect,
    state: &mut TableState,
    selected: bool,
) {
    // TODO: theming
    // Set the state to the currently selected item.
    state.select(Some(cur));
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let headings_iter = headings.iter().map(|h| *h);
    let table_widget = Table::new(table_items, layout)
        .highlight_style(highlight_style())
        .header(
            Row::new(headings_iter).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(TABLE_HEADINGS_COLOUR),
            ),
        )
        .column_spacing(1);
    // TODO: Don't display scrollbar if all items fit on the screen.
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(Some(line::VERTICAL))
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = len.saturating_sub(table_height);
    let inner_chunk = draw_panel(f, title, None, chunk, selected);
    f.render_stateful_widget(table_widget, inner_chunk, state);
    // Call this after rendering table, as offset is mutated.
    let mut scrollbar_state = ScrollbarState::default()
        .position(state.offset().min(scrollable_lines))
        .content_length(scrollable_lines);
    f.render_stateful_widget(
        scrollbar,
        chunk.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    )
}
