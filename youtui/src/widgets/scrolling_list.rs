use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, ListState, StatefulWidget};
use std::borrow::Cow;

pub const DEFAULT_TICKER_GAP: u16 = 4;

#[derive(Debug, Default, Clone)]
pub struct ScrollingListState {
    pub list_state: ListState,
    // Tick recorded last time the user changed the selected item.
    pub last_scrolled_tick: u64,
}

impl ScrollingListState {
    pub fn select(&mut self, index: Option<usize>, cur_tick: u64) {
        if self.list_state.selected() != index {
            tracing::info!("Resetting tick to {cur_tick}");
            self.last_scrolled_tick = cur_tick;
        }
        self.list_state.select(index);
    }
}

pub struct ScrollingList<'a, I> {
    /// The items in the list
    items: I,
    /// Style used as a base style for the widget
    style: Style,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    highlight_symbol: Option<&'a str>,
    /// Monotonically increasing tick count
    cur_tick: u64,
    /// Gap between end of text and start of text (when wrapping around)
    ticker_gap: u16,
}

impl<'a, I> ScrollingList<'a, I> {
    /// `cur_tick` should represent a monotonically and periodically increasing
    /// tick count passed on every render, to determine list scroll frame.
    pub fn new<II>(items: I, cur_tick: u64) -> ScrollingList<'a, I>
    where
        I: IntoIterator<Item = II> + 'a,
        II: Into<Cow<'a, str>>,
    {
        Self {
            items,
            cur_tick,
            ticker_gap: DEFAULT_TICKER_GAP,
            style: Default::default(),
            highlight_style: Default::default(),
            highlight_symbol: Default::default(),
        }
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.highlight_style = style.into();
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    /// Set gap between end of text and start of text (when wrapping around).
    /// Default = [DEFAULT_TICKER_GAP]
    /// ```
    /// assert_eq!(DEFAULT_TICKER_GAP, 4);
    /// ```
    pub fn _ticker_gap(mut self, ticker_gap: u16) -> Self {
        self.ticker_gap = ticker_gap;
        self
    }
}

impl<'a, I, II> StatefulWidget for ScrollingList<'a, I>
where
    I: IntoIterator<Item = II> + 'a,
    II: Into<Cow<'a, str>>,
{
    type State = ScrollingListState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let Self {
            items,
            style,
            highlight_style,
            highlight_symbol,
            cur_tick,
            ticker_gap,
        } = self;
        let cur_selected = state.list_state.selected();
        let adj_tick = cur_tick.saturating_sub(state.last_scrolled_tick);
        let items = items.into_iter().enumerate().map(|(idx, item)| {
            let item: Cow<_> = item.into();
            if Some(idx) == cur_selected {
                return get_scrolled_line(item, adj_tick, ticker_gap, area.width).into();
            }
            ListItem::from(item)
        });
        let list = List::new(items)
            .style(style)
            .highlight_style(highlight_style);
        let list = if let Some(highlight_symbol) = highlight_symbol {
            list.highlight_symbol(highlight_symbol)
        } else {
            list
        };
        list.render(area, buf, &mut state.list_state);
    }
}

/// Returns a Line, scrolled like a stock ticker, with `blank_chars` between end
/// of text and start of text.
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
) -> Line<'a> {
    let text = text.into();
    let (chars_to_remove, blank_chars) =
        get_split_point_and_blanks(cur_tick, blank_chars, text.len(), col_width);
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

/// Gets the point to split the text and the number of blank characters to
/// generate.
fn get_split_point_and_blanks(
    cur_tick: u64,
    gap_size: u16,
    string_len: usize,
    col_width: u16,
) -> (usize, u16) {
    if string_len <= col_width as usize {
        return (0, 0);
    }
    let n_frames = string_len.saturating_add(gap_size as usize);
    let frame_u64 = cur_tick % (u64::try_from(n_frames).unwrap_or(u64::MAX));
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
    (chars_to_remove, blank_chars as u16)
}

#[cfg(test)]
mod tests {
    use crate::widgets::scrolling_list::get_split_point_and_blanks;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_split_point_in_middle() {
        // On third tick frame, skip the first 3 characters, display rest of text, then
        // blanks, then start of text.
        let example = get_split_point_and_blanks(3, 4, 22, 16);
        assert_eq!(example, (3, 4));
    }
    #[test]
    fn test_split_point_string_shorter_than_column() {
        // If string is shorter than column, there is no split point or blank
        // characters.
        let no_adjustment_needed = get_split_point_and_blanks(12, 4, 14, 16);
        assert_eq!(no_adjustment_needed, (0, 0));
    }
    #[test]
    fn test_split_point_end_of_ticker_less_blanks() {
        // when at the very end of the ticker, only a couple of blank characters then
        // the entire string.
        let only_some_blanks = get_split_point_and_blanks(24, 4, 22, 16);
        assert_eq!(only_some_blanks, (22, 2));
    }
}
