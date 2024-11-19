use super::artistalbums::albumsongs::{AlbumSongsInputRouting, AlbumSongsPanel};
use super::artistalbums::artistsearch::ArtistInputRouting;
use super::{Browser, InputRouting};
use crate::app::component::actionhandler::Suggestable;
use crate::app::view::draw::{draw_list, draw_sortable_table};
use crate::app::view::{SortableTableView, TableView};
use crate::drawutils::{
    below_left_rect, bottom_of_rect, ROW_HIGHLIGHT_COLOUR, SELECTED_BORDER_COLOUR, TEXT_COLOUR,
};
use rat_text::text_input::{TextInput, TextInputState};
use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use ytmapi_rs::common::{SuggestionType, TextRun};

// Popups look aesthetically weird when really small, so setting a minimum.
const MIN_POPUP_WIDTH: usize = 20;

pub fn draw_browser(f: &mut Frame, browser: &mut Browser, chunk: Rect, selected: bool) {
    let layout = Layout::new(
        ratatui::prelude::Direction::Horizontal,
        [Constraint::Max(30), Constraint::Min(0)],
    )
    .split(chunk);
    // Potentially could handle this better.
    let albumsongsselected = selected
        && browser.input_routing == InputRouting::Song
        && browser.album_songs_list.route == AlbumSongsInputRouting::List;
    let artistselected = !albumsongsselected
        && selected
        && browser.input_routing == InputRouting::Artist
        && browser.artist_list.route == ArtistInputRouting::List;

    if !browser.artist_list.search_popped {
        browser.artist_list.widget_state =
            draw_list(f, &browser.artist_list, layout[0], artistselected);
    } else {
        let s = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(layout[0]);
        browser.artist_list.widget_state = draw_list(f, &browser.artist_list, s[1], artistselected);
        draw_search_box(f, browser, s[0]);
        // Should this be part of draw_search_box
        if browser.has_search_suggestions() {
            draw_search_suggestions(f, browser, s[0], layout[0])
        }
    }
    browser.album_songs_list.widget_state =
        draw_sortable_table(f, &browser.album_songs_list, layout[1], albumsongsselected);
    if browser.album_songs_list.sort.shown {
        draw_sort_popup(f, &browser.album_songs_list, layout[1]);
    }
    if browser.album_songs_list.filter.shown {
        draw_filter_popup(f, &mut browser.album_songs_list, layout[1]);
    }
}

// TODO: Generalize
fn draw_sort_popup(f: &mut Frame, album_songs_panel: &AlbumSongsPanel, chunk: Rect) {
    let title = "Sort";
    let sortable_columns = album_songs_panel.get_sortable_columns();
    let headers: Vec<_> = album_songs_panel
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
    // TODO: Save the state.
    let mut state = ListState::default().with_selected(Some(album_songs_panel.sort.cur));
    let list = List::new(headers)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .block(
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SELECTED_BORDER_COLOUR)),
        );
    f.render_widget(Clear, popup_chunk);
    f.render_stateful_widget(list, popup_chunk, &mut state);
}

fn draw_filter_popup(f: &mut Frame, album_songs_panel: &mut AlbumSongsPanel, chunk: Rect) {
    let title = "Filter";
    // Hardocde dimensions of filter input.
    let popup_chunk = crate::drawutils::centered_rect(3, 22, chunk);
    f.render_widget(Clear, popup_chunk);
    draw_text_box(
        f,
        title,
        &mut album_songs_panel.filter.filter_text,
        popup_chunk,
    );
}

/// Draw a text input box
// TODO: Shift to a more general module.
fn draw_text_box(
    f: &mut Frame,
    title: impl AsRef<str>,
    contents: &mut TextInputState,
    chunk: Rect,
) {
    // TODO: Scrolling, if input larger than box.
    let text_widget = TextInput::new().block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SELECTED_BORDER_COLOUR))
            .title(title.as_ref()),
    );
    f.render_stateful_widget(text_widget, chunk, contents);
}
fn draw_search_box(f: &mut Frame, browser: &mut Browser, chunk: Rect) {
    draw_text_box(
        f,
        "Search",
        &mut browser.artist_list.search.search_contents,
        chunk,
    );
}

fn draw_search_suggestions(f: &mut Frame, browser: &Browser, chunk: Rect, max_bounds: Rect) {
    let suggestions = browser.get_search_suggestions();
    let height = suggestions.len() + 1;
    let divider_chunk = bottom_of_rect(chunk);
    let suggestion_chunk = below_left_rect(
        height.try_into().unwrap_or(u16::MAX),
        chunk.width,
        chunk,
        max_bounds,
    );
    let suggestion_chunk_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(suggestion_chunk);
    let mut list_state =
        ListState::default().with_selected(browser.artist_list.search.suggestions_cur);
    let list: Vec<_> = suggestions
        .iter()
        .map(|s| {
            ListItem::new(Line::from(
                std::iter::once(s.suggestion_type)
                    .map(|ty| match ty {
                        SuggestionType::History => Span::raw(" "),
                        SuggestionType::Prediction => Span::raw(" "),
                    })
                    .chain(s.runs.iter().map(|s| match s {
                        TextRun::Bold(str) => {
                            Span::styled(str, Style::new().add_modifier(Modifier::BOLD))
                        }
                        TextRun::Normal(str) => Span::raw(str),
                    }))
                    // XXX: Ratatui upgrades may allow this to be passed lazily instead of
                    // collecting.
                    .collect::<Vec<Span>>(),
            ))
        })
        // XXX: Ratatui upgrades may allow this to be passed lazily instead of collecting.
        .collect();
    let block = List::new(list)
        .style(Style::new().fg(TEXT_COLOUR))
        .highlight_style(Style::new().bg(ROW_HIGHLIGHT_COLOUR))
        .block(
            Block::default()
                .borders(Borders::all().difference(Borders::TOP))
                .style(Style::new().fg(SELECTED_BORDER_COLOUR)),
        );
    let side_borders = Block::default()
        .borders(Borders::LEFT.union(Borders::RIGHT))
        .style(Style::new().fg(SELECTED_BORDER_COLOUR));
    let divider = Block::default().borders(Borders::TOP);
    f.render_widget(Clear, suggestion_chunk);
    f.render_widget(side_borders, suggestion_chunk_layout[0]);
    f.render_widget(Clear, divider_chunk);
    f.render_widget(divider, divider_chunk);
    f.render_stateful_widget(block, suggestion_chunk_layout[1], &mut list_state);
}
