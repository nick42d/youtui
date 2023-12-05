use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
};

// Standard app colour scheme
pub const SELECTED_BORDER_COLOUR: Color = Color::Cyan;
pub const DESELECTED_BORDER_COLOUR: Color = Color::Reset;
// TODO: Implement in all locations.
pub const TEXT_COLOUR: Color = Color::Reset;
pub const BUTTON_BG_COLOUR: Color = Color::Gray;
pub const BUTTON_FG_COLOUR: Color = Color::Black;
pub const PROGRESS_BG_COLOUR: Color = Color::DarkGray;
pub const PROGRESS_FG_COLOUR: Color = Color::LightGreen;
pub const PROGRESS_ELAPSED_COLOUR: Color = Color::LightGreen;
pub const TABLE_HEADINGS_COLOUR: Color = Color::LightGreen;
pub const ROW_HIGHLIGHT_COLOUR: Color = Color::Blue;

/// Helper function to create a popup at bottom corner of chunk.
pub fn left_bottom_corner_rect(height: u16, width: u16, r: Rect) -> Rect {
    let r_x2 = r.x + r.width;
    let r_y2 = r.y + r.height;
    let x = r_x2.saturating_sub(width).max(r.x);
    let y = r_y2.saturating_sub(height).max(r.y);
    Rect {
        x,
        y,
        width: width.min(r_x2 - x),
        height: height.min(r_y2 - y),
    }
}
/// Helper function to create a popup below a chunk.
//  We pass in the max bounds that can be rendered by the application,
//  to avoid returning a Rect that is not drawable.
pub fn below_left_rect(height: u16, width: u16, r: Rect, max_bounds: Rect) -> Rect {
    Rect {
        x: r.x,
        y: (r.y + r.height - 1),
        width: width.min(max_bounds.x.saturating_sub(r.x)),
        height: height.min(max_bounds.y.saturating_sub(r.y)),
    }
}
/// Helper function to create a popup in the center of a chunk.
pub fn centered_rect(height: u16, width: u16, r: Rect) -> Rect {
    Rect {
        x: (r.x + r.width / 2).saturating_sub(width / 2).max(r.x),
        y: (r.y + r.height / 2).saturating_sub(height / 2).max(r.y),
        width: width.min(r.width),
        height: width.min(r.height),
    }
}
/// Helper function to get the bottom line of a chunk, ignoring side borders.
/// Warning: Not currently bounds checked.
pub fn bottom_of_rect(r: Rect) -> Rect {
    Rect {
        x: r.x + 1,
        y: r.y + r.height - 1,
        width: r.width - 2,
        height: 1,
    }
}

/// Return the standard list / table highlight style
pub fn highlight_style() -> Style {
    Style::new().bg(ROW_HIGHLIGHT_COLOUR)
}

mod tests {
    use super::{below_left_rect, centered_rect, left_bottom_corner_rect};
    use ratatui::layout::Rect;

    #[test]
    fn bounds_check_left_bottom_corner_rect() {
        left_bottom_corner_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 0,
                height: 50,
                width: 50,
            },
        );
        left_bottom_corner_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 50,
                height: 50,
                width: 50,
            },
        );
        left_bottom_corner_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 0,
                height: 50,
                width: 50,
            },
        );
        left_bottom_corner_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 50,
                height: 50,
                width: 50,
            },
        );
    }
    #[test]
    fn bounds_check_centered_rect() {
        centered_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 0,
                height: 50,
                width: 50,
            },
        );
        centered_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 50,
                height: 50,
                width: 50,
            },
        );
        centered_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 0,
                height: 50,
                width: 50,
            },
        );
        centered_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 50,
                height: 50,
                width: 50,
            },
        );
    }
    #[test]
    fn bounds_check_below_left_rect() {
        // TODO: Add more / generalized test cases.
        below_left_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 0,
                height: 50,
                width: 50,
            },
            Rect {
                x: 100,
                y: 100,
                height: 1050,
                width: 1050,
            },
        );
        below_left_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 0,
                y: 50,
                height: 50,
                width: 50,
            },
            Rect {
                x: 100,
                y: 1050,
                height: 1050,
                width: 1050,
            },
        );
        below_left_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 0,
                height: 50,
                width: 50,
            },
            Rect {
                x: 1050,
                y: 100,
                height: 1050,
                width: 1050,
            },
        );
        below_left_rect(
            u16::MAX,
            u16::MAX,
            Rect {
                x: 50,
                y: 50,
                height: 50,
                width: 50,
            },
            Rect {
                x: 1050,
                y: 1050,
                height: 1050,
                width: 1050,
            },
        );
    }
}
