use std::borrow::Cow;

use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    terminal::Frame,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::{
    actionhandler::{Action, KeybindVisibility},
    contextpane::ContextPane,
    WindowContext,
};

pub fn context_global_keybinds_and_descriptions<'a, C, A>(
    context: &'a C,
) -> Box<dyn Iterator<Item = (Cow<str>, String)> + 'a>
where
    C: ContextPane<A>,
    A: Action + Clone + 'a,
{
    Box::new(
        context
            .get_keybinds()
            .filter(|kb| kb.visibility == KeybindVisibility::Global)
            .map(|c| (c.describe(), format!("{c}"))),
    )
}

pub fn draw_header<B>(f: &mut Frame<B>, w: &super::YoutuiWindow, chunk: Rect)
where
    B: Backend,
{
    let keybinds = match w.context {
        WindowContext::Browser => context_global_keybinds_and_descriptions(&w.browser),
        WindowContext::Playlist => context_global_keybinds_and_descriptions(&w.playlist),
        WindowContext::Logs => context_global_keybinds_and_descriptions(&w.logger),
    };

    let help_string = Line::from(
        keybinds
            .flat_map(|(d, k)| {
                vec![
                    Span::styled(k, Style::default().bg(Color::Gray).fg(Color::Black)),
                    Span::raw(" "),
                    Span::raw(d),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    );

    let header = Paragraph::new(help_string).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title("Commands"),
    );
    f.render_widget(header, chunk);
}
