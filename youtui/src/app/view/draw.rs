use super::{
    basic_constraints_to_table_constraints, SortableTableView, TableSortCommand, TableView,
};
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
    text::Line,
    widgets::{
        block::{Position, Title},
        Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    Frame,
};

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
            .title(Title::from(s.as_ref()).position(Position::Bottom))
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
        // We are allocating here, as list item only implements Display (not Into<Cow>). Consider
        // changing this.
        .map(|item| ListItem::new(item.to_string()))
        // We are allocating here, as List::new won't take an iterator. May change in future.
        .collect();
    // TODO: Better title for list
    let _title = format!("{list_title} - {list_len} items");
    let list_widget =
        List::new(list_items).highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR));
    let inner_chunk = draw_panel(f, list_title, None, chunk, selected);
    f.render_stateful_widget(list_widget, inner_chunk, state);
}

pub fn draw_table<T>(f: &mut Frame, table: &T, chunk: Rect, state: &mut TableState, selected: bool)
where
    T: TableView,
{
    // Set the state to the currently selected item.
    state.select(Some(table.get_selected_item()));
    // TODO: theming
    let table_items = table.get_items().map(Row::new);
    let number_items = table.len();
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let table_widths = basic_constraints_to_table_constraints(
        table.get_layout(),
        chunk.width.saturating_sub(2),
        1,
    ); // Minus block
    let heading_names = table.get_headings();
    let table_widget = Table::new(table_items, table_widths)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .header(
            Row::new(heading_names).style(
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
    let inner_chunk = draw_panel(f, table.get_title(), None, chunk, selected);
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
            chunk.inner(Margin {
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
        table
            .get_filter_commands()
            .iter()
            .map(|f| f.as_basic_readable()),
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
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
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
            chunk.inner(Margin {
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
