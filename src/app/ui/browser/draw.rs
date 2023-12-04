use super::artistalbums::AlbumSongsPanel;
use super::{artistalbums::ArtistInputRouting, Browser, InputRouting};
use crate::app::component::actionhandler::Suggestable;
use crate::app::view::draw::{draw_list, draw_sortable_table, draw_table};
use crate::app::view::{SortableTableView, TableView};
use crate::drawutils::{
    below_left_rect, bottom_of_rect, ROW_HIGHLIGHT_COLOUR, SELECTED_BORDER_COLOUR, TEXT_COLOUR,
};
use ratatui::prelude::Alignment;
use ratatui::widgets::block::Title;
use ratatui::widgets::{TableState, Wrap};
use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use ytmapi_rs::common::{SuggestionType, TextRun};

pub fn draw_browser(
    f: &mut Frame,
    browser: &Browser,
    chunk: Rect,
    artist_list_state: &mut ListState,
    album_songs_table_state: &mut TableState,
) {
    let layout = Layout::new()
        .constraints([Constraint::Max(30), Constraint::Min(0)])
        .direction(ratatui::prelude::Direction::Horizontal)
        .split(chunk);
    // XXX: Naive implementation.
    let _albumsongsselected = browser.input_routing == InputRouting::Song;
    let _artistselected =
        !_albumsongsselected && browser.artist_list.route == ArtistInputRouting::List;

    if !browser.artist_list.search_popped {
        draw_list(
            f,
            &browser.artist_list,
            layout[0],
            _artistselected,
            artist_list_state,
        );
    } else {
        let s = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(layout[0]);
        draw_list(
            f,
            &browser.artist_list,
            s[1],
            _artistselected,
            artist_list_state,
        );
        draw_sort_box(f, &browser, s[0]);
        // Should this be part of draw_search_box
        if browser.has_search_suggestions() {
            draw_search_suggestions(f, &browser, s[0], layout[0])
        }
    }
    draw_sortable_table(
        f,
        &browser.album_songs_list,
        layout[1],
        album_songs_table_state,
        _albumsongsselected,
    );
    if browser.album_songs_list.sort.sort_popped {
        draw_sort_popup(f, &browser.album_songs_list, layout[1]);
    }
}

// TODO: Generalize
fn draw_sort_popup(f: &mut Frame, album_songs_panel: &AlbumSongsPanel, chunk: Rect) {
    let title_l = "Sort";
    let title_r = "Clear: C / Cancel: Esc";
    let footer = "Asc: Enter / Desc: Alt-Enter";
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
    let width = max_header_len.max(footer.len()) + 2;
    let height = sortable_columns.len() + 2;
    let popup_chunk = crate::drawutils::centered_rect(height as u16, width as u16, chunk);
    // TODO: Save the state.
    let mut state = ListState::default().with_selected(Some(album_songs_panel.sort.sort_cur));
    let list = List::new(headers)
        .highlight_style(Style::default().bg(ROW_HIGHLIGHT_COLOUR))
        .block(
            Block::new()
                .title(title_l)
                .title(Title::from(title_r).alignment(Alignment::Right))
                .title(Title::from(footer).position(ratatui::widgets::block::Position::Bottom))
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SELECTED_BORDER_COLOUR)),
        );
    f.render_widget(Clear, popup_chunk);
    f.render_stateful_widget(list, popup_chunk, &mut state);
}

fn draw_sort_box(f: &mut Frame, browser: &Browser, chunk: Rect) {
    let search_widget = Paragraph::new(browser.artist_list.search.search_contents.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SELECTED_BORDER_COLOUR))
            .title("Search"),
    );
    f.render_widget(search_widget, chunk);
    f.set_cursor(
        chunk.x + browser.artist_list.search.text_cur as u16 + 1,
        chunk.y + 1,
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
        .into_iter()
        .map(|s| {
            ListItem::new(Line::from(
                std::iter::once(s.get_type())
                    .map(|ty| match ty {
                        SuggestionType::History => Span::raw(" "),
                        SuggestionType::Prediction => Span::raw(" "),
                    })
                    .chain(s.get_runs().iter().map(|s| match s {
                        TextRun::Bold(str) => {
                            Span::styled(str, Style::new().add_modifier(Modifier::BOLD))
                        }
                        TextRun::Normal(str) => Span::raw(str),
                    }))
                    // XXX: Ratatui upgrades may allow this to be passed lazily instead of collecting.
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
