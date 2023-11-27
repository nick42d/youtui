use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::{borrow::Cow, fmt::Display};
use ytmapi_rs::common::SearchSuggestion;

// An action that can be sent to a component.
pub trait Action {
    fn context(&self) -> Cow<str>;
    fn describe(&self) -> Cow<str>;
}
#[derive(PartialEq, Debug, Clone)]
// Should another type be GlobalHidden?
pub enum KeybindVisibility {
    Hidden,
    Global,
}
#[derive(PartialEq, Debug, Clone)]
pub enum Keymap<A: Action> {
    Action(A),
    Mode(Mode<A>),
}
#[derive(PartialEq, Debug, Clone)]
pub struct Mode<A: Action> {
    pub name: &'static str,
    pub key_binds: Vec<Keybind<A>>,
}
#[derive(PartialEq, Debug, Clone)]
pub struct Keybind<A: Action> {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub key_map: Keymap<A>,
    pub visibility: KeybindVisibility,
}

impl<A: Action> Display for Keybind<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code: Cow<str> = match self.code {
            KeyCode::Enter => "Enter".into(),
            KeyCode::Left => "Left".into(),
            KeyCode::Right => "Right".into(),
            KeyCode::Up => "Up".into(),
            KeyCode::Down => "Down".into(),
            KeyCode::PageUp => "PageUp".into(),
            KeyCode::PageDown => "PageDown".into(),
            KeyCode::Esc => "Esc".into(),
            KeyCode::Char(c) => match c {
                ' ' => "Space".into(),
                c => c.to_string().into(),
            },
            KeyCode::F(x) => format!("F{x}").into(),
            _ => "".into(),
        };
        match self.modifiers {
            KeyModifiers::CONTROL => write!(f, "C-{code}"),
            _ => write!(f, "{code}"),
        }
    }
}

// Is this an implementation of Action?
impl<A: Action> Mode<A> {
    pub fn context(&self) -> Cow<str> {
        self.key_binds
            .get(0)
            .map(|kb| kb.context())
            .unwrap_or_default()
    }
    pub fn describe(&self) -> Cow<str> {
        self.name.into()
    }
    pub fn as_readable_short_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>)> + 'a> {
        Box::new(self.key_binds.iter().map(|bind| bind.as_readable_short()))
    }
    pub fn as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a> {
        Box::new(self.key_binds.iter().map(|bind| bind.as_readable()))
    }
}

