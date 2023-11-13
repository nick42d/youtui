use ratatui::prelude::{Constraint, Direction, Layout, Rect};

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
/// Helper function to create a popup below a chunk.
pub fn below_left_rect(height: u16, width: u16, r: Rect) -> Rect {
    Rect {
        x: r.x,
        y: r.y + r.height - 1,
        width,
        height,
    }
}
/// Helper function to get the bottom line of a chunk, ignoring side borders.
pub fn bottom_of_rect(r: Rect) -> Rect {
    Rect {
        x: r.x + 1,
        y: r.y + r.height - 1,
        width: r.width - 2,
        height: 1,
    }
}
