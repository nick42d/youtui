//! KeyCommand and Keybind model.
//! A KeyCommand is a pairing of Keybinds to an Action or a Mode.
//! A Mode is a modified set of KeyCommands accessible after pressing Keybinds.
use crate::config::keybinds::KeyActionTree;

use super::component::actionhandler::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, char::ParseCharError, fmt::Display, str::FromStr};

// Should another type be GlobalHidden?
#[derive(PartialEq, Copy, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandVisibility {
    #[default]
    Standard,
    // Displayed on Header
    Global,
    // Not displayed in Help menu
    Hidden,
}

#[derive(Hash, Eq, PartialEq, PartialOrd, Debug, Deserialize, Clone, Serialize)]
#[serde(try_from = "String")]
pub struct Keybind {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}
impl TryFrom<String> for Keybind {
    type Error = <Keybind as FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        FromStr::from_str(&value)
    }
}
#[derive(PartialEq, Debug, Clone)]
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

impl Keybind {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
    pub fn new_unmodified(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }
    pub fn contains_keyevent(&self, keyevent: &KeyEvent) -> bool {
        match self.code {
            // If key code is a character it may have shift pressed, if that's the case ignore the
            // shift As may have been used to capitalise the letter, which will already
            // be counted in the key code.
            KeyCode::Char(_) => {
                self.code == keyevent.code
                    && self.modifiers.union(KeyModifiers::SHIFT)
                        == keyevent.modifiers.union(KeyModifiers::SHIFT)
            }
            _ => self.code == keyevent.code && self.modifiers == keyevent.modifiers,
        }
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

#[derive(Debug)]
pub struct KeybindParseError(String);
impl std::fmt::Display for KeybindParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Keybind {
    type Err = KeybindParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_unmodified(s: &str) -> Result<KeyCode, &str> {
            if let Ok(char) = char::from_str(s) {
                return Ok(KeyCode::Char(char));
            }
            match s.to_lowercase().as_str() {
                "enter" => return Ok(KeyCode::Enter),
                "delete" => return Ok(KeyCode::Delete),
                "up" => return Ok(KeyCode::Up),
                "pageup" => return Ok(KeyCode::PageUp),
                "down" => return Ok(KeyCode::Down),
                "pagedown" => return Ok(KeyCode::PageDown),
                "left" => return Ok(KeyCode::Left),
                "right" => return Ok(KeyCode::Right),
                "backspace" => return Ok(KeyCode::Backspace),
                "tab" => return Ok(KeyCode::Tab),
                "backtab" => return Ok(KeyCode::BackTab),
                "esc" => return Ok(KeyCode::Esc),
                // Caps Lock key.
                //
                // **Note:** this key can only be read if
                // [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has
                // been enabled with
                // [`PushKeyboardEnhancementFlags`].
                "caps" => return Ok(KeyCode::CapsLock),
                "home" => return Ok(KeyCode::Home),
                "end" => return Ok(KeyCode::End),
                "insert" => return Ok(KeyCode::Insert),
                "space" => return Ok(KeyCode::Char(' ')),
                _ => (),
            };
            if let Some((before, Ok(num))) = s
                .split_once("F")
                .map(|(before, num)| (before, u8::from_str(num)))
            {
                if before.is_empty() {
                    return Ok(KeyCode::F(num));
                }
            }
            Err(s)
        }
        fn parse_modifier(c: char) -> Result<KeyModifiers, char> {
            match c {
                'A' => Ok(KeyModifiers::ALT),
                'C' => Ok(KeyModifiers::CONTROL),
                'S' => Ok(KeyModifiers::SHIFT),
                c => Err(c),
            }
        }
        if let Ok(code) = parse_unmodified(s) {
            return Ok(Keybind::new(code, KeyModifiers::NONE));
        };
        let mut split = s.rsplit("-");
        if let Some(Ok(code)) = split.next().map(parse_unmodified) {
            if let Ok(Ok(modifiers)) = split
                .map(char::from_str)
                .map(|res| res.map(parse_modifier))
                .collect::<Result<Result<KeyModifiers, char>, ParseCharError>>()
            {
                return Ok(Keybind::new(code, modifiers));
            }
        }
        Err(KeybindParseError(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::Keybind;
    use std::str::FromStr;

    #[test]
    fn parse_char_key() {
        let kb = Keybind::from_str("a").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::Char('a'), KeyModifiers::NONE));
    }
    #[test]
    fn parse_space() {
        Keybind::from_str(" ").unwrap_err();
        let kb = Keybind::from_str("space").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::Char(' '), KeyModifiers::NONE));
    }
    #[test]
    fn parse_f_key() {
        let kb = Keybind::from_str("F10").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::F(10), KeyModifiers::NONE));
    }
    #[test]
    fn parse_enter() {
        let expected = Keybind::new(KeyCode::Enter, KeyModifiers::NONE);
        let kb = Keybind::from_str("enter").unwrap();
        assert_eq!(kb, expected);
        let kb = Keybind::from_str("EnTeR").unwrap();
        assert_eq!(kb, expected);
    }
    #[test]
    fn parse_delete() {
        let kb = Keybind::from_str("delete").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::Delete, KeyModifiers::NONE));
    }
    #[test]
    fn parse_alt_key() {
        let kb = Keybind::from_str("A-a").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::Char('a'), KeyModifiers::ALT));
        let kb = Keybind::from_str("A-enter").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::Enter, KeyModifiers::ALT));
    }
    #[test]
    fn parse_shift_key() {
        let kb = Keybind::from_str("S-F1").unwrap();
        assert_eq!(kb, Keybind::new(KeyCode::F(1), KeyModifiers::SHIFT));
    }
    #[test]
    fn parse_ctrl_key() {
        let kb = Keybind::from_str("C-A-x").unwrap();
        assert_eq!(
            kb,
            Keybind::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );
    }
}
