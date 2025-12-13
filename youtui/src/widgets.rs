use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use std::borrow::Cow;

/// Ratatui widgets used in application
pub struct TabGrid<'a, const N: usize> {
    titles: [Cow<'a, str>; N],
    selected: Option<usize>,
    cols: u16,
    highlight_style: Option<Style>,
    style: Style,
}
impl<'a, const N: usize> TabGrid<'a, N> {
    pub fn new_with_cols(titles: [impl Into<Cow<'a, str>>; N], cols: u16) -> Self {
        Self {
            titles: titles.map(|title| title.into()),
            selected: None,
            cols,
            highlight_style: Default::default(),
            style: Default::default(),
        }
    }
    /// zero indexed
    pub fn select(self, selected: usize) -> Self {
        let Self {
            titles,
            cols,
            highlight_style,
            style,
            ..
        } = self;
        Self {
            titles,
            selected: Some(selected),
            cols,
            highlight_style,
            style,
        }
    }
    #[allow(unused)]
    // This is a library type module and its expected all methods on TabGrid
    // will be eventually used.
    pub fn deselect(self) -> Self {
        let Self {
            titles,
            cols,
            highlight_style,
            style,
            ..
        } = self;
        Self {
            titles,
            selected: None,
            cols,
            highlight_style,
            style,
        }
    }
    /// Sets the style for the highlighted tab - overwriting the base style.
    pub fn highlight_style(self, highlight_style: Style) -> Self {
        let Self {
            titles,
            cols,
            selected,
            style,
            ..
        } = self;
        Self {
            titles,
            selected,
            cols,
            highlight_style: Some(highlight_style),
            style,
        }
    }
    /// Sets the style for all tabs.
    #[allow(unused)]
    // This is a library type module and its expected all methods on TabGrid
    // will be eventually used.
    pub fn style(self, style: Style) -> Self {
        let Self {
            titles,
            cols,
            selected,
            highlight_style,
            ..
        } = self;
        Self {
            titles,
            selected,
            cols,
            highlight_style,
            style,
        }
    }
    /// Returns 0 if there are 0 cols or 0 titles.
    pub fn required_width(&self) -> usize {
        self.longest_title()
            .saturating_mul(self.cols as usize)
            .saturating_add(self.cols as usize)
            .saturating_sub(1)
    }
    /// Returns 0 if there are 0 cols (instead of panicing)
    pub fn required_height(&self) -> usize {
        if self.cols == 0 {
            return 0;
        }
        self.titles.len().div_ceil(self.cols as usize)
    }
    /// Returns 0 if there are 0 titles.
    fn longest_title(&self) -> usize {
        self.titles
            .iter()
            .map(|title| title.len())
            .max()
            .unwrap_or_default()
    }
}
impl<'a, const N: usize> Widget for TabGrid<'a, N> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // Do nothing if cols is 0.
        if self.cols == 0 {
            return;
        }
        let longest_title = self.longest_title();
        let rows = self.required_height();
        let Self {
            titles,
            selected,
            cols,
            highlight_style,
            style,
        } = self;
        for (idx, title) in titles.into_iter().enumerate() {
            let row = idx.rem_euclid(cols as usize);
            let col = idx.div_euclid(rows);
            let tab = if let Some(highlight_style) = highlight_style
                && selected == Some(idx)
            {
                Line::from(title).style(highlight_style)
            } else {
                Line::from(title).style(style)
            }
            .centered();
            let render_area = Rect {
                x: (area.x as usize + col * (longest_title + 1))
                    .try_into()
                    .unwrap_or(u16::MAX),
                y: (area.y as usize + row).try_into().unwrap_or(u16::MAX),
                width: longest_title.try_into().unwrap_or(u16::MAX),
                height: 1,
            }
            // Don't render outside provided area
            .intersection(area);
            tab.render(render_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::TabGrid;
    use pretty_assertions::assert_eq;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;

    #[test]
    fn test_basic_tab_grid() {
        let grid = TabGrid::new_with_cols(["AA", "BBBB", "CCCC", "DD"], 2);
        assert_eq!(grid.required_width(), 9);
        assert_eq!(grid.required_height(), 2);
        let area = Rect::new(0, 0, 9, 2);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        grid.render(area, &mut buf);
        assert_eq!(buf.area, area);
        let rendered_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_cells_as_string = " AA  CCCCBBBB  DD ".to_string();
        assert_eq!(rendered_cells_as_string, expected_cells_as_string);
    }
}
