use std::{borrow::Cow, fmt::Display};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// KeyCommand and Keybind model.
/// A KeyCommand is a pairing of Keybinds to an Action or a Mode.
/// A Mode is a modified set of KeyCommands accessible after pressing Keybinds.
use itertools::Itertools;

use super::component::actionhandler::Action;

// Should another type be GlobalHidden?
#[derive(PartialEq, Debug, Clone)]
pub enum CommandVisibility {
    Standard,
    // Displayed on Header
    Global,
    // Not displayed in Help menu
    Hidden,
}

#[derive(PartialEq, Debug, Clone)]
pub struct KeyCommand<A: Action> {
    pub keybinds: Vec<Keybind>,
    pub key_map: Keymap<A>,
    pub visibility: CommandVisibility,
}
#[derive(PartialEq, Debug, Clone)]
pub struct Keybind {
    code: KeyCode,
    modifiers: KeyModifiers,
}
#[derive(PartialEq, Debug, Clone)]
pub enum Keymap<A: Action> {
    Action(A),
    Mode(Mode<A>),
}
#[derive(PartialEq, Debug, Clone)]
pub struct Mode<A: Action> {
    pub name: &'static str,
    pub commands: Vec<KeyCommand<A>>,
}

impl Keybind {
    fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
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
}

impl<A: Action> Display for KeyCommand<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let w: String = self
            .keybinds
            .iter()
            .map(|kb| Cow::from(kb.to_string()))
            // NOTE: Replace with standard library method once stabilised.
            .intersperse(" / ".into())
            .collect();
        write!(f, "{w}")
    }
}

impl Display for Keybind {
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
            KeyModifiers::ALT => write!(f, "A-{code}"),
            KeyModifiers::SHIFT => write!(f, "S-{code}"),
            _ => write!(f, "{code}"),
        }
    }
}

// Is this an implementation of Action?
impl<A: Action> Mode<A> {
    pub fn context(&self) -> Cow<str> {
        self.commands
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
        Box::new(self.commands.iter().map(|bind| bind.as_readable_short()))
    }
    pub fn _as_readable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<str>, Cow<str>, Cow<str>)> + 'a> {
        Box::new(self.commands.iter().map(|bind| bind.as_readable()))
    }
}

impl<A: Action> KeyCommand<A> {
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
    pub fn contains_keyevent(&self, keyevent: &KeyEvent) -> bool {
        for kb in self.keybinds.iter() {
            if kb.contains_keyevent(keyevent) {
                return true;
            }
        }
        false
    }
    pub fn new_from_codes(code: Vec<KeyCode>, action: A) -> KeyCommand<A> {
        let keybinds = code
            .into_iter()
            .map(|kc| Keybind::new(kc, KeyModifiers::empty()))
            .collect();
        KeyCommand {
            keybinds,
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Standard,
        }
    }
    pub fn new_from_code(code: KeyCode, action: A) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind {
                code,
                modifiers: KeyModifiers::empty(),
            }],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Standard,
        }
    }
    pub fn new_modified_from_code(
        code: KeyCode,
        modifiers: KeyModifiers,
        action: A,
    ) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind { code, modifiers }],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Standard,
        }
    }
    pub fn new_global_from_code(code: KeyCode, action: A) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind {
                code,
                modifiers: KeyModifiers::empty(),
            }],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Global,
        }
    }
    pub fn new_hidden_from_code(code: KeyCode, action: A) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind {
                code,
                modifiers: KeyModifiers::empty(),
            }],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Hidden,
        }
    }
    pub fn new_action_only_mode(
        actions: Vec<(KeyCode, A)>,
        code: KeyCode,
        name: &'static str,
    ) -> KeyCommand<A> {
        let commands = actions
            .into_iter()
            .map(|(code, action)| KeyCommand {
                keybinds: vec![Keybind {
                    code,
                    modifiers: KeyModifiers::empty(),
                }],
                key_map: Keymap::Action(action),
                visibility: CommandVisibility::Standard,
            })
            .collect();
        KeyCommand {
            keybinds: vec![Keybind {
                code,
                modifiers: KeyModifiers::empty(),
            }],
            key_map: Keymap::Mode(Mode { commands, name }),
            visibility: CommandVisibility::Standard,
        }
    }
}
