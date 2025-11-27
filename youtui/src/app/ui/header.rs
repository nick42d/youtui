use crate::app::component::actionhandler::{get_global_keybinds_as_readable_iter, KeyRouter};
use crate::drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR};
use crate::keyaction::DisplayableKeyAction;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
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
    let header = Paragraph::new(help_string);
    let split = Layout::horizontal([Constraint::Min(0), Constraint::Max(19)]).split(chunk);
    let tabs =
        crate::widgets::TabGrid::new_with_cols(["Search", "Library", "Playlist", "Other"], 2);
    f.render_widget(header, block.inner(split[0]));
    f.render_widget(tabs, block.inner(split[1]));
    f.render_widget(block, split[0]);
    f.render_widget(block2, split[1]);
}
