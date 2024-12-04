use crate::{
    config::keymap::{KeyActionTree, Keymap},
    keyaction::{DisplayableKeyAction, KeyAction, KeyActionVisibility},
    keybind::Keybind,
};
use async_callback_manager::AsyncTask;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::borrow::Cow;
use tracing::warn;
use ytmapi_rs::common::SearchSuggestion;

/// Convenience type alias
pub type ComponentEffect<C> = AsyncTask<C, <C as Component>::Bkend, <C as Component>::Md>;
/// A frontend component - has an associated backend and task metadata type.
pub trait Component {
    type Bkend;
    type Md;
}
/// Macro to generate the boilerplate implementation of Component used in this
/// app.
macro_rules! impl_youtui_component {
    ($t:ty) => {
        impl crate::app::component::actionhandler::Component for $t {
            type Bkend = ArcServer;
            type Md = TaskMetadata;
        }
    };
}

/// A component that can handle actions.
pub trait ActionHandler<A: Action>: Component + Sized {
    async fn apply_action(&mut self, action: A) -> ComponentEffect<Self>;
    /// Apply an action that can be mapped to Self.
    async fn apply_action_mapped<B, C, F>(&mut self, action: B, f: F) -> ComponentEffect<Self>
    where
        B: Action,
        C: Component<Bkend = Self::Bkend, Md = Self::Md> + ActionHandler<B> + 'static,
        F: Fn(&mut Self) -> &mut C + Send + Clone + 'static,
        Self::Bkend: 'static,
        Self::Md: 'static,
    {
        f(self)
            .apply_action(action)
            .await
            .map(move |this: &mut Self| f(this))
    }
}

/// An action that can be applied to state.
pub trait Action {
    type State: Component;
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
pub trait KeyRouter<A: Action + 'static> {
    /// Get the list of active keybinds that the component and its route
    /// contain.
    fn get_active_keybinds(&self) -> impl Iterator<Item = &Keymap<A>>;
    /// Get the list of keybinds that the component and any child items can
    /// contain, regardless of current route.
    fn get_all_keybinds(&self) -> impl Iterator<Item = &Keymap<A>>;
}

/// A component of the application that can block parent keybinds.
/// For example, a component that can display a modal dialog that will prevent
/// other inputs.
pub trait DominantKeyRouter<A: Action + 'static> {
    /// Return true if dominant keybinds are active.
    fn dominant_keybinds_active(&self) -> bool;
    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &Keymap<A>>;
}

/// Get the list of all keybinds that the KeyHandler and any child items can
/// contain, regardless of context.
pub fn get_all_visible_keybinds_as_readable_iter<K: KeyRouter<A>, A: Action + 'static>(
    component: &K,
) -> impl Iterator<Item = DisplayableKeyAction<'_>> + '_ {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.iter())
        .filter(|(_, kt)| (*kt).get_visibility() != KeyActionVisibility::Hidden)
        .map(|(kb, kt)| DisplayableKeyAction::from_keybind_and_action_tree(kb, kt))
}
/// Get a context-specific list of all keybinds marked global.
pub fn get_active_global_keybinds_as_readable_iter<K: KeyRouter<A>, A: Action + 'static>(
    component: &K,
) -> impl Iterator<Item = DisplayableKeyAction<'_>> + '_ {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.iter())
        .filter(|(_, kt)| (*kt).get_visibility() == KeyActionVisibility::Global)
        .map(|(kb, kt)| DisplayableKeyAction::from_keybind_and_action_tree(kb, kt))
}
/// Count the number of visible keybinds - helper for Help menu.
pub fn count_visible_keybinds<K: KeyRouter<A>, A: Action + 'static>(component: &K) -> usize {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.iter())
        .filter(|(_, kt)| (*kt).get_visibility() != KeyActionVisibility::Hidden)
        .count()
}
/// A component of the application that handles text entry, currently designed
/// to wrap rat_text::TextInputState.
pub trait TextHandler: Component {
    /// Get a reference to the text.
    fn get_text(&self) -> &str;
    /// Clear text, returning false if it was already clear.
    fn clear_text(&mut self) -> bool;
    /// Replace all text
    fn replace_text(&mut self, text: impl Into<String>);
    /// Text handling could be a subset of the component. Return true if the
    /// text handling subset is active.
    fn is_text_handling(&self) -> bool;
    /// Handle a crossterm event, returning a task if an event was handled.
    fn handle_text_event_impl(
        &mut self,
        event: &Event,
    ) -> Option<AsyncTask<Self, Self::Bkend, Self::Md>>
    where
        Self: Sized;
    /// Default behaviour is to only handle an event if is_text_handling() ==
    /// true.
    fn try_handle_text(&mut self, event: &Event) -> Option<AsyncTask<Self, Self::Bkend, Self::Md>>
    where
        Self: Sized,
    {
        if !self.is_text_handling() {
            return None;
        }
        self.handle_text_event_impl(event)
    }
}
// A text handler that can receive suggestions
// TODO: Seperate library and binary APIs
pub trait Suggestable: TextHandler {
    fn get_search_suggestions(&self) -> &[SearchSuggestion];
    fn has_search_suggestions(&self) -> bool;
}

