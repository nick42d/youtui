use crate::{
    app::component::actionhandler::Action, config::keymap::KeyActionTree, keybind::Keybind,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// This is an Action that will be triggered when pressing a particular Keybind.
pub struct KeyAction<A> {
    // Consider - can there be multiple actions?
    // Consider - can an action access global commands? Or commands from another component?
    pub action: A,
    #[serde(default)]
    pub value: Option<usize>,
    #[serde(default)]
    pub visibility: KeyActionVisibility,
}

#[derive(PartialEq, Copy, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
/// Visibility of a KeyAction.
pub enum KeyActionVisibility {
    /// Displayed on help menu
    #[default]
    Standard,
    /// Displayed on Header and help menu
    Global,
    /// Not displayed
    Hidden,
}

#[derive(PartialEq, Debug, Clone)]
/// Type-erased keybinding for displaying.
pub struct DisplayableKeyAction<'a> {
    // XXX: Do we also want to display sub-keys in Modes?
    pub keybinds: Cow<'a, str>,
    pub context: Cow<'a, str>,
    pub description: Cow<'a, str>,
}
/// Type-erased mode for displaying its actions.
pub struct DisplayableMode<'a, I: Iterator<Item = DisplayableKeyAction<'a>>> {
    pub displayable_commands: I,
    pub description: Cow<'a, str>,
}

impl<'a> DisplayableKeyAction<'a> {
    pub fn from_keybind_and_action_tree<A: Action + 'a>(
        key: &'a Keybind,
        value: &'a KeyActionTree<A>,
    ) -> Self {
        // NOTE: Currently, sub-keys of modes are not displayed.
        match value {
            KeyActionTree::Key(k) => DisplayableKeyAction {
                keybinds: key.to_string().into(),
                context: k.action.context(),
                description: k.action.describe(),
            },
            KeyActionTree::Mode { ref mut name, keys } => DisplayableKeyAction {
                keybinds: key.to_string().into(),
                context: keys.iter().next().unwrap_or_default().1.get_context(),
                description: name
                    .as_ref()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| key.to_string())
                    .into(),
            },
        }
    }
}
