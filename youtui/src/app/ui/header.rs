use crate::{
    app::{
        component::actionhandler::get_active_global_keybinds_as_readable_iter,
        keycommand::DisplayableCommand,
    },
    drawutils::{BUTTON_BG_COLOUR, BUTTON_FG_COLOUR},
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_header(f: &mut Frame, w: &super::YoutuiWindow, chunk: Rect) {
    let keybinds = get_active_global_keybinds_as_readable_iter(w);

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