pub trait MouseHandler {
    /// Not implemented yet!
    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        unimplemented!()
    }
}

/// The action to do after handling a key event
#[derive(Debug)]
pub enum KeyHandleAction<'a, A: Action> {
    Action(A),
    Mode { name: String, keys: &'a Keymap<A> },
    NoMap,
}

/// Check the current stack of keys, to see if an action is produced, a mode is
/// produced, or nothing produced.
pub fn handle_key_stack<'a, A, I>(keys: I, key_stack: &[KeyEvent]) -> KeyHandleAction<'a, A>
where
    A: Action + Copy + 'static,
    I: IntoIterator<Item = &'a Keymap<A>>,
{
    let convert = |k: KeyEvent| {
        // NOTE: kind and state fields currently unused.
        let KeyEvent {
            code,
            mut modifiers,
            ..
        } = k;
        // If the keycode is a character, then the shift modifier should be removed. It
        // will be encoded in the character already. This same stripping occurs when
        // parsing the keycode in Keybind::from_str(..).
        if let KeyCode::Char(_) = code {
            modifiers = modifiers.difference(KeyModifiers::SHIFT);
        }
        Keybind { code, modifiers }
    };
    let mut key_stack_iter = key_stack.iter();
    // First iteration - iterator of hashmaps.
    let Some(first_key) = key_stack_iter.next() else {
        return KeyHandleAction::NoMap;
    };
    let first_found = keys.into_iter().find_map(|km| km.get(&convert(*first_key)));
    let mut next_mode = match first_found {
        Some(KeyActionTree::Key(KeyAction { action, value, .. })) => {
            if let Some(v) = value {
                warn!("Keybind had value {v}, currently unhandled");
            }
            return KeyHandleAction::Action(*action);
        }
        Some(KeyActionTree::Mode { name, keys }) => (name, keys),
        None => return KeyHandleAction::NoMap,
    };
    for key in key_stack_iter {
        let next_found = next_mode.1.get(&convert(*key));
        match next_found {
            Some(KeyActionTree::Key(KeyAction { action, value, .. })) => {
                if let Some(v) = value {
                    warn!("Keybind had value {v}, currently unhandled");
                }
                return KeyHandleAction::Action(*action);
            }
            Some(KeyActionTree::Mode { name, keys }) => next_mode = (name, keys),
            None => return KeyHandleAction::NoMap,
        };
    }
    KeyHandleAction::Mode {
        name: next_mode.0.as_deref().unwrap_or("UNNAMED MODE").to_string(),
        keys: next_mode.1,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::todo)]
    use super::{Action, Component};
    use crate::{
        app::component::actionhandler::{handle_key_stack, KeyHandleAction, Keymap},
        config::keymap::KeyActionTree,
        keybind::Keybind,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use pretty_assertions::assert_eq;

    #[derive(PartialEq, Debug, Copy, Clone)]
    enum TestAction {
        Test1,
        Test2,
        Test3,
        TestStack,
    }
    impl Component for () {
        type Bkend = ();
        type Md = ();
    }
    impl Action for TestAction {
        fn context(&self) -> std::borrow::Cow<str> {
            todo!()
        }
        fn describe(&self) -> std::borrow::Cow<str> {
            todo!()
        }
        type State = ();
    }
    fn test_keymap() -> Keymap<TestAction> {
        [
            (
                Keybind::new_unmodified(KeyCode::F(10)),
                KeyActionTree::new_key_defaulted(TestAction::Test1),
            ),
            (
                Keybind::new_unmodified(KeyCode::F(12)),
                KeyActionTree::new_key_defaulted(TestAction::Test2),
            ),
            (
                Keybind::new_unmodified(KeyCode::Left),
                KeyActionTree::new_key_defaulted(TestAction::Test3),
            ),
            (
                Keybind::new_unmodified(KeyCode::Right),
                KeyActionTree::new_key_defaulted(TestAction::Test3),
            ),
            (
                Keybind::new_unmodified(KeyCode::Enter),
                KeyActionTree::new_mode(
                    [
                        (
                            Keybind::new_unmodified(KeyCode::Enter),
                            KeyActionTree::new_key_defaulted(TestAction::Test2),
                        ),
                        (
                            Keybind::new_unmodified(KeyCode::Char('a')),
                            KeyActionTree::new_key_defaulted(TestAction::Test3),
                        ),
                        (
                            Keybind::new_unmodified(KeyCode::Char('p')),
                            KeyActionTree::new_key_defaulted(TestAction::Test2),
                        ),
                        (
                            Keybind::new_unmodified(KeyCode::Char(' ')),
                            KeyActionTree::new_key_defaulted(TestAction::Test3),
                        ),
                        (
                            Keybind::new_unmodified(KeyCode::Char('P')),
                            KeyActionTree::new_key_defaulted(TestAction::Test2),
                        ),
                        (
                            Keybind::new_unmodified(KeyCode::Char('A')),
                            KeyActionTree::new_key_defaulted(TestAction::TestStack),
                        ),
                    ],
                    "Play".into(),
                ),
            ),
        ]
        .into_iter()
        .collect::<Keymap<_>>()
    }
    #[test]
    fn test_key_stack_shift_modifier() {
        let kb = test_keymap();
        let ks1 = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let ks2 = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        let key_stack = [ks1, ks2];
        let expected = TestAction::TestStack;
        let output = handle_key_stack(std::iter::once(&kb), &key_stack);
        println!("{:?}", output);
        let KeyHandleAction::Action(output) = output else {
            panic!("Expected keyhandleoutcome::action");
        };
        assert_eq!(expected, output);
    }
    #[test]
    fn test_key_stack() {
        let kb = test_keymap();
        let ks1 = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let ks2 = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty());
        let key_stack = [ks1, ks2];
        let expected = TestAction::TestStack;
        let KeyHandleAction::Action(output) = handle_key_stack(std::iter::once(&kb), &key_stack)
        else {
            panic!("Expected keyhandleoutcome::action");
        };
        assert_eq!(expected, output);
    }
    #[test]
    fn test_index_keybinds() {
        let kb = test_keymap();
        let ks = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let expected_keys = [
            (
                Keybind::new_unmodified(KeyCode::Enter),
                KeyActionTree::new_key_defaulted(TestAction::Test2),
            ),
            (
                Keybind::new_unmodified(KeyCode::Char('a')),
                KeyActionTree::new_key_defaulted(TestAction::Test3),
            ),
            (
                Keybind::new_unmodified(KeyCode::Char('p')),
                KeyActionTree::new_key_defaulted(TestAction::Test2),
            ),
            (
                Keybind::new_unmodified(KeyCode::Char(' ')),
                KeyActionTree::new_key_defaulted(TestAction::Test3),
            ),
            (
                Keybind::new_unmodified(KeyCode::Char('P')),
                KeyActionTree::new_key_defaulted(TestAction::Test2),
            ),
            (
                Keybind::new_unmodified(KeyCode::Char('A')),
                KeyActionTree::new_key_defaulted(TestAction::TestStack),
            ),
        ]
        .into_iter()
        .collect::<Keymap<_>>();
        let expected_name = "Play".to_string();
        let KeyHandleAction::Mode { keys, name } = handle_key_stack(std::iter::once(&kb), &[ks])
        else {
            panic!("Expected keyhandleoutcome::mode");
        };
        assert_eq!(name, expected_name);
        assert_eq!(keys, &expected_keys);
    }
}
