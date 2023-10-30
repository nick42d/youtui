use std::borrow::Cow;

use crate::app::ui::view::ListView;
use ratatui::{
    prelude::{Backend, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols::{block, line},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame,
};
use tracing::info;

use super::{basic_constraints_to_constraints, TableItem, TableView};

const SELECTED_BORDER: Color = Color::Cyan;
const DESELECTED_BORDER: Color = Color::White;

// Draw a block, and return the inner rectangle.
// XXX: title could be Into<Cow<str>>
pub fn draw_panel<B>(f: &mut Frame<B>, title: Cow<str>, chunk: Rect, is_selected: bool) -> Rect
where
    B: Backend,
{
    let border_colour = if is_selected {
        SELECTED_BORDER
    } else {
        DESELECTED_BORDER
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

pub fn draw_list<B, L>(f: &mut Frame<B>, list: &L, chunk: Rect, selected: bool)
where
    B: Backend,
    L: ListView,
{
    let list_title = list.get_title();
    let list_len = list.len();
    // We are allocating here, as list item only implements Display (not Into<Cow>). Consider changing this.
    let list_items: Vec<_> = list
        .get_items_display()
        .iter()
        .map(|item| ListItem::new(item.to_string()))
        .collect();
    // TODO: Better title for list
    let _title = format!("{list_title} - {list_len} items");
    let mut state = ListState::default().with_selected(Some(list.get_selected_item()));
    let list_widget = List::new(list_items).highlight_style(Style::default().bg(Color::Blue));
    let inner_chunk = draw_panel(f, list.get_title(), chunk, selected);
    f.render_stateful_widget(list_widget, inner_chunk, &mut state);
}

pub fn draw_table<B, T>(f: &mut Frame<B>, table: &T, chunk: Rect, selected: bool)
where
    B: Backend,
    T: TableView,
{
    // TODO: theming
    let len = table.len();
    let layout = table.get_layout();
    let table_items = table.get_items().map(|item| Row::new(item));
    let number_items = table.len();
    // Minus for height of block and heading.
    let table_height = chunk.height.saturating_sub(4) as usize;
    let offset = table.get_offset(table_height);
    let table_widths =
        basic_constraints_to_constraints(table.get_layout(), chunk.width.saturating_sub(2), 1); // Minus block
    info!("chunk {}, table {:?}", chunk.width, table_widths);
    let mut table_state = TableState::default()
        .with_selected(Some(table.get_selected_item()))
        .with_offset(offset);
    let table_widget = Table::new(table_items)
        .highlight_style(Style::default().bg(Color::Blue))
        .header(
            Row::new(table.get_headings()).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::LightGreen),
            ),
        )
        .widths(table_widths.as_slice())
        .column_spacing(1);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol(block::FULL)
        .track_symbol(line::VERTICAL)
        .begin_symbol(None)
        .end_symbol(None);
    let scrollable_lines = number_items.saturating_sub(table_height) as u16;
    let mut scrollbar_state = ScrollbarState::default()
        .position(offset.min(scrollable_lines as usize) as u16)
        .content_length(scrollable_lines);
    let inner_chunk = draw_panel(f, table.get_title(), chunk, selected);
    if table.is_loading() {
        draw_loading(f, inner_chunk)
    } else {
        f.render_stateful_widget(table_widget, inner_chunk, &mut table_state);
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

pub fn draw_loading<B: Backend>(f: &mut Frame<B>, chunk: Rect) {
    let loading = Paragraph::new("Loading");
    f.render_widget(loading, chunk);
}
