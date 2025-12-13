use ratatui::prelude::Rect;
use ratatui::style::Color;

// Standard app colour scheme
pub const SELECTED_BORDER_COLOUR: Color = Color::Cyan;
pub const DESELECTED_BORDER_COLOUR: Color = Color::Reset;
// TODO: Implement in all locations.
pub const TEXT_COLOUR: Color = Color::Reset;
pub const BUTTON_BG_COLOUR: Color = Color::Gray;
pub const BUTTON_FG_COLOUR: Color = Color::Black;
pub const PROGRESS_BG_COLOUR: Color = Color::DarkGray;
pub const PROGRESS_FG_COLOUR: Color = Color::LightGreen;
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
// TODO: Add a test to ensure this is returning correct area
pub fn below_left_rect(height: u16, width: u16, r: Rect, max_bounds: Rect) -> Rect {
    let y = r.y + r.height - 1;
    Rect {
        x: r.x,
        y,
        width: width.min(max_bounds.right().saturating_sub(r.x)),
        height: (height.saturating_add(1)).min(max_bounds.bottom().saturating_sub(y)),
    }
}
/// Helper function to create a popup in the center of a chunk.
pub fn centered_rect(height: u16, width: u16, r: Rect) -> Rect {
    Rect {
        x: (r.x + r.width / 2).saturating_sub(width / 2).max(r.x),
        y: (r.y + r.height / 2).saturating_sub(height / 2).max(r.y),
        width: width.min(r.width),
        height: height.min(r.height),
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
/// Helper function to get the middle line of a chunk, ignoring side borders.
/// Warning: Not currently bounds checked.
pub fn middle_of_rect(r: Rect) -> Rect {
    Rect {
        x: r.x,
        y: r.y + (r.height - 1) / 2,
        width: r.width,
        height: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{below_left_rect, centered_rect, left_bottom_corner_rect};
    use crate::drawutils::middle_of_rect;
    use ratatui::layout::Rect;

    fn bounds_check_rect(r: Rect, max_bounds: Rect) {
        assert!(r.left() >= max_bounds.left());
        assert!(r.right() <= max_bounds.right());
        assert!(r.bottom() <= max_bounds.bottom());
        assert!(r.top() >= max_bounds.top());
    }
    #[test]
    #[should_panic]
    fn test_bounds_check_rect() {
        // TODO: Rect constructor may make this neater.
        let r1 = Rect {
            x: 0,
            y: 0,
            height: 50,
            width: 50,
        };
        let m1 = Rect {
            x: 0,
            y: 50,
            height: 50,
            width: 50,
        };
        let r2 = Rect {
            x: 30,
            y: 30,
            height: 50,
            width: 50,
        };
        let m2 = Rect {
            x: 30,
            y: 30,
            height: 51,
            width: 51,
        };
        let r3 = Rect {
            x: 30,
            y: 30,
            height: 50,
            width: 50,
        };
        let m3 = Rect {
            x: 30,
            y: 30,
            height: 51,
            width: 50,
        };
        let r4 = Rect {
            x: 30,
            y: 30,
            height: 50,
            width: 50,
        };
        let m4 = Rect {
            x: 30,
            y: 30,
            height: 50,
            width: 51,
        };
        let r5 = Rect {
            x: 30,
            y: 30,
            height: 50,
            width: 50,
        };
        let m5 = Rect {
            x: 31,
            y: 31,
            height: 50,
            width: 50,
        };
        bounds_check_rect(r1, m1);
        bounds_check_rect(r2, m2);
        bounds_check_rect(r3, m3);
        bounds_check_rect(r4, m4);
        bounds_check_rect(r5, m5);
    }
    // These don't actually do anything as they don't try to draw...
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
        let t_r1 = Rect {
            x: 0,
            y: 0,
            height: 50,
            width: 50,
        };
        let t_r2 = Rect {
            x: 0,
            y: 50,
            height: 50,
            width: 50,
        };
        let t_r3 = Rect {
            x: 50,
            y: 0,
            height: 50,
            width: 50,
        };
        let t_r4 = Rect {
            x: 50,
            y: 50,
            height: 50,
            width: 50,
        };
        let r1 = centered_rect(u16::MAX, u16::MAX, t_r1);
        let r2 = centered_rect(u16::MAX, u16::MAX, t_r2);
        let r3 = centered_rect(u16::MAX, u16::MAX, t_r3);
        let r4 = centered_rect(u16::MAX, u16::MAX, t_r4);
        // Unsure if these are correct of there is a better way to check.
        // TODO: Add a bounds check rect function.
        bounds_check_rect(r1, t_r1);
        bounds_check_rect(r2, t_r2);
        bounds_check_rect(r3, t_r3);
        bounds_check_rect(r4, t_r4);
    }
    #[test]
    fn test_middle_of_rect() {
        let r1 = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 3,
        };
        assert_eq!(
            middle_of_rect(r1),
            Rect {
                x: 0,
                y: 1,
                width: 10,
                height: 1
            }
        );
        let r2 = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        assert_eq!(
            middle_of_rect(r2),
            Rect {
                x: 0,
                y: 4,
                width: 10,
                height: 1
            }
        );
        let r3 = Rect {
            x: 0,
            y: 10,
            width: 10,
            height: 5,
        };
        assert_eq!(
            middle_of_rect(r3),
            Rect {
                x: 0,
                y: 12,
                width: 10,
                height: 1
            }
        );
    }
    #[test]
    fn bounds_check_below_left_rect() {
        // TODO: Add more / generalized test cases.
        // TODO: Check hasn't exceeded max_bounds.
        // TODO: Check has appeared where we want it to.
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
