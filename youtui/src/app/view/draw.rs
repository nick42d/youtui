use super::{
    AdvancedTableView, TableSortCommand, TableView, basic_constraints_to_table_constraints,
};
use crate::app::ui::browser::shared_components::{FilterManager, SortManager};
use crate::app::ui::draw::draw_text_box;
use crate::app::view::{BasicConstraint, ListView, Loadable};
use crate::drawutils::{
    DESELECTED_BORDER_COLOUR, ROW_HIGHLIGHT_COLOUR, SELECTED_BORDER_COLOUR, TABLE_HEADINGS_COLOUR,
    TEXT_COLOUR,
};
use itertools::Either;
use rat_text::HasScreenCursor;
use rat_text::text_input::{TextInput, TextInputState};
use ratatui::Frame;
use ratatui::prelude::{Margin, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::{block, line};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, StatefulWidget, Table, TableState, Widget,
};
use std::borrow::Cow;

// Popups look aesthetically weird when really small, so setting a minimum.
pub const MIN_POPUP_WIDTH: usize = 20;

/// Helper function that calls get_stateful_widget but consumes the state and
/// returns the modified version instead of mutating in place
pub fn move_render_stateful_widget<W: StatefulWidget>(
    f: &mut Frame,
    widget: W,
    area: Rect,
    state: W::State,
) -> W::State {
    let mut state = state;
    f.render_stateful_widget(widget, area, &mut state);
    state
}

pub fn get_table_sort_character_array(
    sort_commands: &[TableSortCommand],
) -> Vec<Option<&'static str>> {
    let Some(max_col) = sort_commands
        .iter()
        .max_by_key(|c| c.column)
        .map(|cmd| cmd.column)
    else {
        return Vec::new();
    };
    let mut temp_vec = Vec::new();
    temp_vec.resize(max_col + 1, None);
    sort_commands.iter().fold(temp_vec, |mut acc, e| {
        // We created the Vec to accomodate max col above so this is safe.
        acc[e.column] = match e.direction {
            super::SortDirection::Asc => Some(""),
            super::SortDirection::Desc => Some(""),
        };
        acc
    })
}

// Draw a block, and return the inner rectangle.
pub fn draw_panel<S: AsRef<str>>(
    f: &mut Frame,
    title: S,
    // NOTE: Type is tied to title (same type S - weird quirk!)
    footer: Option<S>,
    chunk: Rect,
    is_selected: bool,
) -> Rect {
    let border_colour = if is_selected {
        SELECTED_BORDER_COLOUR
    } else {
        DESELECTED_BORDER_COLOUR
    };
    if let Some(s) = footer {
        let block = Block::new()
            .title(title.as_ref())
            .title_bottom(s.as_ref())
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_colour));
        let inner_chunk = block.inner(chunk);
        f.render_widget(block, chunk);
        inner_chunk
    } else {
        let block = Block::new()
            .title(title.as_ref())
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_colour));
        let inner_chunk = block.inner(chunk);
        f.render_widget(block, chunk);
        inner_chunk
    }
}

pub fn draw_list(f: &mut Frame, list: &mut impl ListView, chunk: Rect, selected: bool) {
    let selected_item = list.get_selected_item();
    list.get_mut_state().select(Some(selected_item));
    // TODO: Scroll bars
    let list_title = list.get_title();
    let list_widget =
        List::new(list.get_items()).highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR));
    let inner_chunk = draw_panel(f, list_title, None, chunk, selected);
    // ListState is cheap to clone
    *list.get_mut_state() =
        move_render_stateful_widget(f, list_widget, inner_chunk, list.get_state().clone());
}

pub fn draw_table<T>(f: &mut Frame, table: &mut T, chunk: Rect, selected: bool)
where
    T: TableView,
{
    let items = table.get_items();
    let len = items.len();
    *table.get_mut_state() = draw_table_impl(
        f,
        chunk,
        selected,
        table.get_selected_item(),
        table.get_highlighted_row(),
        table.get_state(),
        items,
        len,
        table.get_layout(),
        table.get_headings(),
        table.get_title(),
    );
}

pub fn draw_table_impl<'a>(
    f: &mut Frame,
    chunk: Rect,
    selected: bool,
    cur: usize,
    highlighted: Option<usize>,
    state: &TableState,
    items: impl Iterator<Item = impl Iterator<Item = Cow<'a, str>> + 'a> + 'a,
    len: usize,
    layout: &'a [BasicConstraint],
    headings: impl Iterator<Item = &'static str>,
    title: Cow<'a, str>,
) -> TableState {
    // TableState is cheap to clone
    // Set the state to the currently selected item.
    let mut new_state = state.clone();
    new_state.select(Some(cur));
    let table_items = items.enumerate().map(|(idx, items)| {
        if Some(idx) == highlighted {
            Row::new(items).bold().italic()
        } else {
            Row::new(items)
        }
        .style(Style::new().fg(TEXT_COLOUR))
    });
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let table_widths =

     // Minus block
    basic_constraints_to_table_constraints(layout, chunk.width.saturating_sub(2), 1);
    let table_widget = Table::new(table_items, table_widths)
        .row_highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .header(
            Row::new(headings).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(TABLE_HEADINGS_COLOUR),
            ),
        )
        .column_spacing(1);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(Some(line::VERTICAL))
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = len.saturating_sub(table_height);
    let inner_chunk = draw_panel(f, title, None, chunk, selected);
    let pos = state.offset().min(scrollable_lines);
    let new_state = move_render_stateful_widget(f, table_widget, inner_chunk, new_state);
    // Call this after rendering table, as offset is mutated.
    let mut scrollbar_state = ScrollbarState::default()
        .position(pos)
        .content_length(scrollable_lines);
    f.render_stateful_widget(
        scrollbar,
        chunk.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
    new_state
}

