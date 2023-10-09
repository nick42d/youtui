pub mod draw;
use std::{borrow::Cow, fmt::Display};

use ratatui::{
    prelude::{Backend, Constraint, Rect},
    Frame,
};

struct _TableSort {
    column: usize,
    direction: _SortDirection,
}
enum _SortDirection {
    Asc,
    Desc,
}
enum _TableFilter {
    All(Filter),
    Column { filter: Filter, column: usize },
}
enum Filter {
    Contains(String),
}
// A list of items. An item will always be selected.
// XXX: Should a Scrollable also be a KeyHandler? This way, can potentially have common keybinds.
pub trait Scrollable {
    // Get the current position in the list.
    fn get_selected_item(&self) -> usize;
    // Increment the list by the specified amount.
    fn increment_list(&mut self, amount: isize);
}
// A row in the table with addressable fields.
pub trait TableItem {
    fn get_field(&self, index: usize) -> Option<Cow<'_, str>>;
    // Number of fields
    fn len(&self) -> usize;
}
// A struct that we are able to draw a table from using the underlying data.
pub trait TableView: Scrollable + Loadable {
    type Item: TableItem;
    // Could have a "commontitle" trait to prevent the need for this in both Table and List
    fn get_title(&self) -> Cow<str>;
    fn get_layout(&self) -> Vec<Constraint>;
    fn get_items(&self) -> Vec<&Self::Item>;
    fn get_headings(&self) -> Vec<&'static str>;
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
pub trait List {
    type Item;
    fn get_title(&self) -> Cow<str>;
    fn get_items(&self) -> Vec<&Self::Item>;
    fn len(&self) -> usize {
        self.get_items().len()
    }
}
// A struct that we are able to draw a list from using the underlying data.
pub trait ListView: Scrollable + SortableList + List + Loadable {
    type DisplayItem: Display;
    fn get_items_display(&self) -> Vec<&Self::DisplayItem>;
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
    fn draw_chunk<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect);
    fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        self.draw_chunk(f, f.size());
    }
}
// A selectable part of the application.
pub trait Selectable: Drawable {
    fn draw_selectable_chunk<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect, selected: bool);

    fn draw_selectable<B: Backend>(&self, f: &mut Frame<B>, selected: bool) {
        self.draw_selectable_chunk(f, f.size(), selected);
    }
}
// A part of the application that can be in a Loading state.
pub trait Loadable {
    fn is_loading(&self) -> bool;
}