impl<A: Action> Keybind<A> {
    // Is this an implementation of Action?
    pub fn context(&self) -> Cow<str> {
        match &self.key_map {
            Keymap::Action(a) => a.context(),
            Keymap::Mode(m) => m.context(),
        }
    }
    pub fn describe(&self) -> Cow<str> {
        match &self.key_map {
            Keymap::Action(a) => a.describe(),
            Keymap::Mode(m) => m.describe(),
        }
    }
    pub fn as_readable_short(&self) -> (Cow<str>, Cow<str>) {
        (self.to_string().into(), self.describe())
    }
    pub fn as_readable(&self) -> (Cow<str>, Cow<str>, Cow<str>) {
        // XXX: Do we also want to display sub-keys in Modes?
        (self.to_string().into(), self.context(), self.describe())
    }
    fn contains_keyevent(&self, keyevent: &KeyEvent) -> bool {
        match self.code {
            // If key code is a character it may have shift pressed, if that's the case ignore the shift
            // As may have been used to capitalise the letter, which will already be counted in the key code.
            KeyCode::Char(_) => {
                self.code == keyevent.code
                    && self.modifiers.union(KeyModifiers::SHIFT)
                        == keyevent.modifiers.union(KeyModifiers::SHIFT)
            }
            _ => self.code == keyevent.code && self.modifiers == keyevent.modifiers,
        }
    }
    pub fn new_from_code(code: KeyCode, action: A) -> Keybind<A> {
        Keybind {
            code,
            modifiers: KeyModifiers::empty(),
            key_map: Keymap::Action(action),
            visibility: KeybindVisibility::Hidden,
        }
    }
    pub fn new_modified_from_code(code: KeyCode, modifiers: KeyModifiers, action: A) -> Keybind<A> {
        Keybind {
            code,
            modifiers,
            key_map: Keymap::Action(action),
            visibility: KeybindVisibility::Hidden,
        }
    }
    pub fn new_global_from_code(code: KeyCode, action: A) -> Keybind<A> {
        Keybind {
            code,
            modifiers: KeyModifiers::empty(),
            key_map: Keymap::Action(action),
            visibility: KeybindVisibility::Global,
        }
    }
    pub fn new_action_only_mode(
        actions: Vec<(KeyCode, A)>,
        code: KeyCode,
        name: &'static str,
    ) -> Keybind<A> {
        let key_binds = actions
            .into_iter()
            .map(|(code, action)| Keybind {
                code,
                modifiers: KeyModifiers::empty(),
                key_map: Keymap::Action(action),
                visibility: KeybindVisibility::Hidden,
            })
            .collect();
        Keybind {
            code,
            modifiers: KeyModifiers::empty(),
            key_map: Keymap::Mode(Mode { key_binds, name }),
            visibility: KeybindVisibility::Hidden,
        }
    }
}
/// A component of the application that has its own set of keybinds when focussed.
pub trait KeyHandler<A: Action> {
    /// Get the list of keybinds that are active for the KeyHandler.
    // XXX: This doesn't work recursively as children could contain different Action types.
    // Consider a different approach.
    fn get_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<A>> + 'a>;
}
/// A component of the application that has different keybinds depending on what is focussed.
/// For example, keybinds for browser may differ depending on selected pane.
/// Not every KeyHandler is a KeyRouter - e.g the individual panes themselves.
// Could possibly be a part of EventHandler instead.
pub trait KeyRouter<A: Action>: KeyHandler<A> {
    // Get the list of keybinds that the KeyHandler and any child items can contain.
    fn get_all_keybinds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Keybind<A>> + 'a>;
}
/// A component of the application that has different keybinds depending on what is focussed.
/// For example, keybinds for browser may differ depending on selected pane.
/// Not every KeyHandler is a DisplayableKeyRouter - e.g the individual panes themselves.
// Could possibly be a part of EventHandler instead.
pub trait DisplayableKeyRouter {
    // Get the list of keybinds that the KeyHandler and any child items can contain.
    fn get_all_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a>;
    fn get_all_global_keybinds_as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a>;
}
/// A component of the application that handles actions.
/// Where an action is a message specifically sent to the component.
pub trait ActionHandler<A: Action + Clone> {
    async fn handle_action(&mut self, action: &A);
}
/// A component of the application that handles text entry.
// TODO: Cursor position and movement.
pub trait TextHandler {
    // TODO: cursor manipulation
    fn push_text(&mut self, c: char);
    fn pop_text(&mut self);
    // Assume internal representation is a String.
    fn take_text(&mut self) -> String;
    // Assume internal representation is a String and we'll simply replace it with text.
    // Into<String> may also work.
    fn replace_text(&mut self, text: String);
    fn is_text_handling(&self) -> bool;
    fn handle_text_entry(&mut self, key_event: KeyEvent) -> bool {
        if !self.is_text_handling() {
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
// A next handler that can receive suggestions
// TODO: Seperate library and binary APIs
pub trait Suggestable: TextHandler {
    fn get_search_suggestions(&self) -> &[SearchSuggestion];
    fn has_search_suggestions(&self) -> bool;
}
pub enum KeyHandleOutcome {
    ActionHandled,
    Mode,
    NoMap,
}
/// A component of the application that can check if an action has occurred and act on it.
// XXX: Not fully implemented yet, as ignores many event types by default e.g text..
pub trait ActionProcessor<A: Action + Clone>: ActionHandler<A> + KeyHandler<A> {
    /// Return a list of the current keymap for the provided stack of key_codes.
    /// Note, if multiple options are available returns the first one.
    fn get_key_subset(&self, key_stack: &[KeyEvent]) -> Option<&Keymap<A>> {
        let first = index_keybinds(self.get_keybinds(), key_stack.get(0)?)?;
        index_keymap(first, key_stack.get(1..)?)
    }
    /// Try to handle the passed key_stack if it processes an action.
    /// Returns if it was handle or why not. to see if an action would be taken.
    /// If an action was taken, return true.
    async fn handle_key_stack(&mut self, key_stack: Vec<KeyEvent>) -> KeyHandleOutcome {
        if let Some(subset) = self.get_key_subset(&*key_stack) {
            match &subset {
                Keymap::Action(a) => {
                    // As Action is simply a message that is being passed around
                    // I am comfortable to clone it. Receiver should own the message.
                    // We may be able to improve on this using GATs or reference counting.
                    self.handle_action(&a.clone()).await;
                    return KeyHandleOutcome::ActionHandled;
                }
                Keymap::Mode(_) => return KeyHandleOutcome::Mode,
            }
        }
        KeyHandleOutcome::NoMap
    }
}

pub trait MouseHandler {
    /// Not implemented yet!
    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        unimplemented!()
    }
}

/// If a list of Keybinds contains a binding for the index KeyEvent, return that KeyEvent.
pub fn index_keybinds<'a, A: Action>(
    binds: Box<dyn Iterator<Item = &'a Keybind<A>> + 'a>,
    index: &KeyEvent,
) -> Option<&'a Keymap<A>> {
    let mut binds = binds;
    binds
        .find(|kb| kb.contains_keyevent(index))
        .map(|kb| &kb.key_map)
}
/// Recursively indexes into a Keymap using a list of KeyEvents. Yields the presented Keymap
// , or none if one of the indexes fails to return a value.
pub fn index_keymap<'a, A: Action>(
    map: &'a Keymap<A>,
    indexes: &[KeyEvent],
) -> Option<&'a Keymap<A>> {
    indexes
        .iter()
        .try_fold(map, move |target, i| match &target {
            Keymap::Action(_) => None,
            Keymap::Mode(m) => index_keybinds(Box::new(m.key_binds.iter()), i),
        })
}
#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::component::actionhandler::{index_keybinds, Keymap, Mode};

    use super::{index_keymap, Action, Keybind};

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
            Keybind::new_from_code(KeyCode::F(10), TestAction::Test1),
            Keybind::new_from_code(KeyCode::F(12), TestAction::Test2),
            Keybind::new_from_code(KeyCode::Left, TestAction::Test3),
            Keybind::new_from_code(KeyCode::Right, TestAction::Test3),
            Keybind::new_action_only_mode(
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
        let key_stack = vec![ks1, ks2];
        let first = index_keybinds(Box::new(kb.iter()), key_stack.get(0).unwrap()).unwrap();
        let act = index_keymap(first, key_stack.get(1..).unwrap());
        let Some(Keymap::Action(a)) = act else {
            panic!();
        };
        assert_eq!(*a, TestAction::TestStack);
    }
    #[test]
    fn test_key_stack() {
        let kb = vec![
            Keybind::new_from_code(KeyCode::F(10), TestAction::Test1),
            Keybind::new_from_code(KeyCode::F(12), TestAction::Test2),
            Keybind::new_from_code(KeyCode::Left, TestAction::Test3),
            Keybind::new_from_code(KeyCode::Right, TestAction::Test3),
            Keybind::new_action_only_mode(
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
        let key_stack = vec![ks1, ks2];
        let first = index_keybinds(Box::new(kb.iter()), key_stack.get(0).unwrap()).unwrap();
        let act = index_keymap(first, key_stack.get(1..).unwrap());
        let Some(Keymap::Action(a)) = act else {
            panic!();
        };
        assert_eq!(*a, TestAction::TestStack);
    }
    #[test]
    fn test_index_keybinds() {
        let kb = vec![
            Keybind::new_from_code(KeyCode::F(10), TestAction::Test1),
            Keybind::new_from_code(KeyCode::F(12), TestAction::Test2),
            Keybind::new_from_code(KeyCode::Left, TestAction::Test3),
            Keybind::new_from_code(KeyCode::Right, TestAction::Test3),
            Keybind::new_action_only_mode(
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
        let eq = Keybind::new_action_only_mode(
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
            key_binds: vec![
                Keybind::new_from_code(KeyCode::F(10), TestAction::Test1),
                Keybind::new_from_code(KeyCode::F(12), TestAction::Test2),
                Keybind::new_from_code(KeyCode::Left, TestAction::Test3),
                Keybind::new_from_code(KeyCode::Right, TestAction::Test3),
                Keybind::new_action_only_mode(
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
        let eq = Keybind::new_action_only_mode(
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
