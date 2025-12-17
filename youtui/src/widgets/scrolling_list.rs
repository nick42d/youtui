use ratatui::style::Style;
use ratatui::widgets::{HighlightSpacing, List, ListItem, ListState, StatefulWidget};
use std::borrow::Cow;

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
}

impl<'a, I, II> StatefulWidget for ScrollingList<'a, I>
where
    I: IntoIterator<Item = II> + 'a,
    II: Into<Cow<'a, str>>,
{
    type State = ListState;

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
        let cur_selected = state.selected();
        let width = area.width;
        let offset: usize = (cur_tick % width as u64).try_into().unwrap();
        let items = items.into_iter().enumerate().map(|(idx, item)| {
            if Some(idx) == cur_selected {
                return item.into().get(offset..).unwrap_or_default().into();
            }
            item.into()
        });
        let list = List::new(items)
            .style(style)
            .highlight_style(highlight_style);
        let list = if let Some(highlight_symbol) = highlight_symbol {
            list.highlight_symbol(highlight_symbol)
        } else {
            list
        };
        list.render(area, buf, state);
    }
}
