use super::Browser;
use super::artistsearch::search_panel::ArtistInputRouting;
use super::artistsearch::songs_panel::AlbumSongsInputRouting;
use super::artistsearch::{self, ArtistSearchBrowser};
use super::shared_components::SearchBlock;
use super::songsearch::SongSearchBrowser;
use crate::app::component::actionhandler::Suggestable;
use crate::app::ui::browser::playlistsearch::search_panel::PlaylistInputRouting;
use crate::app::ui::browser::playlistsearch::songs_panel::PlaylistSongsInputRouting;
use crate::app::ui::browser::playlistsearch::{self, PlaylistSearchBrowser};
use crate::app::view::SortableTableView;
use crate::app::view::draw::{draw_list, draw_sortable_table};
use crate::drawutils::{
    ROW_HIGHLIGHT_COLOUR, SELECTED_BORDER_COLOUR, TEXT_COLOUR, below_left_rect, bottom_of_rect,
};
use rat_text::HasScreenCursor;
use rat_text::text_input::{TextInput, TextInputState};
use ratatui::Frame;
use ratatui::prelude::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ytmapi_rs::common::{SuggestionType, TextRun};

// Popups look aesthetically weird when really small, so setting a minimum.
const MIN_POPUP_WIDTH: usize = 20;

pub fn draw_browser(f: &mut Frame, browser: &mut Browser, chunk: Rect, selected: bool) {
    match browser.variant {
        super::BrowserVariant::ArtistSearch => {
            draw_artist_search_browser(f, &mut browser.artist_search_browser, chunk, selected)
        }
        super::BrowserVariant::SongSearch => {
            draw_song_search_browser(f, &mut browser.song_search_browser, chunk, selected)
        }
        super::BrowserVariant::PlaylistSearch => {
            draw_playlist_search_browser(f, &mut browser.playlist_search_browser, chunk, selected)
        }
    }
}
pub fn draw_artist_search_browser(
    f: &mut Frame,
    browser: &mut ArtistSearchBrowser,
    chunk: Rect,
    selected: bool,
) {
    let layout = Layout::new(
        ratatui::prelude::Direction::Horizontal,
        [Constraint::Max(30), Constraint::Min(0)],
    )
    .split(chunk);
    // Potentially could handle this better.
    let albumsongsselected = selected
        && browser.input_routing == artistsearch::InputRouting::Song
        && browser.album_songs_panel.route == AlbumSongsInputRouting::List;
    let artistselected = !albumsongsselected
        && selected
        && browser.input_routing == artistsearch::InputRouting::Artist
        && browser.artist_search_panel.route == ArtistInputRouting::List;

    if !browser.artist_search_panel.search_popped {
        browser.artist_search_panel.widget_state =
            draw_list(f, &browser.artist_search_panel, layout[0], artistselected);
    } else {
        let s = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(layout[0]);
        browser.artist_search_panel.widget_state =
            draw_list(f, &browser.artist_search_panel, s[1], artistselected);
        draw_search_box(
            f,
            "Search Artists",
            &mut browser.artist_search_panel.search,
            s[0],
        );
        // Should this be part of draw_search_box
        if browser.artist_search_panel.has_search_suggestions() {
            draw_search_suggestions(f, &browser.artist_search_panel.search, s[0], layout[0])
        }
    }
    browser.album_songs_panel.widget_state =
        draw_sortable_table(f, &browser.album_songs_panel, layout[1], albumsongsselected);
    if browser.album_songs_panel.sort.shown {
        browser.album_songs_panel.sort.state =
            draw_sort_popup(f, &browser.album_songs_panel, layout[1]);
    }
    if browser.album_songs_panel.filter.shown {
        draw_filter_popup(
            f,
            &mut browser.album_songs_panel.filter.filter_text,
            layout[1],
        );
    }
}
pub fn draw_playlist_search_browser(
    f: &mut Frame,
    browser: &mut PlaylistSearchBrowser,
    chunk: Rect,
    selected: bool,
) {
    let layout = Layout::new(
        ratatui::prelude::Direction::Horizontal,
        [Constraint::Max(30), Constraint::Min(0)],
    )
    .split(chunk);
    // Potentially could handle this better.
    let albumsongsselected = selected
        && browser.input_routing == playlistsearch::InputRouting::Song
        && browser.playlist_songs_panel.route == PlaylistSongsInputRouting::List;
    let playlistselected = !albumsongsselected
        && selected
        && browser.input_routing == playlistsearch::InputRouting::Playlist
        && browser.playlist_search_panel.route == PlaylistInputRouting::List;

    if !browser.playlist_search_panel.search_popped {
        browser.playlist_search_panel.widget_state = draw_list(
            f,
            &browser.playlist_search_panel,
            layout[0],
            playlistselected,
        );
    } else {
        let s = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(layout[0]);
        browser.playlist_search_panel.widget_state =
            draw_list(f, &browser.playlist_search_panel, s[1], playlistselected);
        draw_search_box(
            f,
            "Search Playlists",
            &mut browser.playlist_search_panel.search,
            s[0],
        );
        // Should this be part of draw_search_box
        if browser.playlist_search_panel.has_search_suggestions() {
            draw_search_suggestions(f, &browser.playlist_search_panel.search, s[0], layout[0])
        }
    }
    browser.playlist_songs_panel.widget_state = draw_sortable_table(
        f,
        &browser.playlist_songs_panel,
        layout[1],
        albumsongsselected,
    );
    if browser.playlist_songs_panel.sort.shown {
        browser.playlist_songs_panel.sort.state =
            draw_sort_popup(f, &browser.playlist_songs_panel, layout[1]);
    }
    if browser.playlist_songs_panel.filter.shown {
        draw_filter_popup(
            f,
            &mut browser.playlist_songs_panel.filter.filter_text,
            layout[1],
        );
    }
}
pub fn draw_song_search_browser(
    f: &mut Frame,
    browser: &mut SongSearchBrowser,
    chunk: Rect,
    selected: bool,
) {
    if !browser.search_popped {
        browser.widget_state = draw_sortable_table(f, browser, chunk, selected);
    } else {
        let s = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(chunk);
        browser.widget_state = draw_sortable_table(f, browser, s[1], false);
        draw_search_box(f, "Search Songs", &mut browser.search, s[0]);
        // Should this be part of draw_search_box
        if browser.has_search_suggestions() {
            draw_search_suggestions(f, &browser.search, s[0], chunk)
        }
    }
    if browser.sort.shown {
        browser.sort.state = draw_sort_popup(f, browser, chunk);
    }
    if browser.filter.shown {
        draw_filter_popup(f, &mut browser.filter.filter_text, chunk);
    }
}

