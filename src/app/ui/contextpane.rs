use std::borrow::Cow;

use ratatui::{
    prelude::{Backend, Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Row, Table},
    Frame,
};

use super::draw::left_bottom_corner_rect;
use super::{
    actionhandler::{Action, ActionProcessor, KeyRouter},
    view::Drawable,
};

// A pane of the application. This is the place that renders in the app and handles key events.
// XXX: May be redundant - consider removing.
pub trait ContextPane<A: Action + Clone>: ActionProcessor<A> + KeyRouter<A> + Drawable {}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// A window context containing multiple panes for which input should be easily swapped.
trait MultiPane {
    fn select(&mut self, dir: Direction);
    // For example, tabcycling
    fn select_next(&mut self);
}
