use crate::widgets::get_scrolled_line;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Cell, Row, StatefulWidget, Table, TableState};
use std::borrow::Cow;

pub const DEFAULT_TICKER_GAP: u16 = 4;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ScrollingTableState {
    pub table_state: TableState,
    // Tick recorded last time the user changed the selected item.
    pub last_scrolled_tick: u64,
}

impl ScrollingTableState {
    pub fn select(&mut self, index: Option<usize>, cur_tick: u64) {
        if self.table_state.selected() != index {
            self.last_scrolled_tick = cur_tick;
        }
        self.table_state.select(index);
    }
    pub fn offset(&self) -> usize {
        self.table_state.offset()
    }
    pub fn offset_mut(&mut self) -> &mut usize {
        self.table_state.offset_mut()
    }
}

pub struct ScrollingTable<I, H> {
    /// The items in the list
    items: I,
    /// The headings
    headings: H,
    /// Style used as a base style for the widget
    style: Style,
    /// Style used as a base style for the headings
    headings_style: Style,
    /// Style used to render selected item
    row_highlight_style: Style,
    /// Spacing between columns
    column_spacing: u16,
    /// Monotonically increasing tick count
    cur_tick: u64,
    /// Min gap between end of text and start of text (when wrapping around)
    min_ticker_gap: u16,
    /// Column widths
    table_widths: Vec<Constraint>,
    /// Maximum number of times to scroll text before stopping (None for
    /// unlimited).
    max_times_to_scroll: Option<u16>,
}

impl<I, H> ScrollingTable<I, H> {
    /// `cur_tick` should represent a monotonically and periodically increasing
    /// tick count passed on every render, to determine list scroll frame.
    pub fn new<'a, C, II>(
        items: I,
        headings: H,
        table_widths: Vec<Constraint>,
        cur_tick: u64,
    ) -> ScrollingTable<I, H>
    where
        H: IntoIterator<Item = C>,
        C: Into<Cell<'static>>,
        I: IntoIterator<Item = II>,
        II: IntoIterator<Item = Cow<'a, str>> + 'a,
    {
        Self {
            items,
            headings,
            cur_tick,
            table_widths,
            min_ticker_gap: DEFAULT_TICKER_GAP,
            max_times_to_scroll: None,
            style: Default::default(),
            row_highlight_style: Default::default(),
            headings_style: Default::default(),
            column_spacing: Default::default(),
        }
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn row_highlight_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.row_highlight_style = style.into();
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn headings_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.headings_style = style.into();
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn column_spacing(mut self, column_spacing: u16) -> Self {
        self.column_spacing = column_spacing;
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    /// Set gap between end of text and start of text (when wrapping around).
    /// Default = [DEFAULT_TICKER_GAP]
    pub fn min_ticker_gap(mut self, min_ticker_gap: u16) -> Self {
        self.min_ticker_gap = min_ticker_gap;
        self
    }
    #[must_use = "method moves the value of self and returns the modified value"]
    /// Set maximum number of times to scroll text before stopping.
    pub fn max_times_to_scroll(mut self, max_times_to_scroll: Option<u16>) -> Self {
        self.max_times_to_scroll = max_times_to_scroll;
        self
    }
}

impl<'a, I, II, H, C> StatefulWidget for ScrollingTable<I, H>
where
    H: IntoIterator<Item = C>,
    C: Into<Cell<'static>>,
    I: IntoIterator<Item = II>,
    II: IntoIterator<Item = Cow<'a, str>> + 'a,
{
    type State = ScrollingTableState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let Self {
            items,
            headings,
            style,
            headings_style,
            row_highlight_style,
            column_spacing,
            cur_tick,
            min_ticker_gap,
            table_widths,
            max_times_to_scroll,
        } = self;
        let cur_selected = state.table_state.selected();
        let adj_tick = cur_tick.saturating_sub(state.last_scrolled_tick);

        /// Copied from ratatui
        fn get_column_widths(
            column_spacing: u16,
            table_widths: &[Constraint],
            max_table_width: u16,
            col_count: usize,
        ) -> Vec<u16> {
            let widths = if table_widths.is_empty() {
                // Divide the space between each column equally
                vec![Constraint::Length(max_table_width / col_count.max(1) as u16); col_count]
            } else {
                table_widths.to_vec()
            };
            let rects = Layout::horizontal(widths)
                .spacing(column_spacing)
                .split(Rect::new(0, 0, max_table_width, 1));
            rects.iter().map(|c| c.width).collect()
        }

        let column_widths = get_column_widths(
            column_spacing,
            &table_widths,
            area.width,
            // XXX: This is a hack to get col count. Must be changed.
            table_widths.len(),
        );
        let items = items.into_iter().enumerate().map(|(idx, row)| {
            if Some(idx) == cur_selected {
                // TODO: See if there is a way to remove allocation (may not be).
                let items_vec: Vec<_> = row.into_iter().collect();
                // Sync scrolling between all columns.
                let max_col_length = items_vec
                    .iter()
                    .max_by_key(|item| item.len())
                    .map(|item| item.len())
                    .unwrap_or_default();
                let row = items_vec
                    .into_iter()
                    .enumerate()
                    // TODO: confirm col_width safety
                    .map(|(idx, item)| {
                        let item_len = item.len();
                        get_scrolled_line(
                            item,
                            adj_tick,
                            (max_col_length.saturating_sub(item_len) + min_ticker_gap as usize)
                                as u16,
                            column_widths[idx],
                            max_times_to_scroll,
                        )
                    });
                return Row::new(row);
            }
            Row::new(row)
        });
        let table = Table::new(items, table_widths)
            .style(style)
            .row_highlight_style(row_highlight_style)
            .column_spacing(column_spacing)
            .header(Row::new(headings).style(headings_style));
        table.render(area, buf, &mut state.table_state);
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::{ScrollingTable, ScrollingTableState};
    use pretty_assertions::assert_eq;
    use ratatui::layout::{Constraint, Rect};
    use ratatui::widgets::StatefulWidget;
    use std::borrow::Cow;

    #[test]
    fn test_basic_scrolling_table_not_scrolled() {
        let headings = ["AA", "ABCD"];
        let table_items = [
            [Cow::from("AA"), Cow::from("ABCD")],
            [Cow::from("AA"), Cow::from("ABCD")],
        ];
        let mut table_state = ScrollingTableState::default();
        table_state.select(Some(1), 0);
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = ratatui::buffer::Buffer::empty(area);

        let table = ScrollingTable::new(
            table_items,
            headings,
            vec![Constraint::Length(2), Constraint::Length(3)],
            0,
        )
        .min_ticker_gap(1);
        table.render(area, &mut buf, &mut table_state);
        let cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_cells_as_string = "AAABCAAABCAAABC".to_string();
        assert_eq!(cells_as_string, expected_cells_as_string);
    }

    #[test]
    fn test_basic_scrolling_scroll_one_frame() {
        let headings = ["AA", "ABCD"];
        let table_items = [
            [Cow::from("AA"), Cow::from("ABCD")],
            [Cow::from("AA"), Cow::from("ABCD")],
        ];
        let mut table_state = ScrollingTableState::default();
        table_state.select(Some(1), 0);
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = ratatui::buffer::Buffer::empty(area);

        let table = ScrollingTable::new(
            table_items,
            headings,
            vec![Constraint::Length(2), Constraint::Length(3)],
            0,
        )
        .min_ticker_gap(1);
        table.render(area, &mut buf, &mut table_state);
        let cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_cells_as_string = "AAABCAAABCAAABC".to_string();
        assert_eq!(cells_as_string, expected_cells_as_string);
    }
}
