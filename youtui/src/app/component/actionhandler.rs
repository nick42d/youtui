use crate::{
    app::keycommand::{CommandVisibility, DisplayableCommand, KeyCommand, Keybind},
    config::keybinds::{KeyAction, KeyActionTree},
};
use async_callback_manager::AsyncTask;
use crossterm::event::{Event, KeyEvent, MouseEvent};
use rodio::cpal::FromSample;
use std::{borrow::Cow, collections::HashMap, marker::PhantomData};
use tracing::warn;
use ytmapi_rs::common::SearchSuggestion;

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
pub struct Map<A, F, N> {
    action: A,
    f: F,
    p: PhantomData<N>,
}
impl<A, F, N> Action for Map<A, F, N>
where
    A: Action,
    F: Fn(&mut N) -> &mut A::State + Clone + Send + 'static,
    N: Component<Bkend = <A::State as Component>::Bkend, Md = <A::State as Component>::Md>,
    <A::State as Component>::Bkend: 'static,
    <A::State as Component>::Md: 'static,
    A::State: 'static,
{
    type State = N;
    fn context(&self) -> Cow<str> {
        self.action.context()
    }
    fn describe(&self) -> Cow<str> {
        self.action.describe()
    }
    async fn apply(self, state: &mut Self::State) -> ComponentEffect<Self::State>
    where
        Self: Sized,
    {
        self.action.apply((self.f)(state)).await.map(self.f)
    }
}

/// An action that can be applied to state.
pub trait Action {
    type State: Component;
    fn context(&self) -> Cow<str>;
    fn describe(&self) -> Cow<str>;
    async fn apply(self, state: &mut Self::State) -> ComponentEffect<Self::State>
    where
        Self: Sized;
    fn map<N, F>(self, f: F) -> Map<Self, F, N>
    where
        F: Fn(&mut N) -> &mut Self::State,
        Self: Sized,
    {
        Map {
            action: self,
            f,
            p: PhantomData,
        }
    }
}
pub type Keymap<A> = HashMap<Keybind, KeyActionTree<A>>;
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
    fn get_active_keybinds(&self) -> impl Iterator<Item = &'_ Keymap<A>> + '_;
    /// Get the list of keybinds that the component and any child items can
    /// contain, regardless of current route.
    fn get_all_keybinds(&self) -> impl Iterator<Item = &'_ Keymap<A>> + '_;
}

/// A component of the application that can block parent keybinds.
/// For example, a component that can display a modal dialog that will prevent
/// other inputs.
pub trait DominantKeyRouter<A: Action + 'static> {
    /// Return true if dominant keybinds are active.
    fn dominant_keybinds_active(&self) -> bool;
    fn get_dominant_keybinds(&self) -> impl Iterator<Item = &'_ Keymap<A>> + '_;
}

// XXX: Can these all just be derived from KeyRouter?
/// Get the list of all keybinds that the KeyHandler and any child items can
/// contain, regardless of context.
pub fn get_all_visible_keybinds_as_readable_iter<K: KeyRouter<A>, A: Action + 'static>(
    component: &K,
) -> impl Iterator<Item = DisplayableCommand<'_>> + '_ {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.into_iter())
        .filter(|(_, kt)| (*kt).get_visibility() != CommandVisibility::Hidden)
        .map(|(kb, kt)| DisplayableCommand::from_command(kb, kt))
}
/// Get the list of all non-hidden keybinds that the KeyHandler and any
/// child items can contain, regardless of context.
pub fn get_all_keybinds_as_readable_iter<K: KeyRouter<A>, A: Action + 'static>(
    component: &K,
) -> impl Iterator<Item = DisplayableCommand<'_>> + '_ {
    component
        .get_all_keybinds()
        .flat_map(|keymap| keymap.into_iter())
        .map(|(kb, kt)| DisplayableCommand::from_command(kb, kt))
}
/// Get a context-specific list of all keybinds marked global.
// TODO: Put under DisplayableKeyHandler
pub fn get_active_global_keybinds_as_readable_iter<K: KeyRouter<A>, A: Action + 'static>(
    component: &K,
) -> impl Iterator<Item = DisplayableCommand<'_>> + '_ {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.into_iter())
        .filter(|(_, kt)| (*kt).get_visibility() == CommandVisibility::Global)
        .map(|(kb, kt)| DisplayableCommand::from_command(kb, kt))
}
// e.g - for use in help menu.
pub fn count_visible_keybinds<K: KeyRouter<A>, A: Action + 'static>(component: &K) -> usize {
    component
        .get_active_keybinds()
        .flat_map(|keymap| keymap.into_iter())
        .filter(|(_, kt)| (*kt).get_visibility() != CommandVisibility::Hidden)
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
pub enum KeyHandleAction<A: Action> {
    Action(A),
    Mode,
    NoMap,
}
impl<Frntend, Bkend, Md> KeyHandleOutcome<Frntend, Bkend, Md>
where
    Frntend: 'static,
    Bkend: 'static,
    Md: 'static,
{
    pub fn map<NewFrntend>(
        self,
        f: impl Fn(&mut NewFrntend) -> &mut Frntend + Send + Clone + 'static,
    ) -> KeyHandleOutcome<NewFrntend, Bkend, Md> {
        match self {
            KeyHandleOutcome::Action(a) => KeyHandleOutcome::Action(a.map(f)),
            KeyHandleOutcome::Mode => KeyHandleOutcome::Mode,
            KeyHandleOutcome::NoMap => KeyHandleOutcome::NoMap,
        }
    }
}
/// The action from handling a key event (no Action type required)
pub enum KeyHandleOutcome<Frntend, Bkend, Md> {
    Action(AsyncTask<Frntend, Bkend, Md>),
    Mode,
    NoMap,
}

