use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{HighlightSpacing, List, ListItem, ListState, StatefulWidget};
use std::borrow::Cow;

#[derive(Debug)]
pub struct ScrollingListState {
    pub list_state: ListState,
    // Tick recorded last time the user changed the selected item.
    pub last_scrolled_tick: u64,
}

impl ScrollingListState {
    pub fn select(&mut self, index: Option<usize>, cur_tick: u64) {
        if self.list_state.selected() != index {
            tracing::info!("Resetting tick");
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
    cur_tick: u64,
}

impl<'a, I> ScrollingList<'a, I> {
    /// cur_tick should represent a monotonically increasing tick count
    /// passed on every render, to determine list scroll position.
    pub fn new<II>(items: I, cur_tick: u64) -> ScrollingList<'a, I>
    where
        I: IntoIterator<Item = II> + 'a,
        II: Into<Cow<'a, str>>,
    {
        Self {
            items,
            cur_tick,
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
        } = self;
        let cur_selected = state.list_state.selected();
        const BLANK_CHARS: u32 = 4;
        let items = items
            .into_iter()
            .map(|item| -> Cow<str> { item.into() })
            .enumerate()
            .map(|(idx, item)| {
                if Some(idx) == cur_selected {
                    return get_scrolled_line(
                        item,
                        // Doesn't seem to be quite working...
                        cur_tick.saturating_sub(state.last_scrolled_tick),
                        BLANK_CHARS,
                        area.width as usize,
                    )
                    .into();
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

fn get_scrolled_line<'a>(
    text: impl Into<Cow<'a, str>>,
    cur_tick: u64,
    blank_chars: u32,
    col_width: usize,
) -> Line<'a> {
    let text = text.into();
    let Some((chars_to_remove, blank_chars)) =
        get_offset(cur_tick, blank_chars as u64, text.len(), col_width)
    else {
        return Line::from(text);
    };
    return match text {
        Cow::Borrowed(b) => {
            let (front, back) = b.split_at(chars_to_remove);
            Line::from_iter([
                Cow::Borrowed(back),
                Cow::Owned(" ".repeat(blank_chars)),
                Cow::Borrowed(front),
            ])
        }
        Cow::Owned(mut o) => {
            let back_half = o.split_off(chars_to_remove);
            Line::from_iter([
                Cow::Owned(back_half),
                Cow::Owned(" ".repeat(blank_chars)),
                Cow::Owned(o),
            ])
        }
    };
}

/// Number of characters to remove from front of string to fit it in the column,
/// number of blank characters. Or, no adjustment at all.
/// ```
/// // If string is shorter than column, offset is always zero.
/// let no_adjustment_needed = get_offset(12, 0, 0, 14, 16);
/// assert_eq!(no_adjustment_needed, 0);
/// ```
fn get_offset(
    cur_tick: u64,
    gap_size: u64,
    string_len: usize,
    col_width: usize,
) -> Option<(usize, usize)> {
    if string_len <= col_width {
        return None;
    }
    let n_frames = u64::try_from(string_len).unwrap().saturating_add(gap_size);
    let frame = cur_tick % n_frames;
    let chars_to_remove = usize::try_from(frame).unwrap().min(string_len);
    let blank_chars = (string_len + usize::try_from(gap_size).unwrap())
        .saturating_sub(frame.try_into().unwrap())
        .min(gap_size.try_into().unwrap());
    Some((chars_to_remove, blank_chars))
}
