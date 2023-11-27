use super::{
    super::view::Drawable,
    actionhandler::{Action, ActionProcessor, KeyRouter},
};

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