pub fn draw_sortable_table(
    f: &mut Frame,
    table: &mut impl AdvancedTableView,
    chunk: Rect,
    selected: bool,
) {
    // Set the state to the currently selected item.
    let selected_item = table.get_selected_item();
    table.get_mut_state().select(Some(selected_item));
    // TODO: theming
    let table_items = table.get_filtered_items().map(Row::new);
    // Likely expensive, and could be optimised.
    let number_items = table.get_filtered_items().count();
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let table_widths = basic_constraints_to_table_constraints(
        table.get_layout(),
        chunk.width.saturating_sub(2),
        1,
    ); // Minus block
    let heading_names = table.get_headings();
    let sort_headings = get_table_sort_character_array(table.get_sort_commands())
        .into_iter()
        .chain(std::iter::repeat(None));
    let sortable_headings = table.get_sortable_columns();
    // TODO: Improve how we do this - may not need to use the enumerate/contains
    let combined_headings =
        heading_names
            .zip(sort_headings)
            .enumerate()
            .map(|(idx, (heading, sort_char))| {
                if let Some(sort_char) = sort_char {
                    Cell::from(Line::from_iter([heading, sort_char]))
                } else if sortable_headings.contains(&idx) {
                    Cell::from(Line::from_iter([heading, ""]))
                } else {
                    Cell::from(heading)
                }
            });
    let filter_str: String = itertools::intersperse(
        table.get_filter_commands().iter().map(|f| f.as_readable()),
        ", ".to_string(),
    )
    .collect();
    // Naive implementation
    let filter_str = if filter_str.len() > 1 {
        ": ".to_string() + &filter_str
    } else {
        filter_str
    };
    let table_widget = Table::new(table_items, table_widths)
        .row_highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .header(
            Row::new(combined_headings).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(TABLE_HEADINGS_COLOUR),
            ),
        )
        .column_spacing(1);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(Some(line::VERTICAL))
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = number_items.saturating_sub(table_height);
    let inner_chunk = draw_panel(
        f,
        table.get_title(),
        Some(filter_str.into()),
        chunk,
        selected,
    );
    // Clone of TableState is cheap
    let mut new_table_state = table.get_state().clone();
    f.render_stateful_widget(table_widget, inner_chunk, &mut new_table_state);
    *table.get_mut_state() = new_table_state;
    // Call this after rendering table, as offset is mutated.
    let mut scrollbar_state = ScrollbarState::default()
        .position(table.get_state().offset().min(scrollable_lines))
        .content_length(scrollable_lines);
    f.render_stateful_widget(
        scrollbar,
        chunk.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    if table.sort_popup_shown() {
        draw_sort_popup(f, table, chunk);
    }

    if table.filter_popup_shown() {
        draw_filter_popup(f, table, chunk);
    }
}

pub fn draw_loadable_mut<T: Loadable>(
    f: &mut Frame,
    t: &mut T,
    chunk: Rect,
    draw_call: impl FnOnce(&mut T, &mut Frame, Rect),
) {
    if t.is_loading() {
        let loading = Paragraph::new("Loading");
        return f.render_widget(loading, chunk);
    };
    draw_call(t, f, chunk);
}

/// Returns a new ListState for the sort popup.
fn draw_sort_popup(f: &mut Frame, table: &mut impl AdvancedTableView, chunk: Rect) {
    let title = "Sort";
    let sortable_columns = table.get_sortable_columns();
    let headers: Vec<_> = table
        .get_headings()
        .enumerate()
        .filter_map(|(i, h)| {
            if sortable_columns.contains(&i) {
                // TODO: Remove allocation
                Some(ListItem::new(h))
            } else {
                None
            }
        })
        // TODO: Remove allocation
        .collect();
    let max_header_len = headers.iter().fold(0, |acc, e| acc.max(e.width()));
    // List looks a bit nicer with a minimum width, so passing a hardcoded minimum
    // here.
    let width = max_header_len.max(title.len()).max(MIN_POPUP_WIDTH) + 2;
    let height = sortable_columns.len() + 2;
    let popup_chunk = crate::drawutils::centered_rect(height as u16, width as u16, chunk);
    // Clone of ListState is cheap
    let mut new_state = table
        .get_sort_state()
        .clone()
        .with_selected(Some(table.get_sort_popup_cur()));
    let list = List::new(headers)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .block(
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SELECTED_BORDER_COLOUR)),
        );
    f.render_widget(Clear, popup_chunk);
    f.render_stateful_widget(list, popup_chunk, &mut new_state);
    *table.get_mut_sort_state() = new_state;
}

fn draw_filter_popup(f: &mut Frame, table: &mut impl AdvancedTableView, chunk: Rect) {
    let title = "Filter";
    // Hardocde dimensions of filter input.
    let popup_chunk = crate::drawutils::centered_rect(3, 22, chunk);
    f.render_widget(Clear, popup_chunk);
    let mut text_state = table
        .get_filter_state()
        .try_borrow_mut()
        .expect("This only place filter text_state is mutably borrowed");
    draw_text_box(f, title, &mut *text_state, popup_chunk);
}
