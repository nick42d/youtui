use std::collections::HashMap;

pub struct RgbColour {
    r: u16,
    g: u16,
    b: u16,
}

pub struct Theme {
    app_bg_colour: String,
    selected_panel_border_colour: String,
    selected_panel_text_colour: String,
    deselected_panel_border_colour: String,
    deselected_panel_text_colour: String,
    selected_text_colour: String,
    deselected_text_colour: String,
    button_bg_colour: String,
    button_fg_colour: String,
    progress_bg_colour: String,
    progress_fg_colour: String,
    progress_text_colour: String,
    table_headings_colour: String,
    row_highlight_colour: String,
    palette: HashMap<String, RgbColour>,
}
