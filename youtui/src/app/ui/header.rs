use crate::app::component::actionhandler::{KeyRouter, get_global_keybinds_as_readable_iter};
use crate::app::ui::WindowContext;
use crate::app::view::HasContext;
use crate::drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR};
use crate::keyaction::DisplayableKeyAction;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

const TAB_ROWS: u16 = 2;

pub fn header_required_height(w: &super::YoutuiWindow) -> u16 {
    if matches!(w.context, WindowContext::Browser) {
        4
    } else {
        3
    }
}

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
    // Only render the tab menu if Browser is selected.
    if matches!(w.context, WindowContext::Browser) {
        let title = w.browser.context_menu_title();
        let items = w.browser.context_menu_items();
        let selected_item = w.browser.context_menu_selected_item_idx();
        let tabs_block = Block::default().borders(Borders::ALL).title(title);
        let tabs_widget = crate::widgets::TabGrid::new_with_max_rows(items, TAB_ROWS)
            .select(selected_item)
            .highlight_style(Style::new().fg(BUTTON_FG_COLOUR).bg(BUTTON_BG_COLOUR));
        let [commands_chunk, tabs_chunk] = Layout::horizontal([
            Constraint::Min(0),
            // Add two to accommodate block
            Constraint::Max(tabs_widget.required_width().try_into().unwrap_or(u16::MAX) + 2),
        ])
        .areas(chunk);
        f.render_widget(commands_widget, commands_block.inner(commands_chunk));
        f.render_widget(commands_block, commands_chunk);
        f.render_widget(tabs_widget, tabs_block.inner(tabs_chunk));
        f.render_widget(tabs_block, tabs_chunk);
    } else {
        f.render_widget(commands_widget, commands_block.inner(chunk));
        f.render_widget(commands_block, chunk);
    }
}
