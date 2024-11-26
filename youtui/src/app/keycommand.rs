//! KeyCommand and Keybind model.
//! A KeyCommand is a pairing of Keybinds to an Action or a Mode.
//! A Mode is a modified set of KeyCommands accessible after pressing Keybinds.
use super::component::actionhandler::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, char::ParseCharError, fmt::Display, str::FromStr};

// Should another type be GlobalHidden?
#[derive(PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
pub enum CommandVisibility {
    #[default]
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
#[derive(Hash, Eq, PartialEq, Debug, Deserialize, Clone, Serialize)]
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
pub enum Keymap<A: Action> {
    Action(A),
    Mode(Mode<A>),
}
#[derive(PartialEq, Debug, Clone)]
pub struct Mode<A: Action> {
    pub name: &'static str,
    pub commands: Vec<KeyCommand<A>>,
}
#[derive(PartialEq, Debug, Clone)]
pub struct DisplayableCommand<'a> {
    // XXX: Do we also want to display sub-keys in Modes?
    pub keybinds: Cow<'a, str>,
    pub context: Cow<'a, str>,
    pub description: Cow<'a, str>,
}
pub struct DisplayableMode<'a> {
    pub displayable_commands: Box<dyn Iterator<Item = DisplayableCommand<'a>> + 'a>,
    pub description: Cow<'a, str>,
}

impl<'a, A: Action + 'a> From<&'a KeyCommand<A>> for DisplayableCommand<'a> {
    fn from(value: &'a KeyCommand<A>) -> Self {
        // XXX: Do we also want to display sub-keys in Modes?
        Self {
            keybinds: value.to_string().into(),
            context: value.context(),
            description: value.describe(),
        }
    }
}

impl Keybind {
    fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
    fn contains_keyevent(&self, keyevent: &KeyEvent) -> bool {
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

impl<A: Action> Display for KeyCommand<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let w: String =
            // NOTE: Replace with standard library method once stabilised.
            itertools::intersperse(
                self
                    .keybinds
                    .iter()
                    .map(|kb| Cow::from(kb.to_string()))
                ," / ".into()
            )
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
            .first()
            .map(|kb| kb.context())
            .unwrap_or_default()
    }
    pub fn describe(&self) -> Cow<str> {
        self.name.into()
    }
    pub fn as_displayable_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = DisplayableCommand<'a>> + 'a> {
        Box::new(self.commands.iter().map(|bind| bind.as_displayable()))
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
    pub fn as_displayable(&self) -> DisplayableCommand<'_> {
        self.into()
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
            keybinds: vec![Keybind::new(code, KeyModifiers::empty())],
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
            keybinds: vec![Keybind::new(code, modifiers)],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Standard,
        }
    }
    pub fn new_modified_from_code_with_visibility(
        code: KeyCode,
        modifiers: KeyModifiers,
        visibility: CommandVisibility,
        action: A,
    ) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind::new(code, modifiers)],
            key_map: Keymap::Action(action),
            visibility,
        }
    }
    pub fn new_global_modified_from_code(
        code: KeyCode,
        modifiers: KeyModifiers,
        action: A,
    ) -> KeyCommand<A> {
        KeyCommand {
            keybinds: vec![Keybind { code, modifiers }],
            key_map: Keymap::Action(action),
            visibility: CommandVisibility::Global,
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
        todo!("add test to parse all acceptable values");
        let expected = Keybind::new(KeyCode::Enter, KeyModifiers::NONE);
        let kb = Keybind::from_str("enter").unwrap();
        assert_eq!(kb, expected);
        let kb = Keybind::from_str("Enter").unwrap();
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
