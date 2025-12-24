use ratatui::text::Line;
pub use scrolling_list::{ScrollingList, ScrollingListState};
pub use scrolling_table::{ScrollingTable, ScrollingTableState};
use std::borrow::Cow;
pub use tab_grid::TabGrid;

mod scrolling_list;
mod scrolling_table;
mod tab_grid;

/// Returns a Line, scrolled like a stock ticker, with `blank_chars` between end
/// of text and start of text (unless `max_times_to_wrap` has been reached).
///
/// Does not scroll if text is shorter than `col_width`.
///
/// `cur_tick` should represent a monotonically and periodically increasing
/// tick count passed on every render, to determine scroll frame.
fn get_scrolled_line<'a>(
    text: impl Into<Cow<'a, str>>,
    cur_tick: u64,
    blank_chars: u16,
    col_width: u16,
    max_times_to_wrap: Option<u16>,
) -> Line<'a> {
    let text = text.into();
    let (chars_to_remove, blank_chars, times_wrapped) =
        get_split_point_and_blanks_and_wrapped(cur_tick, blank_chars, text.len(), col_width);
    if let Some(max_times_to_wrap) = max_times_to_wrap
        && times_wrapped >= max_times_to_wrap as u64
    {
        return Line::from(text);
    }
    match text {
        Cow::Borrowed(b) => {
            // TODO: Handle actual terminal with of string bytes. Currently, this ticker may
            // render incorrectly for Strings containing multi-byte characters.
            let safe_split_point = b.floor_char_boundary(chars_to_remove);
            let (front, back) = b.split_at(safe_split_point);
            Line::from_iter([
                Cow::Borrowed(back),
                Cow::Owned(" ".repeat(blank_chars as usize)),
                Cow::Borrowed(front),
            ])
        }
        Cow::Owned(mut o) => {
            // TODO: Handle actual terminal with of string bytes. Currently, this ticker may
            // render incorrectly for Strings containing multi-byte characters.
            let safe_split_point = o.floor_char_boundary(chars_to_remove);
            let back_half = o.split_off(safe_split_point);
            Line::from_iter([
                Cow::Owned(back_half),
                Cow::Owned(" ".repeat(blank_chars as usize)),
                Cow::Owned(o),
            ])
        }
    }
}

/// Gets the point to split the text, the number of blank characters to
/// generate, and a number representing number of times text wrapped.
///
/// Panics if string_len + gap_size = 0
fn get_split_point_and_blanks_and_wrapped(
    cur_tick: u64,
    gap_size: u16,
    string_len: usize,
    col_width: u16,
) -> (usize, u16, u64) {
    if string_len <= col_width as usize {
        return (0, 0, 0);
    }
    let n_frames_usize = string_len.saturating_add(gap_size as usize);
    let n_frames_u64 = u64::try_from(n_frames_usize).unwrap_or(u64::MAX);
    let frame_u64 = cur_tick % n_frames_u64;
    let times_wrapped = cur_tick / n_frames_u64;
    // Safe cast, since either usize is bigger than u64, or, frame no bigger than a
    // usize (since the output of <u64> % <usize> can be no bigger than usize)
    let frame = frame_u64 as usize;
    let chars_to_remove = frame.min(string_len);
    let blank_chars = (string_len + gap_size as usize)
        .saturating_sub(frame)
        .min(gap_size as usize);
    debug_assert!(blank_chars <= gap_size as usize);
    // Safe cast, since we are manually asserting gap size to be the maximum value
    // of blank chars above.
    (chars_to_remove, blank_chars as u16, times_wrapped)
}

#[cfg(test)]
mod tests {
    use crate::widgets::get_split_point_and_blanks_and_wrapped;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_split_point_in_middle() {
        // On third tick frame, skip the first 3 characters, display rest of text, then
        // blanks, then start of text.
        let example = get_split_point_and_blanks_and_wrapped(3, 4, 22, 16);
        assert_eq!(example, (3, 4, 0));
    }
    #[test]
    fn test_split_point_in_middle_wrapped() {
        // 29th tick should be equal to third tick, wrapped once.
        let example = get_split_point_and_blanks_and_wrapped(29, 4, 22, 16);
        assert_eq!(example, (3, 4, 1));
    }
    #[test]
    fn test_split_point_string_shorter_than_column() {
        // If string is shorter than column, there is no split point or blank
        // characters.
        let no_adjustment_needed = get_split_point_and_blanks_and_wrapped(12, 4, 14, 16);
        assert_eq!(no_adjustment_needed, (0, 0, 0));
    }
    #[test]
    fn test_split_point_end_of_ticker_less_blanks() {
        // when at the very end of the ticker, only a couple of blank characters then
        // the entire string.
        let only_some_blanks = get_split_point_and_blanks_and_wrapped(24, 4, 22, 16);
        assert_eq!(only_some_blanks, (22, 2, 0));
    }
}
