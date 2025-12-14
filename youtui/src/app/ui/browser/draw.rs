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
use crate::app::view::draw::{draw_advanced_table, draw_list, draw_loadable, draw_panel_mut};
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

pub fn draw_browser(f: &mut Frame, browser: &mut Browser, chunk: Rect, selected: bool) {
    match browser.variant {
        super::BrowserVariant::Artist => {
            draw_artist_search_browser(f, &mut browser.artist_search_browser, chunk, selected)
        }
        super::BrowserVariant::Song => {
            draw_song_search_browser(f, &mut browser.song_search_browser, chunk, selected)
        }
        super::BrowserVariant::Playlist => {
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
    let [artists_chunk, songs_chunk] = Layout::new(
        ratatui::prelude::Direction::Horizontal,
        [Constraint::Max(30), Constraint::Min(0)],
    )
    .areas(chunk);
    // Potentially could handle this better.
    let albumsongsselected = selected
        && browser.input_routing == artistsearch::InputRouting::Song
        && browser.album_songs_panel.route == AlbumSongsInputRouting::List;
    let artistselected = !albumsongsselected
        && selected
        && browser.input_routing == artistsearch::InputRouting::Artist
        && browser.artist_search_panel.route == ArtistInputRouting::List;

    if !browser.artist_search_panel.search_popped {
        draw_panel_mut(
            f,
            &mut browser.artist_search_panel,
            artists_chunk,
            artistselected,
            |t, f, chunk| {
                draw_list(f, t, chunk);
                None
            },
        );
    } else {
        let [search_box_chunk, shrunk_artists_chunk] = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .areas(artists_chunk);
        draw_panel_mut(
            f,
            &mut browser.artist_search_panel,
            shrunk_artists_chunk,
            artistselected,
            |t, f, chunk| {
                draw_list(f, t, chunk);
                None
            },
        );
        draw_search_box(
            f,
            "Search Artists",
            &mut browser.artist_search_panel.search,
            search_box_chunk,
        );
        // Should this be part of draw_search_box
        if browser.artist_search_panel.has_search_suggestions() {
            draw_search_suggestions(
                f,
                &browser.artist_search_panel.search,
                search_box_chunk,
                artists_chunk,
            )
        }
    }
    draw_panel_mut(
        f,
        &mut browser.album_songs_panel,
        songs_chunk,
        albumsongsselected,
        |t, f, chunk| {
            draw_loadable(f, t, chunk, |t, f, chunk| {
                Some(draw_advanced_table(f, t, chunk))
            })
        },
    );
}
pub fn draw_playlist_search_browser(
    f: &mut Frame,
    browser: &mut PlaylistSearchBrowser,
    chunk: Rect,
    selected: bool,
) {
    let [playlists_chunk, songs_chunk] = Layout::new(
        ratatui::prelude::Direction::Horizontal,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .areas(chunk);
    // Potentially could handle this better.
    let songs_selected = selected
        && browser.input_routing == playlistsearch::InputRouting::Song
        && browser.playlist_songs_panel.route == PlaylistSongsInputRouting::List;
    let playlists_selected = !songs_selected
        && selected
        && browser.input_routing == playlistsearch::InputRouting::Playlist
        && browser.playlist_search_panel.route == PlaylistInputRouting::List;

    if !browser.playlist_search_panel.search_popped {
        draw_panel_mut(
            f,
            &mut browser.playlist_search_panel,
            playlists_chunk,
            playlists_selected,
            |t, f, chunk| {
                draw_list(f, t, chunk);
                None
            },
        );
    } else {
        let [search_box_chunk, shrunk_playlists_chunk] = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .areas(playlists_chunk);
        draw_panel_mut(
            f,
            &mut browser.playlist_search_panel,
            shrunk_playlists_chunk,
            playlists_selected,
            |t, f, chunk| {
                draw_list(f, t, chunk);
                None
            },
        );
        draw_search_box(
            f,
            "Search Playlists",
            &mut browser.playlist_search_panel.search,
            search_box_chunk,
        );
        // Should this be part of draw_search_box
        if browser.playlist_search_panel.has_search_suggestions() {
            draw_search_suggestions(
                f,
                &browser.playlist_search_panel.search,
                search_box_chunk,
                playlists_chunk,
            )
        }
    }
    draw_panel_mut(
        f,
        &mut browser.playlist_songs_panel,
        songs_chunk,
        songs_selected,
        |t, f, chunk| {
            draw_loadable(f, t, chunk, |t, f, chunk| {
                Some(draw_advanced_table(f, t, chunk))
            })
        },
    );
}
pub fn draw_song_search_browser(
    f: &mut Frame,
    browser: &mut SongSearchBrowser,
    chunk: Rect,
    selected: bool,
) {
    if !browser.search_popped {
        draw_panel_mut(f, browser, chunk, selected, |t, f, chunk| {
            draw_loadable(f, t, chunk, |t, f, chunk| {
                Some(draw_advanced_table(f, t, chunk))
            })
        });
    } else {
        let [search_box_chunk, new_chunk] = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .areas(chunk);
        draw_panel_mut(f, browser, new_chunk, false, |t, f, chunk| {
            draw_loadable(f, t, chunk, |t, f, chunk| {
                Some(draw_advanced_table(f, t, chunk))
            })
        });
        draw_search_box(f, "Search Songs", &mut browser.search, search_box_chunk);
        // Should this be part of draw_search_box
        if browser.has_search_suggestions() {
            draw_search_suggestions(f, &browser.search, search_box_chunk, chunk)
        }
    }
}

/// Draw a text input box
// TODO: Shift to a more general module.
pub fn draw_text_box(
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
    let [suggestion_side_borders_chunk, suggestion_list_chunk] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .areas(suggestion_chunk);
    let mut list_state = ListState::default().with_selected(search.suggestions_cur);
    let list_items = suggestions.iter().map(|s| {
        ListItem::new(Line::from_iter(
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
                })),
        ))
    });
    let block = List::new(list_items)
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
    f.render_widget(side_borders, suggestion_side_borders_chunk);
    f.render_widget(Clear, divider_chunk);
    f.render_widget(divider, divider_chunk);
    f.render_stateful_widget(block, suggestion_list_chunk, &mut list_state);
}
