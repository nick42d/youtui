use crate::config::keymap::KeyActionTree;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, char::ParseCharError, fmt::Display, str::FromStr};

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
/// Type-erased KeyAction
pub struct DisplayableCommand<'a> {
    // XXX: Do we also want to display sub-keys in Modes?
    pub keybinds: Cow<'a, str>,
    pub context: Cow<'a, str>,
    pub description: Cow<'a, str>,
}
pub struct DisplayableMode<'a, I: Iterator<Item = DisplayableCommand<'a>>> {
    pub displayable_commands: I,
    pub description: Cow<'a, str>,
}

impl<'a> DisplayableCommand<'a> {
    pub fn from_command<A: Action + 'a>(key: &'a Keybind, value: &'a KeyActionTree<A>) -> Self {
        // NOTE: Currently, sub-keys of modes are not displayed.
        match value {
            KeyActionTree::Key(k) => DisplayableCommand {
                keybinds: key.to_string().into(),
                context: k.action.context(),
                description: k.action.describe(),
            },
            KeyActionTree::Mode { name, .. } => DisplayableCommand {
                keybinds: key.to_string().into(),
                context: "TODO".into(),
                description: name
                    .as_ref()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| key.to_string())
                    .into(),
            },
        }
    }
}
