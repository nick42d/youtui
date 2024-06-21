use crate::{
    app::{component::actionhandler::KeyDisplayer, keycommand::DisplayableCommand},
    drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR},
};
use ratatui::{
    layout::Rect,
    style::Style,
    terminal::Frame,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

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