/// Returns a new ListState for the sort popup.
fn draw_sort_popup(f: &mut Frame, table: &impl SortableTableView, chunk: Rect) -> ListState {
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
    // TODO: Save the state.
    let mut state = table
        .get_sort_popup_state()
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
    f.render_stateful_widget(list, popup_chunk, &mut state);
    state
}

fn draw_filter_popup(f: &mut Frame, state: &mut TextInputState, chunk: Rect) {
    let title = "Filter";
    // Hardocde dimensions of filter input.
    let popup_chunk = crate::drawutils::centered_rect(3, 22, chunk);
    f.render_widget(Clear, popup_chunk);
    draw_text_box(f, title, state, popup_chunk);
}

/// Draw a text input box
// TODO: Shift to a more general module.
fn draw_text_box(
    f: &mut Frame,
    title: impl AsRef<str>,
    contents: &mut TextInputState,
    chunk: Rect,
) {
    let block_widget = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(SELECTED_BORDER_COLOUR))
        .title(title.as_ref());
    let text_chunk = block_widget.inner(chunk);
    let text_chunk = Rect {
        x: text_chunk.x,
        y: text_chunk.y,
        width: text_chunk.width.saturating_sub(1),
        height: text_chunk.height,
    };
    // TODO: Scrolling, if input larger than box.
    let text_widget = TextInput::new();
    f.render_widget(block_widget, chunk);
    f.render_stateful_widget(text_widget, text_chunk, contents);
    if let Some(cursor_pos) = contents.screen_cursor() {
        f.set_cursor_position(cursor_pos)
    };
}
fn draw_search_box(f: &mut Frame, title: impl AsRef<str>, search: &mut SearchBlock, chunk: Rect) {
    draw_text_box(f, title, &mut search.search_contents, chunk);
}

fn draw_search_suggestions(f: &mut Frame, search: &SearchBlock, chunk: Rect, max_bounds: Rect) {
    let suggestions = search.get_search_suggestions();
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
    let mut list_state = ListState::default().with_selected(search.suggestions_cur);
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
