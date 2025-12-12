use super::{
    AdvancedTableView, TableSortCommand, TableView, basic_constraints_to_table_constraints,
};
use crate::app::ui::browser::shared_components::{FilterManager, SortManager};
use crate::app::ui::draw::draw_text_box;
use crate::app::view::{BasicConstraint, HasTitle, ListView, Loadable};
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

/// Draw inside a panel.
pub fn draw_panel_mut<T: HasTitle>(
    f: &mut Frame,
    t: &mut T,
    chunk: Rect,
    is_selected: bool,
    draw_call: impl for<'a> FnOnce(&'a mut T, &mut Frame, Rect) -> Option<PanelEffect<'static>>,
) {
    draw_panel_mut_impl(f, t, chunk, is_selected, |t| t.get_title(), draw_call);
}

/// Draw inside a panel.
pub fn draw_panel_mut_impl<T>(
    f: &mut Frame,
    t: &mut T,
    chunk: Rect,
    is_selected: bool,
    get_title: impl for<'a> FnOnce(&'a mut T) -> Cow<'a, str>,
    draw_call: impl for<'a> FnOnce(&'a mut T, &mut Frame, Rect) -> Option<PanelEffect<'static>>,
) {
    let border_colour = if is_selected {
        SELECTED_BORDER_COLOUR
    } else {
        DESELECTED_BORDER_COLOUR
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_colour));
    let inner_chunk = block.inner(chunk);
    let effect = draw_call(t, f, inner_chunk);
    let block = block.title(get_title(t));
    if let Some(effect) = effect {
        effect.apply_and_render(block, f, chunk);
    } else {
        f.render_widget(block, chunk);
    }
}

pub fn draw_loadable<T, E, W>(
    f: &mut Frame,
    t: &mut T,
    chunk: Rect,
    draw_call: impl for<'a> FnOnce(&'a mut T, &mut Frame, Rect) -> Option<E>,
) -> Option<E>
where
    T: Loadable,
    E: WidgetEffect<W>,
    W: Widget,
{
    if t.is_loading() {
        let loading = Paragraph::new("Loading");
        f.render_widget(loading, chunk);
        return None;
    };
    draw_call(t, f, chunk)
}

pub fn draw_list(f: &mut Frame, list: &mut impl ListView, chunk: Rect) {
    let selected_item = list.get_selected_item();
    list.get_mut_state().select(Some(selected_item));
    // TODO: Scroll bars
    let list_widget =
        List::new(list.get_items()).highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR));
    // ListState is cheap to clone
    *list.get_mut_state() =
        move_render_stateful_widget(f, list_widget, chunk, list.get_state().clone());
}

#[must_use = "PanelEffect does nothing if it is not used"]
pub struct PanelEffect<'a> {
    footer: Option<Cow<'a, str>>,
    scrollbar: Option<ScrollbarState>,
}
impl<'a> WidgetEffect<Block<'_>> for PanelEffect<'a> {
    fn apply_and_render(self, panel: Block<'_>, f: &mut Frame, chunk: Rect) {
        if let Some(footer) = self.footer {
            let panel = panel.title_bottom(footer);
            f.render_widget(panel, chunk);
        } else {
            f.render_widget(panel, chunk);
        }
        if let Some(mut scrollbar_state) = self.scrollbar {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_symbol(block::FULL)
                .track_symbol(Some(line::VERTICAL))
                .begin_symbol(None)
                .end_symbol(None);
            f.render_stateful_widget(
                scrollbar,
                chunk.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }
}
/// WidgetEffect represents an effect to be applied to a widget.
/// This allows Child widgets to apply effects to their parents in a controlled,
/// testable manner.
pub trait WidgetEffect<T>
where
    T: Widget,
{
    fn apply_and_render(self, widget: T, f: &mut Frame, chunk: Rect);
}

pub fn draw_table_impl<'a>(
    f: &mut Frame,
    chunk: Rect,
    cur: usize,
    highlighted: Option<usize>,
    state: &TableState,
    items: impl Iterator<Item = impl Iterator<Item = Cow<'a, str>> + 'a> + 'a,
    len: usize,
    layout: &'a [BasicConstraint],
    headings: impl Iterator<Item = impl Into<Cell<'static>>>,
    footer: Option<String>,
) -> (TableState, PanelEffect<'static>) {
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
    let table_height = chunk.height as usize;
    let table_widths = basic_constraints_to_table_constraints(layout, chunk.width, 1);
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
    let scrollable_lines = len.saturating_sub(table_height);
    let pos = state.offset().min(scrollable_lines);
    let new_state = move_render_stateful_widget(f, table_widget, chunk, new_state);
    // Call this after rendering table, as offset is mutated.
    let scrollbar_state = ScrollbarState::default()
        .position(pos)
        .content_length(scrollable_lines);
    (
        new_state,
        PanelEffect {
            footer: footer.map(Into::into),
            scrollbar: Some(scrollbar_state),
        },
    )
}

/// Returns a PanelEffect that can be used if rendered in a scrollable
/// panel.
pub fn draw_table<T>(f: &mut Frame, table: &mut T, chunk: Rect) -> PanelEffect<'static>
where
    T: TableView,
{
    let items = table.get_items();
    let len = items.len();
    let (new_table_state, effect) = draw_table_impl(
        f,
        chunk,
        table.get_selected_item(),
        table.get_highlighted_row(),
        table.get_state(),
        items,
        len,
        table.get_layout(),
        table.get_headings(),
        None,
    );

    *table.get_mut_state() = new_table_state;
    effect
}

/// Returns a PanelEffect that can be used if rendered in a scrollable
/// panel.
pub fn draw_advanced_table<'a>(
    f: &mut Frame,
    table: &'a mut impl AdvancedTableView,
    chunk: Rect,
) -> PanelEffect<'static> {
    // Set the state to the currently selected item.
    let selected_item = table.get_selected_item();
    table.get_mut_state().select(Some(selected_item));
    // Likely expensive, and could be optimised.
    let number_items = table.get_filtered_items().count();
    // Minus for height of block and heading.
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
    let filter_string = if filter_str.len() > 1 {
        ": ".to_string() + &filter_str
    } else {
        filter_str
    };
    // Clone of TableState is cheap
    let new_table_state = table.get_state().clone();
    let (new_table_state, effect) = draw_table_impl(
        f,
        chunk,
        table.get_selected_item(),
        table.get_highlighted_row(),
        &new_table_state,
        table.get_filtered_items(),
        number_items,
        table.get_layout(),
        combined_headings,
        Some(filter_string),
    );
    *table.get_mut_state() = new_table_state;

    if table.sort_popup_shown() {
        draw_sort_popup(f, table, chunk);
    }

    if table.filter_popup_shown() {
        draw_filter_popup(f, table, chunk);
    }
    effect
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
    draw_text_box(f, title, table.get_mut_filter_state(), popup_chunk);
}
