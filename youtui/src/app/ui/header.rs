use crate::app::component::actionhandler::{KeyRouter, get_global_keybinds_as_readable_iter};
use crate::drawutils::{
    BUTTON_BG_COLOUR, BUTTON_FG_COLOUR, DESELECTED_BORDER_COLOUR, SELECTED_BORDER_COLOUR,
};
use crate::keyaction::DisplayableKeyAction;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::fmt::Debug;

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
    let block = Block::default().borders(Borders::ALL).title("Commands");
    let block2 = Block::default().borders(Borders::ALL).title("Mode");
    let header = Paragraph::new(help_string).wrap(Wrap { trim: true });
    let split = Layout::horizontal([Constraint::Min(0), Constraint::Max(19)]).split(chunk);
    let selected = match w.context {
        super::WindowContext::Browser => 0,
        super::WindowContext::Playlist => 1,
        super::WindowContext::Logs => 2,
    };
    let tabs = crate::widgets::TabGrid::new_with_cols(["Search", "Playlist", "Logs"], 2)
        .select(selected)
        .highlight_style(Style::new().fg(BUTTON_FG_COLOUR).bg(BUTTON_BG_COLOUR));
    f.render_widget(header, block.inner(split[0]));
    f.render_widget(tabs, block.inner(split[1]));
    f.render_widget(block, split[0]);
    f.render_widget(block2, split[1]);
}
