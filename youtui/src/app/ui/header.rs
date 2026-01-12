use crate::app::component::actionhandler::{KeyRouter, get_global_keybinds_as_readable_iter};
use crate::app::view::HasContext;
use crate::drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR};
use crate::keyaction::DisplayableKeyAction;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

const TAB_COLS: u16 = 2;

pub fn draw_header(f: &mut Frame, w: &super::YoutuiWindow, chunk: Rect) {
    let keybinds = get_global_keybinds_as_readable_iter(w.get_active_keybinds(&w.config));

    let help_string = Line::from_iter(keybinds.flat_map(
        |DisplayableKeyAction {
             keybinds,
             description,
             ..
         }| {
            vec![
                Span::styled(
                    keybinds,
                    Style::default().bg(BUTTON_BG_COLOUR).fg(BUTTON_FG_COLOUR),
                ),
                Span::raw(" "),
                Span::raw(description),
                Span::raw(" "),
            ]
        },
    ));
    let commands_block = Block::default().borders(Borders::ALL).title("Commands");
    let commands_widget = Paragraph::new(help_string).wrap(Wrap { trim: true });
    let title = w.browser.context_menu_title();
    let items = w.browser.context_menu_items();
    let selected_item = w.browser.context_menu_selected_item_idx();
    let mode_block = Block::default().borders(Borders::ALL).title(title);
    let mode_widget = crate::widgets::TabGrid::new_with_cols(items, TAB_COLS)
        .select(selected_item)
        .highlight_style(Style::new().fg(BUTTON_FG_COLOUR).bg(BUTTON_BG_COLOUR));
    let split = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Max(mode_widget.required_width().try_into().unwrap_or(u16::MAX)),
    ])
    .split(chunk);
    f.render_widget(commands_widget, commands_block.inner(split[0]));
    f.render_widget(mode_widget, mode_block.inner(split[1]));
    f.render_widget(commands_block, split[0]);
    f.render_widget(mode_block, split[1]);
}
