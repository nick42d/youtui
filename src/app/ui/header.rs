use crate::{
    app::{
        component::actionhandler::{Action, KeyDisplayer, KeyHandler},
        keycommand::{CommandVisibility, DisplayableCommand},
    },
    drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR},
};
use ratatui::{
    layout::Rect,
    style::Style,
    terminal::Frame,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::borrow::Cow;

pub fn _context_global_keybinds_and_descriptions<'a, C, A>(
    context: &'a C,
) -> Box<dyn Iterator<Item = (Cow<str>, String)> + 'a>
where
    C: KeyHandler<A>,
    A: Action + Clone + 'a,
{
    Box::new(
        context
            .get_keybinds()
            .filter(|kb| kb.visibility == CommandVisibility::Global)
            .map(|c| (c.describe(), format!("{c}"))),
    )
}

pub fn draw_header(f: &mut Frame, w: &super::YoutuiWindow, chunk: Rect) {
    let keybinds = w.get_context_global_keybinds_as_readable_iter();

    let help_string = Line::from(
        keybinds
            .flat_map(
                |DisplayableCommand {
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
