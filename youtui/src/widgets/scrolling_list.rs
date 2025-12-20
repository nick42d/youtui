use ratatui::style::Style;
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
        const PAUSED_TICKS_AFTER_SCROLL: u64 = 10;
        const PAUSED_TICKS_BEFORE_SCROLL: u64 = 10;
        let items = items
            .into_iter()
            .map(|item| -> Cow<str> { item.into() })
            .enumerate()
            .map(|(idx, item)| {
                if Some(idx) == cur_selected {
                    let offset = get_offset(
                        // Doesn't seem to be quite working...
                        cur_tick.saturating_sub(state.last_scrolled_tick),
                        PAUSED_TICKS_BEFORE_SCROLL,
                        PAUSED_TICKS_AFTER_SCROLL,
                        item.len(),
                        area.width as usize,
                    );
                    return match item {
                        Cow::Borrowed(b) => Cow::Borrowed(b.get(offset..).unwrap_or_default()),
                        Cow::Owned(o) => Cow::Owned(o.chars().skip(offset).collect()),
                    };
                }
                item
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

/// Number of characters to remove from front of string to fit it in the column.
/// ```
/// // If string is shorter than column, offset is always zero.
/// let no_adjustment_needed = get_offset(12, 0, 0, 14, 16);
/// assert_eq!(no_adjustment_needed, 0);
/// ```
fn get_offset(
    cur_tick: u64,
    pause_front: u64,
    pause_back: u64,
    string_len: usize,
    col_width: usize,
) -> usize {
    let max_adjustment = string_len.saturating_sub(col_width);
    if max_adjustment == 0 {
        return 0;
    }
    let n_frames = u64::try_from(max_adjustment).unwrap() + pause_front + pause_back;
    let frame = cur_tick % n_frames;
    frame
        .saturating_sub(pause_front)
        .min(max_adjustment.try_into().unwrap())
        .try_into()
        .unwrap()
}