pub fn handle_key_stack_2<'a, A, I>(keys: I, key_stack: &[KeyEvent]) -> KeyHandleAction<A>
where
    A: Action + Copy + 'static,
    I: IntoIterator<Item = &'a Keymap<A>>,
{
    let convert = |k: KeyEvent| {
        let KeyEvent {
            code,
            modifiers,
            kind,
            state,
        } = k;
        Keybind { code, modifiers }
    };
    let mut is_mode = false;
    // let mut next_keys = None;
    let mut next_keys = Box::new(keys.into_iter()) as Box<dyn Iterator<Item = &Keymap<A>>>;
    for k in key_stack {
        let next_found = next_keys.find_map(|km| km.get(&convert(*k)));
        match next_found {
            Some(KeyActionTree::Key(KeyAction { action, value, .. })) => {
                if let Some(v) = value {
                    warn!("Keybind had value {v}, currently unhandled");
                }
                return KeyHandleAction::Action(*action);
            }
            Some(KeyActionTree::Mode { name, keys }) => {
                is_mode = true;
                // The 'Once' here is a neat hack, could be improved.
                next_keys = Box::new(std::iter::once(keys))
            }
            None => is_mode = false,
        };
    }
    if is_mode {
        return KeyHandleAction::Mode;
    }
    KeyHandleAction::NoMap
}

/// Return a list of the current keymap for the provided stack of key_codes.
/// Note, if multiple options are available returns the first one.
pub fn get_key_subset<'a, A: Action>(
    binds: impl Iterator<Item = &'a KeyCommand<A>> + 'a,
    key_stack: &[KeyEvent],
) -> Option<&'a Keymap<A>> {
    let first = index_keybinds(binds, key_stack.first()?)?;
    index_keymap(first, key_stack.get(1..)?)
}
/// Check if key stack will result in an action for binds.
// Requires returning an action type so can be awkward.
pub fn handle_key_stack<'a, A>(
    binds: impl Iterator<Item = &'a KeyCommand<A>> + 'a,
    key_stack: &[KeyEvent],
) -> KeyHandleAction<A>
where
    A: Action + Clone + 'a,
{
    if let Some(subset) = get_key_subset(binds, key_stack) {
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
) -> KeyHandleOutcome<B, B::Bkend, B::Md>
where
    A: Action<State = B> + Clone + 'static,
    B: KeyRouter<A> + Component,
{
    if let Some(subset) = get_key_subset(handler.get_active_keybinds(), &key_stack) {
        match &subset {
            Keymap::Action(a) => {
                // As Action is simply a message that is being passed around
                // I am comfortable to clone it. Receiver should own the message.
                // We may be able to improve on this using GATs or reference counting.
                let effect = a.clone().apply(handler).await;
                return KeyHandleOutcome::Action(effect);
            }
            Keymap::Mode(_) => return KeyHandleOutcome::Mode,
        }
    }
    KeyHandleOutcome::NoMap
}
/// If a list of Keybinds contains a binding for the index KeyEvent, return that
/// KeyEvent.
pub fn index_keybinds<'a, A: Action>(
    binds: impl Iterator<Item = &'a KeyCommand<A>> + 'a,
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

    use super::{index_keymap, Action, Component, KeyCommand};

    #[derive(PartialEq, Debug)]
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
        async fn apply(self, _: &mut Self::State) -> async_callback_manager::AsyncTask<(), (), ()>
        where
            Self: Sized,
        {
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
                "Play".into(),
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
                "Play".into(),
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
                "Play".into(),
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
            "Play".into(),
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
                    "Play".into(),
                ),
            ],
            name: "test".into(),
        });
        let ks = [KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())];
        let idx = index_keymap(&kb, &ks);
        let eq = KeyCommand::new_action_only_mode(
            vec![
                (KeyCode::Char('A'), TestAction::Test2),
                (KeyCode::Char('a'), TestAction::Test3),
            ],
            KeyCode::Enter,
            "Play".into(),
        )
        .key_map;
        assert_eq!(idx, Some(&eq));
    }
}
