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
