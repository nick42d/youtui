use crate::app::component::actionhandler::{get_global_keybinds_as_readable_iter, KeyRouter};
use crate::drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR};
use crate::keyaction::DisplayableKeyAction;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn draw_header(f: &mut Frame, w: &super::YoutuiWindow, chunk: Rect) {
    let keybinds = get_global_keybinds_as_readable_iter(w.get_active_keybinds(&w.config));

    let help_string = Line::from(
        keybinds
            .flat_map(
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
            )
            // XXX: Consider removing allocation
            .collect::<Vec<_>>(),
    );

    let header =
        Paragraph::new(help_string).block(Block::default().borders(Borders::ALL).title("Commands"));
    f.render_widget(header, chunk);
}
