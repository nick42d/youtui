use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, char::ParseCharError, fmt::Display, str::FromStr};

#[derive(Hash, Eq, PartialEq, PartialOrd, Debug, Deserialize, Clone, Serialize)]
#[serde(try_from = "String")]
/// A keybind - particularly, a KeyCode that may have 0 to many KeyModifiers.
pub struct Keybind {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
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
}
// Since KeyCode and KeyModifiers derive PartialOrd, it's safe to implement this
// as per below.
//
// Upstream PR that would allow derive(Ord): https://github.com/crossterm-rs/crossterm/pull/951
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Keybind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).expect("Keybind should be able to provide ordering for any values. Has crossterm made a breaking change?")
    }
}
impl TryFrom<String> for Keybind {
    type Error = <Keybind as FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        FromStr::from_str(&value)
    }
}
impl FromStr for Keybind {
    type Err = String;
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
        // For ergonomics and to reduce edge cases, all whitespace is removed prior to
        // parsing.
        let s = s.split_whitespace().collect::<String>();
        if let Ok(code) = parse_unmodified(&s) {
            return Ok(Keybind::new(code, KeyModifiers::NONE));
        };
        let mut split = s.rsplit("-");
        if let Some(Ok(code)) = split.next().map(parse_unmodified) {
            if let Ok(Ok(mut modifiers)) = split
                .map(char::from_str)
                .map(|res| res.map(parse_modifier))
                .collect::<Result<Result<KeyModifiers, char>, ParseCharError>>()
            {
                // If the keycode is a character, then the shift modifier should be removed. It
                // will be encoded in the character already.
                if let KeyCode::Char(_) = code {
                    modifiers = modifiers.difference(KeyModifiers::SHIFT);
                }
                return Ok(Keybind::new(code, modifiers));
            }
        }
        Err(s.to_string())
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

#[cfg(test)]
mod tests {
    use super::Keybind;
    use crossterm::event::{KeyCode, KeyModifiers};
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
