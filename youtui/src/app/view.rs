/// Traits related to viewable application components.
use super::structures::{ListSong, ListSongDisplayableField, Percentage};
use ratatui::{
    prelude::{Constraint, Rect},
    widgets::{ListState, TableState},
    Frame,
};
use std::{borrow::Cow, fmt::Display};

pub mod draw;

#[derive(Clone, Debug)]
pub struct TableSortCommand {
    pub column: usize,
    pub direction: SortDirection,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub enum TableFilterCommand {
    All(Filter),
    Column { filter: Filter, column: usize },
}
#[derive(Clone, Debug)]
pub enum Filter {
    Contains(FilterString),
    NotContains(FilterString),
    Equal(FilterString),
}
#[derive(Clone, Debug)]
pub enum FilterString {
    CaseSensitive(String),
    CaseInsensitive(String),
}

impl TableFilterCommand {
    fn as_readable(&self) -> String {
        match self {
            TableFilterCommand::All(f) => format!("ALL{}", f.as_readable()),
            TableFilterCommand::Column { filter, column } => {
                format!("COL{}{}", column, filter.as_readable())
            }
        }
    }
    #[deprecated = "Temporary function to be replaced with as_readable"]
    fn as_basic_readable(&self) -> String {
        match self {
            TableFilterCommand::All(f) => match f {
                Filter::Contains(f) => match f {
                    FilterString::CaseSensitive(_) => todo!(),
                    FilterString::CaseInsensitive(s) => format!("[a-Z]*{}*", s),
                },
                Filter::NotContains(_) => todo!(),
                Filter::Equal(_) => todo!(),
            },
            TableFilterCommand::Column { .. } => todo!(),
        }
    }
    pub fn matches_row<const N: usize>(
        &self,
        row: &ListSong,
        fields_in_table: [ListSongDisplayableField; N],
        filterable_colums: &[usize],
    ) -> bool {
        let fields = row.get_fields(fields_in_table);
        match self {
            TableFilterCommand::All(filter) => match filter {
                Filter::Contains(filter_string) => filterable_colums
                    .iter()
                    .any(|col| filter_string.is_in(fields[*col].as_ref())),
                Filter::NotContains(filter_string) => filterable_colums
                    .iter()
                    .all(|col| !filter_string.is_in(fields[*col].as_ref())),
                Filter::Equal(filter_string) => filterable_colums
                    .iter()
                    .any(|col| filter_string.is_equal(fields[*col].as_ref())),
            },
            TableFilterCommand::Column { filter, column } => match filter {
                Filter::Contains(filter_string) => filter_string.is_in(fields[*column].as_ref()),
                Filter::NotContains(filter_string) => {
                    !filter_string.is_in(fields[*column].as_ref())
                }
                Filter::Equal(filter_string) => filter_string.is_equal(fields[*column].as_ref()),
            },
        }
    }
}
impl Filter {
    fn as_readable(&self) -> String {
        match self {
            Filter::Contains(f) => format!("~{}", f.as_readable()),
            Filter::NotContains(f) => format!("!={}", f.as_readable()),
            Filter::Equal(f) => format!("={}", f.as_readable()),
        }
    }
}
impl FilterString {
    fn as_readable(&self) -> String {
        match self {
            FilterString::CaseSensitive(s) => format!("A:{s}"),
            FilterString::CaseInsensitive(s) => format!("a:{s}"),
        }
    }
    pub fn is_in<S: AsRef<str>>(&self, test_str: S) -> bool {
        match self {
            FilterString::CaseSensitive(s) => test_str.as_ref().contains(s),
            // XXX: Ascii lowercase may not be correct.
            FilterString::CaseInsensitive(s) => test_str
                .as_ref()
                .to_ascii_lowercase()
                .contains(s.to_ascii_lowercase().as_str()),
        }
    }
    pub fn is_equal<S: AsRef<str>>(&self, test_str: S) -> bool {
        match self {
            FilterString::CaseSensitive(s) => todo!(),
            FilterString::CaseInsensitive(s) => todo!(),
        }
    }
}

/// Basic wrapper around constraint to allow mixing of percentage and length.
pub enum BasicConstraint {
    Length(u16),
    Percentage(Percentage),
}

// TODO: Add more tests
/// Use basic constraints to construct dynamic column widths for a table.
pub fn basic_constraints_to_table_constraints(
    basic_constraints: &[BasicConstraint],
    length: u16,
    margin: u16,
) -> Vec<Constraint> {
    let sum_lengths = basic_constraints.iter().fold(0, |acc, c| {
        acc + match c {
            BasicConstraint::Length(l) => *l,
            BasicConstraint::Percentage(_) => 0,
        } + margin
    });
    basic_constraints
        .iter()
        .map(|bc| match bc {
            BasicConstraint::Length(l) => Constraint::Length(*l),
            BasicConstraint::Percentage(p) => {
                Constraint::Length(p.0 as u16 * length.saturating_sub(sum_lengths) / 100)
            }
        })
        .collect()
}

/// A struct that we are able to draw a table from using the underlying data.
pub trait TableView: Loadable {
    /// An item will always be selected.
    fn get_selected_item(&self) -> usize;
    /// Get an owned version of the widget state, e.g scroll offset position.
    /// In practice this will clone, and this is acceptable due to the low cost.
    fn get_state(&self) -> TableState;
    // NOTE: Consider if the Playlist is a NonSortableTable (or Browser a
    // SortableTable), as possible we don't want to sort the Playlist (what happens
    // to play order, for eg). Could have a "commontitle" trait to prevent the
    // need for this in both Table and List
    fn get_title(&self) -> Cow<str>;
    fn get_layout(&self) -> &[BasicConstraint];
    // A row can be highlighted.
    fn get_highlighted_row(&self) -> Option<usize>;
    // TODO: Consider if generics <T: Iterator> can be used instead of dyn Iterator.
    fn get_items(
        &self,
    ) -> Box<dyn ExactSizeIterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> + '_>;
    // XXX: This doesn't need to be so fancy - could return a static slice.
    fn get_headings(&self) -> Box<dyn Iterator<Item = &'static str>>;
    // Not a particularly useful function for a sortabletableview
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
pub trait SortableTableView: TableView {
    fn get_sortable_columns(&self) -> &[usize];
    fn get_sort_commands(&self) -> &[TableSortCommand];
    /// This can fail if the TableSortCommand is not within the range of
    /// sortable columns.
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> anyhow::Result<()>;
    fn clear_sort_commands(&mut self);
    // Assuming a SortableTable is also Filterable.
    fn get_filterable_columns(&self) -> &[usize];
    // This can't be ExactSized as return type may be Filter<T>
    fn get_filtered_items(
        &self,
    ) -> Box<dyn Iterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> + '_>;
    fn get_filter_commands(&self) -> &[TableFilterCommand];
    fn push_filter_command(&mut self, filter_command: TableFilterCommand);
    fn clear_filter_commands(&mut self);
    // SortableTableView should maintain it's own popup state.
    fn get_sort_popup_cur(&self) -> usize;
    // SortableTableView should maintain it's own popup state.
    fn get_sort_popup_state(&self) -> ListState;
}
// A struct that we are able to draw a list from using the underlying data.
pub trait ListView: Loadable {
    type DisplayItem: Display;
    /// An item will always be selected.
    fn get_selected_item(&self) -> usize;
    /// Get an owned version of the widget state, e.g scroll offset position.
    /// In practice this will clone, and this is acceptable due to the low cost.
    fn get_state(&self) -> ListState;
    fn get_title(&self) -> Cow<str>;
    fn get_items_display(&self) -> Vec<&Self::DisplayItem>;
    fn len(&self) -> usize {
        self.get_items_display().len()
    }
}
// A drawable part of the application.
pub trait Drawable {
    // Helper function to draw.
    fn draw_chunk(&self, f: &mut Frame, chunk: Rect, selected: bool);
    fn draw(&self, f: &mut Frame, selected: bool) {
        self.draw_chunk(f, f.area(), selected);
    }
}
// A drawable part of the application that mutates its state on draw.
pub trait DrawableMut {
    // Helper function to draw.
    // TODO: Clean up function signature regarding mutable state.
    fn draw_mut_chunk(&mut self, f: &mut Frame, chunk: Rect, selected: bool);
    fn draw_mut(&mut self, f: &mut Frame, selected: bool) {
        self.draw_mut_chunk(f, f.area(), selected)
    }
}
// A part of the application that can be in a Loading state.
pub trait Loadable {
    fn is_loading(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use ratatui::prelude::Constraint;

    use super::{basic_constraints_to_table_constraints, BasicConstraint};
    use crate::app::structures::Percentage;

    #[test]
    fn test_constraints() {
        let basic_constraints = &[
            BasicConstraint::Length(5),
            BasicConstraint::Length(5),
            BasicConstraint::Percentage(Percentage(100)),
        ];
        let constraints = vec![
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(10),
        ];
        let converted = basic_constraints_to_table_constraints(basic_constraints, 20, 0);
        assert_eq!(converted, constraints);
        let basic_constraints = &[
            BasicConstraint::Length(5),
            BasicConstraint::Length(5),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Percentage(Percentage(50)),
        ];
        let constraints = vec![
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
        ];
        let converted = basic_constraints_to_table_constraints(basic_constraints, 20, 0);
        assert_eq!(converted, constraints);
    }
}
