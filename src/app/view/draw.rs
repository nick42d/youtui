use super::{basic_constraints_to_constraints, SortableTableView, TableSortCommand, TableView};
use crate::{
    app::view::ListView,
    drawutils::{
        DESELECTED_BORDER_COLOUR, ROW_HIGHLIGHT_COLOUR, SELECTED_BORDER_COLOUR,
        TABLE_HEADINGS_COLOUR,
    },
};
use ratatui::{
    prelude::{Margin, Rect},
    style::{Modifier, Style},
    symbols::{block, line},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame,
};
use std::borrow::Cow;

pub fn get_table_sort_character_array(sort_commands: &[TableSortCommand]) -> Vec<Option<char>> {
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
            super::SortDirection::Asc => Some(''),
            super::SortDirection::Desc => Some(''),
        };
        acc
    })
}

// Draw a block, and return the inner rectangle.
// XXX: title could be Into<Cow<str>>
pub fn draw_panel(f: &mut Frame, title: Cow<str>, chunk: Rect, is_selected: bool) -> Rect {
    let border_colour = if is_selected {
        SELECTED_BORDER_COLOUR
    } else {
        DESELECTED_BORDER_COLOUR
    };
    let block = Block::new()
        // TODO: Remove allocation
        .title(title.as_ref())
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_colour));

    let inner_chunk = block.inner(chunk);
    f.render_widget(block, chunk);
    inner_chunk
}

pub fn draw_list<L>(f: &mut Frame, list: &L, chunk: Rect, selected: bool, state: &mut ListState)
where
    L: ListView,
{
    // Set the state to the currently selected item.
    state.select(Some(list.get_selected_item()));
    // TODO: Scroll bars
    let list_title = list.get_title();
    let list_len = list.len();
    let list_items: Vec<_> = list
        .get_items_display()
        .iter()
        // We are allocating here, as list item only implements Display (not Into<Cow>). Consider changing this.
        .map(|item| ListItem::new(item.to_string()))
        // We are allocating here, as List::new won't take an iterator. May change in future.
        .collect();
    // TODO: Better title for list
    let _title = format!("{list_title} - {list_len} items");
    let list_widget =
        List::new(list_items).highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR));
    let inner_chunk = draw_panel(f, list_title, chunk, selected);
    f.render_stateful_widget(list_widget, inner_chunk, state);
}

pub fn draw_table<T>(f: &mut Frame, table: &T, chunk: Rect, state: &mut TableState, selected: bool)
where
    T: TableView,
{
    // Set the state to the currently selected item.
    state.select(Some(table.get_selected_item()));
    // TODO: theming
    let table_items = table.get_items().map(|item| Row::new(item));
    let number_items = table.len();
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let table_widths =
        basic_constraints_to_constraints(table.get_layout(), chunk.width.saturating_sub(2), 1); // Minus block
    let heading_names = table.get_headings();
    let table_widget = Table::new(table_items)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .header(
            Row::new(heading_names).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(TABLE_HEADINGS_COLOUR),
            ),
        )
        .widths(table_widths.as_slice())
        .column_spacing(1);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(Some(line::VERTICAL))
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = number_items.saturating_sub(table_height);
    let inner_chunk = draw_panel(f, table.get_title(), chunk, selected);
    if table.is_loading() {
        draw_loading(f, inner_chunk)
    } else {
        f.render_stateful_widget(table_widget, inner_chunk, state);
        // Call this after rendering table, as offset is mutated.
        let mut scrollbar_state = ScrollbarState::default()
            .position(state.offset().min(scrollable_lines))
            .content_length(scrollable_lines);
        f.render_stateful_widget(
            scrollbar,
            chunk.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        )
    }
}

pub fn draw_sortable_table<T>(
    f: &mut Frame,
    table: &T,
    chunk: Rect,
    state: &mut TableState,
    selected: bool,
) where
    T: SortableTableView,
{
    // Set the state to the currently selected item.
    state.select(Some(table.get_selected_item()));
    // TODO: theming
    let table_items = table.get_items().map(|item| Row::new(item));
    let number_items = table.len();
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let table_widths =
        basic_constraints_to_constraints(table.get_layout(), chunk.width.saturating_sub(2), 1); // Minus block
    let heading_names = table.get_headings();
    let mut sort_headings = get_table_sort_character_array(table.get_sort_commands()).into_iter();
    let sortable_headings = table.get_sortable_columns();
    let combined_headings = heading_names.enumerate().map(|(i, h)| {
        let mut hstr = h.to_string();
        let sort_char = sort_headings.next().unwrap_or_default().unwrap_or_default();
        if sort_char == '\x00' && sortable_headings.contains(&i) {
            hstr.push('');
        }
        hstr.push(sort_char);
        hstr
    });
    let table_widget = Table::new(table_items)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .header(
            Row::new(combined_headings).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(TABLE_HEADINGS_COLOUR),
            ),
        )
        .widths(table_widths.as_slice())
        .column_spacing(1);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(Some(line::VERTICAL))
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = number_items.saturating_sub(table_height);
    let inner_chunk = draw_panel(f, table.get_title(), chunk, selected);
    if table.is_loading() {
        draw_loading(f, inner_chunk)
    } else {
        f.render_stateful_widget(table_widget, inner_chunk, state);
        // Call this after rendering table, as offset is mutated.
        let mut scrollbar_state = ScrollbarState::default()
            .position(state.offset().min(scrollable_lines))
            .content_length(scrollable_lines);
        f.render_stateful_widget(
            scrollbar,
            chunk.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        )
    }
}

pub fn draw_loading(f: &mut Frame, chunk: Rect) {
    let loading = Paragraph::new("Loading");
    f.render_widget(loading, chunk);
}
