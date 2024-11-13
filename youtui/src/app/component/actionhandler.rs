use crate::app::keycommand::{CommandVisibility, DisplayableCommand, KeyCommand, Keymap};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::borrow::Cow;
use ytmapi_rs::common::SearchSuggestion;

// An action that can be sent to a component.
pub trait Action {
    fn context(&self) -> Cow<str>;
    fn describe(&self) -> Cow<str>;
}
/// A component of the application that has different keybinds depending on what
/// is focussed. For example, keybinds for browser may differ depending on
/// selected pane. A keyrouter does not necessarily need to be a keyhandler and
/// vice-versa. e.g a component that routes all keys and doesn't have its own
/// commands, Or a component that handles but does not route.
/// Not every KeyHandler is a KeyRouter - e.g the individual panes themselves.
/// NOTE: To implment this, the component can only have a single Action type.
// XXX: Could possibly be a part of EventHandler instead.
// XXX: Does this actually need to be a keyhandler?
pub trait KeyRouter<A: Action> {
    /// Get the list of active keybinds that the component and its route
    /// contain.
    fn get_routed_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a>;
    /// Get the list of keybinds that the component and any child items can
    /// contain, regardless of current route.
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a>;
    // e.g - for use in help menu.
    fn get_all_visible_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a> {
        Box::new(
            self.get_all_keybinds()
                .filter(|kb| kb.visibility != CommandVisibility::Hidden),
        )
    }
    // e.g - for use in header.
    fn get_routed_global_keybinds<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a> {
        Box::new(
            self.get_routed_keybinds()
                .filter(|kb| kb.visibility == CommandVisibility::Global),
        )
    }
}
/// A component of the application that can block parent keybinds.
/// For example, a component that can display a modal dialog that will prevent
/// other inputs.
pub trait DominantKeyRouter {
    /// Return true if dominant keybinds are active.
    fn dominant_keybinds_active(&self) -> bool;
}

/// A component of the application that can display all it's keybinds.
/// Not every KeyHandler/KeyRouter is a DisplayableKeyRouter - as
/// DisplayAbleKeyRouter unables conversion of typed Actions to generic.
// TODO: Type safety
// Could possibly be a part of EventHandler instead.
pub trait KeyDisplayer {
    // XXX: Can these all just be derived from KeyRouter?
    /// Get the list of all keybinds that the KeyHandler and any child items can
    /// contain, regardless of context.
    fn get_all_visible_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand<'a>> + 'a>;
    /// Get the list of all non-hidden keybinds that the KeyHandler and any
    /// child items can contain, regardless of context.
    fn get_all_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand<'a>> + 'a>;
    /// Get a context-specific list of all keybinds marked global.
    // TODO: Put under DisplayableKeyHandler
    fn get_context_global_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand<'a>> + 'a>;
}
/// A component of the application that handles text entry.
// TODO: Cursor position and movement.
pub trait TextHandler {
    // TODO: cursor manipulation
    fn push_text(&mut self, c: char);
    fn pop_text(&mut self);
    // Assume internal representation is a String.
    fn take_text(&mut self) -> String;
    // Assume internal representation is a String and we'll simply replace it with
    // text. Into<String> may also work.
    fn replace_text(&mut self, text: String);
    fn is_text_handling(&self) -> bool;
    fn handle_text_entry(&mut self, key_event: KeyEvent) -> bool {
        if !self.is_text_handling() {
            return false;
        }
        // The only accepted modifier is shift - if pressing another set of modifiers,
        // we won't handle it. Somewhere else should instead.
        if !key_event.modifiers.is_empty() && key_event.modifiers != KeyModifiers::SHIFT {
            return false;
        }
        match key_event.code {
            KeyCode::Char(c) => {
                self.push_text(c);
                true
            }
            KeyCode::Backspace => {
                self.pop_text();
                true
            }
            _ => false,
        }
    }
}
// A text handler that can receive suggestions
// TODO: Seperate library and binary APIs
pub trait Suggestable: TextHandler {
    fn get_search_suggestions(&self) -> &[SearchSuggestion];
    fn has_search_suggestions(&self) -> bool;
}
/// A component of the application that handles actions.
/// Where an action is a message specifically sent to the component.
/// Consider if this should be inside ActionProcessor
pub trait ActionHandler<A: Action + Clone> {
    async fn handle_action(&mut self, action: &A);
}

pub trait MouseHandler {
    /// Not implemented yet!
    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        unimplemented!()
    }
}

