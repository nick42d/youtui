use super::{WindowContext, YoutuiWindow, footer, header};
use crate::app::view::draw::{draw_panel_mut_impl, draw_table_impl};
use crate::app::view::{BasicConstraint, Drawable, DrawableMut};
use crate::drawutils::{SELECTED_BORDER_COLOUR, TEXT_COLOUR, left_bottom_corner_rect};
use crate::keyaction::{DisplayableKeyAction, DisplayableMode};
use rat_text::HasScreenCursor;
use rat_text::text_input::{TextInput, TextInputState};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Row, Table};
use ratatui_image::picker::Picker;

// Add tests to try and draw app with oddly sized windows.
pub fn draw_app(f: &mut Frame, w: &mut YoutuiWindow, terminal_image_capabilities: &Picker) {
    let base_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(4),
                Constraint::Min(2),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.area());
    header::draw_header(f, w, base_layout[0]);
    let context_selected = !w.help.shown && !w.key_pending();
    match w.context {
        WindowContext::Browser => {
            w.browser
                .draw_mut_chunk(f, base_layout[1], context_selected, w.tick);
        }
        WindowContext::Logs => w.logger.draw_chunk(f, base_layout[1], context_selected),
        WindowContext::Playlist => {
            w.playlist
                .draw_mut_chunk(f, base_layout[1], context_selected, w.tick);
        }
    }
    if w.help.shown {
        draw_help(f, w, base_layout[1]);
    }
    if w.key_pending() {
        draw_popup(f, w, base_layout[1]);
    }
    footer::draw_footer(f, w, base_layout[2], terminal_image_capabilities);
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
         DisplayableKeyAction {
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

/// Draw the help page. The help page should show all visible commands for the
/// current page.
fn draw_help(f: &mut Frame, w: &mut YoutuiWindow, chunk: Rect) {
    // XXX: Probably don't need to map then fold,
    // just fold.
    //
    // XXX: Fold closure could be written as a function, then becomes
    // testable.
    let (mut s_len, mut c_len, mut d_len, items) = w
        .get_help_list_items()
        .map(
            |DisplayableKeyAction {
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
    // XXX: We're running get_help_list_items a second time here.
    // Better to move to the fold above.
    let table_constraints = [
        BasicConstraint::Length(s_len.try_into().unwrap_or(u16::MAX)),
        BasicConstraint::Length(c_len.try_into().unwrap_or(u16::MAX)),
        BasicConstraint::Length(d_len.try_into().unwrap_or(u16::MAX)),
    ];
    let headings = ["Key", "Context", "Command"].into_iter();
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    draw_panel_mut_impl(
        f,
        w,
        area,
        true,
        |_| "Help".into(),
        |t, f, chunk| {
            let commands_table = t.get_help_list_items().map(
                |DisplayableKeyAction {
                     keybinds,
                     context,
                     description,
                 }| { [keybinds, context, description].into_iter() },
            );
            let (new_state, effect) = draw_table_impl(
                f,
                chunk,
                t.help.cur,
                None,
                &t.help.widget_state,
                commands_table,
                items,
                &table_constraints,
                headings,
                None,
            );
            t.help.widget_state = new_state;
            Some(effect)
        },
    );
}

/// Draw a text input box
pub fn draw_text_box(
    f: &mut Frame,
    title: impl AsRef<str>,
    contents: &mut TextInputState,
    chunk: Rect,
) {
    let block_widget = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(SELECTED_BORDER_COLOUR))
        .title(title.as_ref());
    let text_chunk = block_widget.inner(chunk);
    let text_chunk = Rect {
        x: text_chunk.x,
        y: text_chunk.y,
        width: text_chunk.width.saturating_sub(1),
        height: text_chunk.height,
    };
    // TODO: Scrolling, if input larger than box.
    let text_widget = TextInput::new();
    f.render_widget(block_widget, chunk);
    f.render_stateful_widget(text_widget, text_chunk, contents);
    if let Some(cursor_pos) = contents.screen_cursor() {
        f.set_cursor_position(cursor_pos)
    };
}
