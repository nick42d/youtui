use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use std::borrow::Cow;
/// Ratatui widgets used in application
pub struct TabGrid<'a> {
    titles: Vec<Cow<'a, str>>,
    selected: Option<usize>,
    constraint: TabGridConstraint,
    highlight_style: Option<Style>,
    style: Style,
}
#[derive(PartialEq)]
enum TabGridConstraint {
    MaxRows(u16),
    MaxCols(u16),
}
impl<'a> TabGrid<'a> {
    pub fn new_with_max_cols(
        titles: impl IntoIterator<Item = impl Into<Cow<'a, str>>>,
        cols: u16,
    ) -> Self {
        TabGrid {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: None,
            constraint: TabGridConstraint::MaxCols(cols),
            highlight_style: Default::default(),
            style: Default::default(),
        }
    }
    pub fn new_with_max_rows(
        titles: impl IntoIterator<Item = impl Into<Cow<'a, str>>>,
        rows: u16,
    ) -> Self {
        TabGrid {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: None,
            constraint: TabGridConstraint::MaxRows(rows),
            highlight_style: Default::default(),
            style: Default::default(),
        }
    }
    /// zero indexed
    pub fn select(self, selected: usize) -> Self {
        Self {
            selected: Some(selected),
            ..self
        }
    }
    #[allow(unused)]
    // This is a library type module and its expected all methods on TabGrid
    // will be eventually used.
    pub fn deselect(self) -> Self {
        Self {
            selected: None,
            ..self
        }
    }
    /// Sets the style for the highlighted tab - overwriting the base style.
    pub fn highlight_style(self, highlight_style: Style) -> Self {
        Self {
            highlight_style: Some(highlight_style),
            ..self
        }
    }
    /// Sets the style for all tabs.
    #[allow(unused)]
    // This is a library type module and its expected all methods on TabGrid
    // will be eventually used.
    pub fn style(self, style: Style) -> Self {
        Self { style, ..self }
    }
    /// Returns 0 if there are 0 cols or 0 titles.
    pub fn required_width(&self) -> usize {
        match self.constraint {
            TabGridConstraint::MaxCols(cols) => self
                .longest_title()
                .saturating_mul(cols as usize)
                .saturating_add(cols as usize)
                .saturating_sub(1),
            TabGridConstraint::MaxRows(rows) => {
                if rows == 0 {
                    return 0;
                }
                let cols = self.titles.len().div_ceil(rows as usize);
                self.longest_title()
                    .saturating_mul(cols)
                    .saturating_add(cols)
                    .saturating_sub(1)
            }
        }
    }
    /// Returns 0 if there are 0 cols (instead of panicing)
    pub fn required_height(&self) -> usize {
        match self.constraint {
            TabGridConstraint::MaxCols(cols) => {
                if cols == 0 {
                    return 0;
                }
                self.titles.len().div_ceil(cols as usize)
            }
            TabGridConstraint::MaxRows(rows) => self.titles.len().min(rows as usize),
        }
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
impl<'a> Widget for TabGrid<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // Do nothing if constraint is 0 cols/rows.
        if self.constraint == TabGridConstraint::MaxCols(0)
            || self.constraint == TabGridConstraint::MaxRows(0)
        {
            return;
        }
        let longest_title = self.longest_title();
        let rows = self.required_height();
        let Self {
            titles,
            selected,
            constraint,
            highlight_style,
            style,
        } = self;
        match constraint {
            TabGridConstraint::MaxCols(max_cols) => {
                for (idx, title) in titles.into_iter().enumerate() {
                    let row = idx.rem_euclid(max_cols as usize);
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
            TabGridConstraint::MaxRows(_) => {
                let cols = titles.len().div_euclid(rows);
                for (idx, title) in titles.into_iter().enumerate() {
                    let row = idx.rem_euclid(cols);
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
        let grid = TabGrid::new_with_max_cols(["AA", "BBBB", "CCCC", "DD"], 2);
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
    #[test]
    fn test_basic_tab_grid_max_cols() {
        let grid = TabGrid::new_with_max_cols(["AA", "BBBB", "CCCC", "DD", "EEEEE", "FF"], 3);
        assert_eq!(grid.required_width(), 17);
        assert_eq!(grid.required_height(), 2);
        let area = Rect::new(0, 0, 17, 2);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        grid.render(area, &mut buf);
        assert_eq!(buf.area, area);
        let rendered_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_cells_as_string = " AA    DD  BBBB EEEEECCCC  FF  ".to_string();
        assert_eq!(rendered_cells_as_string, expected_cells_as_string);
    }
    #[test]
    fn test_basic_tab_grid_max_rows() {
        let grid = TabGrid::new_with_max_rows(["AA", "BBBB", "CCCC", "DD", "EEEEE", "FF"], 3);
        assert_eq!(grid.required_width(), 11);
        assert_eq!(grid.required_height(), 3);
        let area = Rect::new(0, 0, 11, 3);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        grid.render(area, &mut buf);
        assert_eq!(buf.area, area);
        let rendered_cells_as_string = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        let expected_cells_as_string = " AA   CCCC EEEEEBBBB   DD   FF  ".to_string();
        assert_eq!(rendered_cells_as_string, expected_cells_as_string);
    }
}