/// The action to do after handling a key event
pub enum KeyHandleAction<A: Action> {
    Action(A),
    Mode,
    NoMap,
}
/// The action from handling a key event (no Action type required)
pub enum KeyHandleOutcome {
    Action,
    Mode,
    NoMap,
}
/// Return a list of the current keymap for the provided stack of key_codes.
/// Note, if multiple options are available returns the first one.
pub fn get_key_subset<'a, A: Action>(
    binds: Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a>,
    key_stack: &[KeyEvent],
) -> Option<&'a Keymap<A>> {
    let first = index_keybinds(binds, key_stack.first()?)?;
    index_keymap(first, key_stack.get(1..)?)
}
/// Check if key stack will result in an action for binds.
// Requires returning an action type so can be awkward.
pub fn handle_key_stack<'a, A>(
    binds: Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a>,
    key_stack: Vec<KeyEvent>,
) -> KeyHandleAction<A>
where
    A: Action + Clone,
{
    if let Some(subset) = get_key_subset(binds, &key_stack) {
        match &subset {
            Keymap::Action(a) => {
                // As Action is simply a message that is being passed around
                // I am comfortable to clone it. Receiver should own the message.
                // We may be able to improve on this using GATs or reference counting.
                return KeyHandleAction::Action(a.clone());
            }
            Keymap::Mode(_) => return KeyHandleAction::Mode,
        }
    }
    KeyHandleAction::NoMap
}
/// Try to handle the passed key_stack if it processes an action.
/// Returns if it was handled or why it was not.
// Doesn't require returning an Action type.
pub async fn handle_key_stack_and_action<'a, A, B>(
    handler: &mut B,
    key_stack: Vec<KeyEvent>,
) -> KeyHandleOutcome
where
    A: Action + Clone,
    B: KeyRouter<A> + ActionHandler<A>,
{
    if let Some(subset) = get_key_subset(handler.get_routed_keybinds(), &key_stack) {
        match &subset {
            Keymap::Action(a) => {
                // As Action is simply a message that is being passed around
                // I am comfortable to clone it. Receiver should own the message.
                // We may be able to improve on this using GATs or reference counting.
                handler.handle_action(&a.clone()).await;
                return KeyHandleOutcome::Action;
            }
            Keymap::Mode(_) => return KeyHandleOutcome::Mode,
        }
    }
    KeyHandleOutcome::NoMap
}
/// If a list of Keybinds contains a binding for the index KeyEvent, return that
/// KeyEvent.
pub fn index_keybinds<'a, A: Action>(
    binds: Box<dyn Iterator<Item = &'a KeyCommand<A>> + 'a>,
    index: &KeyEvent,
) -> Option<&'a Keymap<A>> {
    let mut binds = binds;
    binds
        .find(|kb| kb.contains_keyevent(index))
        .map(|kb| &kb.key_map)
}
/// Recursively indexes into a Keymap using a list of KeyEvents. Yields the
/// presented Keymap,
//  or none if one of the indexes fails to return a value.
pub fn index_keymap<'a, A: Action>(
    map: &'a Keymap<A>,
    indexes: &[KeyEvent],
) -> Option<&'a Keymap<A>> {
    indexes
        .iter()
        .try_fold(map, move |target, i| match &target {
            Keymap::Action(_) => None,
            Keymap::Mode(m) => index_keybinds(Box::new(m.commands.iter()), i),
        })
}
#[cfg(test)]
mod tests {
    #![allow(clippy::todo)]
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{
        component::actionhandler::{index_keybinds, Keymap},
        keycommand::Mode,
    };

    use super::{index_keymap, Action, KeyCommand};

    #[derive(PartialEq, Debug)]
    enum TestAction {
        Test1,
        Test2,
        Test3,
        TestStack,
    }
    impl Action for TestAction {
        fn context(&self) -> std::borrow::Cow<str> {
            todo!()
        }

