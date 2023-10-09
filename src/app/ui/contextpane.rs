use std::borrow::Cow;

use ratatui::{
    prelude::{Backend, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Row, Table},
    Frame,
};

use super::{
    actionhandler::{Action, EventHandler, KeyRouter},
    panel::Drawable,
};

// A pane of the application. This is the place that renders in the app and handles key events.
// If a pending key event exists, a popup will be drawn outlining the next commands.
pub trait ContextPane<A: Action + Clone>: EventHandler<A> + KeyRouter<A> + Drawable {
    fn context_name(&self) -> Cow<'static, str>;
    // Should be at app level instead of ContextPane level.
    fn help_shown(&self) -> bool;
    fn draw_popup<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        draw_popup(f, self, chunk);
    }
    fn draw_help<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        draw_help(f, self, chunk);
    }
    fn draw_context_chunk<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        self.draw_chunk(f, chunk);
        if self.help_shown() {
            self.draw_help(f, chunk);
        }
        if self.key_pending() {
            self.draw_popup(f, chunk);
        }
    }
    fn draw_context<B: Backend>(&self, f: &mut Frame<B>) {
        self.draw_context_chunk(f, f.size());
    }
}

enum _Direction {
    Up,
    Down,
    Left,
    Right,
}
// A window context containing multiple panes for which input should be easily swapped.
trait MultiPane {
    fn select(&mut self, dir: _Direction);
    // For example, tabcycling
    fn select_next(&mut self);
}

fn draw_help<A: Action + Clone, B: Backend, C: ContextPane<A> + ?Sized>(
    f: &mut Frame<B>,
    context: &C,
    chunk: Rect,
) {
    let commands = context.get_all_keybinds();
    // Vector of keybind fields.
    let commands_zip: Vec<_> = commands
        .map(|c| (format!("{c}"), c.context(), c.describe()))
        .collect();
    // Get the maximum length of each element in the tuple vector created above.
    let (mut s_len, mut c_len, mut d_len) = commands_zip
        .iter()
        .map(|(s, c, d)| (s.len(), c.len(), d.len()))
        .fold((0, 0, 0), |(smax, cmax, dmax), (s, c, d)| {
            (smax.max(s), cmax.max(c), dmax.max(d))
        });
    // Ensure the width of each column is at least as wide as header.
    (s_len, c_len, d_len) = (s_len.max(3), c_len.max(7), d_len.max(7));
    // Total block width required, including padding and borders.
    let width = s_len + c_len + d_len + 4;
    // Total block height required, including header and borders.
    let height = commands_zip.len() + 3;
    let mut commands_vec = Vec::new();
    // Naive implementation
    for (s, c, d) in commands_zip {
        commands_vec.push(
            Row::new(vec![format!("{}", s), format!("{c}"), format!("{}", d)])
                .style(Style::new().fg(Color::White)),
        );
    }
    let table_constraints = [
        Constraint::Min(s_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(c_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(d_len.try_into().unwrap_or(u16::MAX)),
    ];
    // let table_constraints = [
    //     Constraint::Length(shortcut_width + 10),
    //     Constraint::Length(description_width),
    // ];
    let block = Table::new(commands_vec)
        .header(Row::new(vec!["Key", "Context", "Command"]))
        .style(Style::new().fg(Color::White))
        .block(
            Block::default()
                // TODO: Remove borrow.
                .title("Help")
                .borders(Borders::ALL)
                .style(Style::new().fg(Color::Cyan)),
        )
        .widths(&table_constraints);
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}

fn draw_popup<A: Action + Clone, B: Backend, C: ContextPane<A> + ?Sized>(
    f: &mut Frame<B>,
    context: &C,
    chunk: Rect,
) {
    let title = context.context_name();
    let commands = context.get_cur_mode();
    let shortcuts = commands.iter().map(|c| format!("{c}"));
    let descriptions = commands.iter().map(|c| c.describe());
    // Cloning here only clones iterators, so it's low-cost.
    let shortcut_len = shortcuts.clone().map(|s| s.len()).max().unwrap_or_default();
    let description_len = descriptions
        .clone()
        .map(|d| d.len())
        .max()
        .unwrap_or_default();
    let commands_zip: Vec<_> = shortcuts.zip(descriptions).collect();
    let width = shortcut_len + description_len + 3;
    let height = commands.len() + 2;
    let mut commands_vec = Vec::new();
    for (s, d) in commands_zip {
        commands_vec.push(
            Row::new(vec![format!("{}", s), format!("{}", d)]).style(Style::new().fg(Color::White)),
        );
    }
    let table_constraints = [
        Constraint::Min(shortcut_len.try_into().unwrap_or(u16::MAX)),
        Constraint::Min(description_len.try_into().unwrap_or(u16::MAX)),
    ];
    // let table_constraints = [
    //     Constraint::Length(shortcut_width + 10),
    //     Constraint::Length(description_width),
    // ];
    let block = Table::new(commands_vec)
        .style(Style::new().fg(Color::White))
        .block(
            Block::default()
                .title(title.as_ref())
                .borders(Borders::ALL)
                .style(Style::new().fg(Color::Cyan)),
        )
        .widths(&table_constraints);
    let area = left_bottom_corner_rect(
        height.try_into().unwrap_or(u16::MAX),
        width.try_into().unwrap_or(u16::MAX),
        chunk,
    );
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}
/// Helper function to create a popup at bottom corner of chunk.
pub fn left_bottom_corner_rect(height: u16, width: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(height)].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(width)].as_ref())
        .split(popup_layout[1])[1]
}
