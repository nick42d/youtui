use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Widget};
use std::borrow::Cow;

/// Ratatui widgets used in application
pub struct TabGrid<'a, const N: usize> {
    titles: [Cow<'a, str>; N],
    selected: Option<usize>,
    cols: u16,
    selected_style: Style,
    deselected_style: Style,
}
impl<'a, const N: usize> TabGrid<'a, N> {
    pub fn new_with_cols(titles: [impl Into<Cow<'a, str>>; N], cols: u16) -> Self {
        Self {
            titles: titles.map(|title| title.into()),
            selected: None,
            cols,
            selected_style: Default::default(),
            deselected_style: Default::default(),
        }
    }
    /// zero indexed
    pub fn select(self, selected: usize) -> Self {
        let Self {
            titles,
            cols,
            selected_style,
            deselected_style,
            ..
        } = self;
        Self {
            titles,
            selected: Some(selected),
            cols,
            selected_style,
            deselected_style,
        }
    }
    pub fn deselect(self) -> Self {
        let Self {
            titles,
            cols,
            selected_style,
            deselected_style,
            ..
        } = self;
        Self {
            titles,
            selected: None,
            cols,
            selected_style,
            deselected_style,
        }
    }
    pub fn with_selected_style(self, selected_style: Style) -> Self {
        let Self {
            titles,
            cols,
            selected,
            deselected_style,
            ..
        } = self;
        Self {
            titles,
            selected,
            cols,
            selected_style,
            deselected_style,
        }
    }
    pub fn with_deselected_style(self, deselected_style: Style) -> Self {
        let Self {
            titles,
            cols,
            selected,
            selected_style,
            ..
        } = self;
        Self {
            titles,
            selected,
            cols,
            selected_style,
            deselected_style,
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
            selected_style,
            deselected_style,
        } = self;
        for (idx, title) in titles.into_iter().enumerate() {
            let row = idx.rem_euclid(cols as usize);
            let col = idx.div_euclid(rows);
            let tab = if selected == Some(idx) {
                Line::from(title).style(selected_style)
            } else {
                Line::from(title).style(deselected_style)
            }
            .centered();
            let render_area = Rect {
                x: (area.x as usize + col * (longest_title + 1))
                    .try_into()
                    .unwrap(),
                y: (area.y as usize + row).try_into().unwrap(),
                width: longest_title.try_into().unwrap(),
                height: 1,
            };
            tab.render(render_area, buf);
        }
    }
}
