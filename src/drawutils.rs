use ratatui::prelude::{Constraint, Direction, Layout, Rect};

/// Helper function to create a popup at bottom corner of chunk.
pub fn left_bottom_corner_rect(height: u16, width: u16, r: Rect) -> Rect {
    Rect {
        x: (r.x + r.width).saturating_sub(width),
        y: (r.y + r.height).saturating_sub(height),
        width,
        height,
    }
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
/// Helper function to create a popup in the center of a chunk.
pub fn centered_rect(height: u16, width: u16, r: Rect) -> Rect {
    Rect {
        x: (r.x) + r.width / 2 - width / 2,
        y: (r.y) + r.height / 2 - height / 2,
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
