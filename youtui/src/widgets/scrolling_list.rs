use crate::widgets::get_scrolled_line;
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

#[cfg(test)]
mod tests {
    use crate::widgets::{ScrollingList, ScrollingListState};
    use pretty_assertions::assert_eq;
    use ratatui::layout::Rect;
    use ratatui::widgets::StatefulWidget;

    #[test]
    fn test_basic_scrolling_list() {
        let list_items = ["AA", "ABCD"];
        let mut list_state = ScrollingListState::default();
        list_state.select(Some(1), 0);
        let area = Rect::new(0, 0, 3, 2);
        let mut buf = ratatui::buffer::Buffer::empty(area);

        // Frame 1 - scrolling hasn't started yet
        let list_frame_1 = ScrollingList::new(list_items, 0)._ticker_gap(1);
        list_frame_1.render(area, &mut buf, &mut list_state);
        let frame_1_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_frame_1_cells_as_string = "AA ABC".to_string();
        assert_eq!(frame_1_cells_as_string, expected_frame_1_cells_as_string);

        // Frame 2 - scrolling only
        let list_frame_2 = ScrollingList::new(list_items, 1)._ticker_gap(1);
        list_frame_2.render(area, &mut buf, &mut list_state);
        let frame_2_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_frame_2_cells_as_string = "AA BCD".to_string();
        assert_eq!(frame_2_cells_as_string, expected_frame_2_cells_as_string);

        // Frame 3 - padding after scrolling
        let list_frame_3 = ScrollingList::new(list_items, 2)._ticker_gap(1);
        list_frame_3.render(area, &mut buf, &mut list_state);
        let frame_3_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_frame_3_cells_as_string = "AA CD ".to_string();
        assert_eq!(frame_3_cells_as_string, expected_frame_3_cells_as_string);

        // Frame 4 - wraparound
        let list_frame_4 = ScrollingList::new(list_items, 3)._ticker_gap(1);
        list_frame_4.render(area, &mut buf, &mut list_state);
        let frame_4_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_frame_4_cells_as_string = "AA D A".to_string();
        assert_eq!(frame_4_cells_as_string, expected_frame_4_cells_as_string);
    }
}
