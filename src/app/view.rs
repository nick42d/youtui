use super::{structures::Percentage, ui::YoutuiMutableState};
use crate::Result;
use ratatui::{
    prelude::{Constraint, Rect},
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

enum _TableFilter {
    All(_Filter),
    Column { filter: _Filter, column: usize },
}
enum _Filter {
    Contains(String),
    NotContains(String),
    Equal(String),
}

pub enum BasicConstraint {
    Length(u16),
    Percentage(Percentage),
}

pub fn basic_constraints_to_constraints(
    basic_constraints: &[BasicConstraint],
    length: u16,
    margin: u16,
) -> Vec<Constraint> {
    let sum_lengths = basic_constraints
        .iter()
        .fold(0, |acc, c| {
            acc + match c {
                BasicConstraint::Length(l) => *l,
                BasicConstraint::Percentage(_) => 0,
            } + margin
        })
        // One less margin than number of rows.
        .saturating_sub(1);
    basic_constraints
        .iter()
        .map(|bc| match bc {
            BasicConstraint::Length(l) => Constraint::Length(*l),
            BasicConstraint::Percentage(p) => {
                Constraint::Length(p.0 as u16 * (length.saturating_sub(sum_lengths)) / 100)
            }
        })
        .collect()
}

// A struct that is able to be "scrolled". An item will always be selected.
// XXX: Should a Scrollable also be a KeyHandler? This way, can potentially have common keybinds.
pub trait Scrollable {
    // Increment the list by the specified amount.
    fn increment_list(&mut self, amount: isize);
    fn get_selected_item(&self) -> usize;
}

// A simple row in the table.
pub type TableItem<'a> = Box<dyn Iterator<Item = Cow<'a, str>> + 'a>;

// A struct that we are able to draw a table from using the underlying data.
pub trait TableView: Scrollable + Loadable {
    // NOTE: Consider if the Playlist is a NonSortableTable (or Browser a SortableTable), as possible we don't want to sort the Playlist (what happens to play order, for eg).
    // Could have a "commontitle" trait to prevent the need for this in both Table and List
    fn get_title(&self) -> Cow<str>;
    fn get_layout(&self) -> &[BasicConstraint];
    // TODO: Consider if generics <T: Iterator> can be used instead of dyn Iterator.
    fn get_items(&self) -> Box<dyn ExactSizeIterator<Item = TableItem> + '_>;
    // XXX: This doesn't need to be so fancy - could return a static slice.
    fn get_headings(&self) -> Box<dyn Iterator<Item = &'static str>>;
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
pub trait SortableTableView: TableView {
    fn get_sortable_columns(&self) -> &[usize];
    fn get_sort_commands(&self) -> &[TableSortCommand];
    /// This can fail if the TableSortCommand is not within the range of sortable columns.
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()>;
    fn clear_sort_commands(&mut self);
}
// A struct that we are able to draw a list from using the underlying data.
pub trait ListView: Scrollable + SortableList + Loadable {
    type DisplayItem: Display;
    fn get_title(&self) -> Cow<str>;
    fn get_items_display(&self) -> Vec<&Self::DisplayItem>;
    fn len(&self) -> usize {
        self.get_items_display().len()
    }
}
pub trait SortableList {
    fn push_sort_command(&mut self, list_sort_command: String);
    fn clear_sort_commands(&mut self);
}
pub trait FilterableList {
    fn push_filter_command(&mut self, list_filter_command: String);
    fn clear_filter_commands(&mut self);
}
// A drawable part of the application.
pub trait Drawable {
    // Helper function to draw.
    fn draw_chunk(&self, f: &mut Frame, chunk: Rect);
    fn draw(&self, f: &mut Frame) {
        self.draw_chunk(f, f.size());
    }
}
// A drawable part of the application that mutates its state on draw.
pub trait DrawableMut {
    // Helper function to draw.
    // TODO: Clean up function signature regarding mutable state.
    fn draw_mut_chunk(&self, f: &mut Frame, chunk: Rect, mutable_state: &mut YoutuiMutableState);
    fn draw_mut(&self, f: &mut Frame, mutable_state: &mut YoutuiMutableState) {
        self.draw_mut_chunk(f, f.size(), mutable_state);
    }
}
// A selectable part of the application.
pub trait Selectable: Drawable {
    fn draw_selectable_chunk(&self, f: &mut Frame, chunk: Rect, selected: bool);

    fn draw_selectable(&self, f: &mut Frame, selected: bool) {
        self.draw_selectable_chunk(f, f.size(), selected);
    }
}
// A part of the application that can be in a Loading state.
pub trait Loadable {
    fn is_loading(&self) -> bool;
}
