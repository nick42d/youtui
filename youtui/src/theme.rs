use ratatui::style::Color;
use std::collections::HashMap;

pub struct RgbColour {
    r: u16,
    g: u16,
    b: u16,
}

pub struct Theme2<P> {
    pallete: HashMap<P, RgbColour>,
}

pub struct Theme {
    selected_panel_border_colour: Color,
    selected_panel_text_colour: Color,
    deselected_panel_border_colour: Color,
    deselected_panel_text_colour: Color,
    selected_text_colour: Color,
    selected_text_bg_colour: Color,
    deselected_text_colour: Color,
    error_text_colour: Color,
    warn_text_colour: Color,
    info_text_colour: Color,
    debug_text_colour: Color,
    trace_text_colour: Color,
    button_bg_colour: Color,
    button_fg_colour: Color,
    progress_bg_colour: Color,
    progress_fg_colour: Color,
    table_headings_colour: Color,
}

fn default_theme() -> Theme {
    let selected_border_colour = Color::Cyan;
    let deselected_border_colour = Color::Reset;
    Theme {
        selected_panel_border_colour: selected_border_colour,
        selected_panel_text_colour: selected_border_colour,
        deselected_panel_border_colour: deselected_border_colour,
        deselected_panel_text_colour: deselected_border_colour,
        selected_text_colour: Color::Cyan,
        selected_text_bg_colour: Color::Green,
        deselected_text_colour: Color::Reset,
        error_text_colour: Color::Red,
        warn_text_colour: Color::Yellow,
        info_text_colour: Color::Cyan,
        debug_text_colour: Color::LightCyan,
        trace_text_colour: Color::Gray,
        button_bg_colour: Color::Gray,
        button_fg_colour: Color::Black,
        progress_bg_colour: Color::DarkGray,
        progress_fg_colour: Color::LightGreen,
        table_headings_colour: Color::LightGreen,
    }
}
