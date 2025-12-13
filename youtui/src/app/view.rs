/// Traits related to viewable application components.
use super::structures::{ListSong, ListSongDisplayableField, Percentage};
use rat_text::text_input::TextInputState;
use ratatui::Frame;
use ratatui::prelude::{Constraint, Rect};
use ratatui::widgets::{ListState, TableState};
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Display;

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
            FilterString::CaseSensitive(s) => format!("a=a:{s}"),
            FilterString::CaseInsensitive(s) => format!("a=A:{s}"),
        }
    }
    pub fn is_in<S: AsRef<str>>(&self, test_str: S) -> bool {
        match self {
            FilterString::CaseSensitive(s) => test_str.as_ref().contains(s),
            // Ascii lowercase may not be correct but it avoids frequent allocations.
            FilterString::CaseInsensitive(s) => test_str
                .as_ref()
                .to_ascii_lowercase()
                .contains(s.to_ascii_lowercase().as_str()),
        }
    }
    pub fn is_equal<S: AsRef<str>>(&self, test_str: S) -> bool {
        match self {
            FilterString::CaseSensitive(s) => test_str.as_ref() == s,
            // Ascii lowercase may not be correct but it avoids frequent allocations.
            FilterString::CaseInsensitive(s) => {
                test_str.as_ref().to_ascii_lowercase() == s.to_ascii_uppercase()
            }
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
pub trait TableView {
    fn get_state(&self) -> &TableState;
    fn get_mut_state(&mut self) -> &mut TableState;
    /// An item will always be selected.
    fn get_selected_item(&self) -> usize;
    fn get_layout(&self) -> &[BasicConstraint];
    // A row can be highlighted.
    fn get_highlighted_row(&self) -> Option<usize>;
    fn get_items(&self) -> impl ExactSizeIterator<Item = impl Iterator<Item = Cow<'_, str>> + '_>;
    fn get_headings(&self) -> impl Iterator<Item = &'static str>;
    // Not a particularly useful function for a sortabletableview
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
/// TableView with built in filtering and sorting.
pub trait AdvancedTableView: TableView {
    fn get_filter_state(&self) -> &TextInputState;
    fn get_mut_filter_state(&mut self) -> &mut TextInputState;
    fn filter_popup_shown(&self) -> bool;
    fn get_filterable_columns(&self) -> &[usize];
    // This can't be ExactSized as return type may be Filter<T>
    fn get_filtered_items(&self) -> impl Iterator<Item = impl Iterator<Item = Cow<'_, str>> + '_>;
    fn get_filter_commands(&self) -> &[TableFilterCommand];
    fn push_filter_command(&mut self, filter_command: TableFilterCommand);
    fn clear_filter_commands(&mut self);
    // SortableTableView should maintain it's own popup state.
    fn get_sort_popup_cur(&self) -> usize;
    fn sort_popup_shown(&self) -> bool;
    fn get_sort_state(&self) -> &ListState;
    fn get_mut_sort_state(&mut self) -> &mut ListState;
    /// Add a new TableSortCommand and sort the table.
    /// This can fail if the TableSortCommand is not within the range of
    /// sortable columns.
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> anyhow::Result<()>;
    fn clear_sort_commands(&mut self);
    fn get_sortable_columns(&self) -> &[usize];
    fn get_sort_commands(&self) -> &[TableSortCommand];
}
// A struct that we are able to draw a list from using the underlying data.
pub trait ListView {
    /// An item will always be selected.
    fn get_selected_item(&self) -> usize;
    fn get_state(&self) -> &ListState;
    fn get_mut_state(&mut self) -> &mut ListState;
    fn get_items(&self) -> impl ExactSizeIterator<Item = Cow<'_, str>> + '_;
    fn len(&self) -> usize {
        self.get_items().len()
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
// A part of the application that has a title
pub trait HasTitle {
    fn get_title(&self) -> Cow<'_, str>;
}

#[cfg(test)]
mod tests {
    use super::{BasicConstraint, basic_constraints_to_table_constraints};
    use crate::app::structures::Percentage;
    use ratatui::prelude::Constraint;

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