        fn describe(&self) -> std::borrow::Cow<str> {
            todo!()
        }
    }
    #[test]
    fn test_key_stack_shift_modifier() {
        let kb = vec![
            KeyCommand::new_from_code(KeyCode::F(10), TestAction::Test1),
            KeyCommand::new_from_code(KeyCode::F(12), TestAction::Test2),
            KeyCommand::new_from_code(KeyCode::Left, TestAction::Test3),
            KeyCommand::new_from_code(KeyCode::Right, TestAction::Test3),
            KeyCommand::new_action_only_mode(
                vec![
                    (KeyCode::Enter, TestAction::Test2),
                    (KeyCode::Char('a'), TestAction::Test3),
                    (KeyCode::Char('p'), TestAction::Test2),
                    (KeyCode::Char(' '), TestAction::Test3),
                    (KeyCode::Char('P'), TestAction::Test2),
                    (KeyCode::Char('A'), TestAction::TestStack),
                ],
                KeyCode::Enter,
                "Play",
            ),
        ];
        let ks1 = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let ks2 = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        let key_stack = [ks1, ks2];
        let first = index_keybinds(Box::new(kb.iter()), key_stack.first().unwrap()).unwrap();
        let act = index_keymap(first, key_stack.get(1..).unwrap());
        let Some(Keymap::Action(a)) = act else {
            panic!();
        };
        assert_eq!(*a, TestAction::TestStack);
    }
    #[test]
    fn test_key_stack() {
        let kb = vec![
            KeyCommand::new_from_code(KeyCode::F(10), TestAction::Test1),
            KeyCommand::new_from_code(KeyCode::F(12), TestAction::Test2),
            KeyCommand::new_from_code(KeyCode::Left, TestAction::Test3),
            KeyCommand::new_from_code(KeyCode::Right, TestAction::Test3),
            KeyCommand::new_action_only_mode(
                vec![
                    (KeyCode::Enter, TestAction::Test2),
                    (KeyCode::Char('a'), TestAction::Test3),
                    (KeyCode::Char('p'), TestAction::Test2),
                    (KeyCode::Char(' '), TestAction::Test3),
                    (KeyCode::Char('P'), TestAction::Test2),
                    (KeyCode::Char('A'), TestAction::TestStack),
                ],
                KeyCode::Enter,
                "Play",
            ),
        ];
        let ks1 = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let ks2 = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty());
        let key_stack = [ks1, ks2];
        let first = index_keybinds(Box::new(kb.iter()), key_stack.first().unwrap()).unwrap();
        let act = index_keymap(first, key_stack.get(1..).unwrap());
        let Some(Keymap::Action(a)) = act else {
            panic!();
        };
        assert_eq!(*a, TestAction::TestStack);
    }
    #[test]
    fn test_index_keybinds() {
        let kb = vec![
            KeyCommand::new_from_code(KeyCode::F(10), TestAction::Test1),
            KeyCommand::new_from_code(KeyCode::F(12), TestAction::Test2),
            KeyCommand::new_from_code(KeyCode::Left, TestAction::Test3),
            KeyCommand::new_from_code(KeyCode::Right, TestAction::Test3),
            KeyCommand::new_action_only_mode(
                vec![
                    (KeyCode::Char('A'), TestAction::Test2),
                    (KeyCode::Char('a'), TestAction::Test3),
                ],
                KeyCode::Enter,
                "Play",
            ),
        ];
        let ks = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let idx = index_keybinds(Box::new(kb.iter()), &ks);
        let eq = KeyCommand::new_action_only_mode(
            vec![
                (KeyCode::Char('A'), TestAction::Test2),
                (KeyCode::Char('a'), TestAction::Test3),
            ],
            KeyCode::Enter,
            "Play",
        )
        .key_map;
        assert_eq!(idx, Some(&eq));
    }
    #[test]
    fn test_index_keymap() {
        let kb = Keymap::Mode(Mode {
            commands: vec![
                KeyCommand::new_from_code(KeyCode::F(10), TestAction::Test1),
                KeyCommand::new_from_code(KeyCode::F(12), TestAction::Test2),
                KeyCommand::new_from_code(KeyCode::Left, TestAction::Test3),
                KeyCommand::new_from_code(KeyCode::Right, TestAction::Test3),
                KeyCommand::new_action_only_mode(
                    vec![
                        (KeyCode::Char('A'), TestAction::Test2),
                        (KeyCode::Char('a'), TestAction::Test3),
                    ],
                    KeyCode::Enter,
                    "Play",
                ),
            ],
            name: "test",
        });
        let ks = [KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())];
        let idx = index_keymap(&kb, &ks);
        let eq = KeyCommand::new_action_only_mode(
            vec![
                (KeyCode::Char('A'), TestAction::Test2),
                (KeyCode::Char('a'), TestAction::Test3),
            ],
            KeyCode::Enter,
            "Play",
        )
        .key_map;
        assert_eq!(idx, Some(&eq));
    }
}
