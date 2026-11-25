//! Key notation: tmux `key-string.c` conventions (`C-b`, `S-Up`, `PPage`, `F5`).

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// A key with a fixed tmux name (not a plain character).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum NamedKey {
    Enter,
    Escape,
    Space,
    Tab,
    BTab,
    BSpace,
    Up,
    Down,
    Left,
    Right,
    PPage,
    NPage,
    Home,
    End,
    IC,
    DC,
    /// Function key `F1`..=`F12`.
    F(u8),
}

/// The key itself, without modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum KeyBase {
    Char(char),
    Named(NamedKey),
}

/// A key press: a base key plus modifiers. `shift` is only used with named keys
/// (`S-Up`); characters carry their case (`B` vs `b`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Key {
    pub base: KeyBase,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

/// Why a key string could not be parsed.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum KeyParseError {
    #[error("empty key")]
    Empty,
    #[error("modifier without a key in {0:?}")]
    MissingKey(String),
    #[error("unknown key {0:?}")]
    Unknown(String),
    #[error("S- only applies to named keys, not {0:?}")]
    ShiftOnChar(String),
    #[error("no key types the character {0:?}")]
    Untypeable(char),
}

const NAMES: [(&str, NamedKey); 16] = [
    ("Enter", NamedKey::Enter),
    ("Escape", NamedKey::Escape),
    ("Space", NamedKey::Space),
    ("Tab", NamedKey::Tab),
    ("BTab", NamedKey::BTab),
    ("BSpace", NamedKey::BSpace),
    ("Up", NamedKey::Up),
    ("Down", NamedKey::Down),
    ("Left", NamedKey::Left),
    ("Right", NamedKey::Right),
    ("PPage", NamedKey::PPage),
    ("NPage", NamedKey::NPage),
    ("Home", NamedKey::Home),
    ("End", NamedKey::End),
    ("IC", NamedKey::IC),
    ("DC", NamedKey::DC),
];

const ALIASES: [(&str, NamedKey); 6] = [
    ("PageUp", NamedKey::PPage),
    ("PgUp", NamedKey::PPage),
    ("PageDown", NamedKey::NPage),
    ("PgDn", NamedKey::NPage),
    ("Insert", NamedKey::IC),
    ("Delete", NamedKey::DC),
];

fn parse_named(s: &str) -> Option<NamedKey> {
    if let Some((_, k)) = NAMES.iter().chain(ALIASES.iter()).find(|(n, _)| *n == s) {
        return Some(*k);
    }
    let n: u8 = s.strip_prefix('F')?.parse().ok()?;
    // Reject forms like "F+5" / "F05" so that parse and display round-trip.
    if (1..=12).contains(&n) && n.to_string() == s[1..] {
        Some(NamedKey::F(n))
    } else {
        None
    }
}

impl fmt::Display for NamedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let NamedKey::F(n) = self {
            return write!(f, "F{n}");
        }
        match NAMES.iter().find(|(_, k)| k == self) {
            Some((n, _)) => f.write_str(n),
            None => Ok(()),
        }
    }
}

impl FromStr for Key {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(KeyParseError::Empty);
        }
        let (mut ctrl, mut meta, mut shift) = (false, false, false);
        let mut rest = s;
        // `^B` is tmux's alias for `C-b`.
        let mut chars = s.chars();
        if let (Some('^'), Some(c), None) = (chars.next(), chars.next(), chars.next()) {
            rest = &s[1..];
            ctrl = true;
            if c.is_whitespace() || c.is_control() {
                return Err(KeyParseError::Unknown(s.to_owned()));
            }
        }
        while !ctrl || rest.len() > 1 {
            let Some((flag, tail)) = [("C-", 0), ("M-", 1), ("S-", 2)]
                .iter()
                .find_map(|(p, i)| rest.strip_prefix(p).map(|t| (*i, t)))
            else {
                break;
            };
            if tail.is_empty() {
                return Err(KeyParseError::MissingKey(s.to_owned()));
            }
            match flag {
                0 => ctrl = true,
                1 => meta = true,
                _ => shift = true,
            }
            rest = tail;
        }
        let base = if let Some(n) = parse_named(rest) {
            KeyBase::Named(n)
        } else {
            let mut it = rest.chars();
            match (it.next(), it.next()) {
                (Some(' '), None) => KeyBase::Named(NamedKey::Space),
                (Some(c), None) if !c.is_whitespace() && !c.is_control() => {
                    if shift {
                        return Err(KeyParseError::ShiftOnChar(s.to_owned()));
                    }
                    KeyBase::Char(if ctrl { c.to_ascii_lowercase() } else { c })
                }
                _ => return Err(KeyParseError::Unknown(s.to_owned())),
            }
        };
        Ok(Key {
            base,
            ctrl,
            meta,
            shift,
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.meta {
            f.write_str("M-")?;
        }
        if self.ctrl {
            f.write_str("C-")?;
        }
        if self.shift {
            f.write_str("S-")?;
        }
        match self.base {
            KeyBase::Char(c) => write!(f, "{c}"),
            KeyBase::Named(n) => write!(f, "{n}"),
        }
    }
}

impl Key {
    /// A plain key with no modifiers.
    fn plain(base: KeyBase) -> Key {
        Key {
            base,
            ctrl: false,
            meta: false,
            shift: false,
        }
    }

    /// The key that types `c`: `' '` is `Space`, `'\t'` is `Tab`, `'\n'` and
    /// `'\r'` are `Enter`, a printable character is itself. `None` for other
    /// control characters and other whitespace (no key notation names them).
    pub fn from_char(c: char) -> Option<Key> {
        let base = match c {
            ' ' => KeyBase::Named(NamedKey::Space),
            '\t' => KeyBase::Named(NamedKey::Tab),
            '\n' | '\r' => KeyBase::Named(NamedKey::Enter),
            c if c.is_control() || c.is_whitespace() => return None,
            c => KeyBase::Char(c),
        };
        Some(Key::plain(base))
    }

    /// The same key in the form [`FromStr`] produces, so keys from any source
    /// compare equal to parsed ones: `shift` on a character is folded into it
    /// (shift + `b` is `B`; dropped for non-letters, whose case the character
    /// already carries), `ctrl` with an ASCII letter lower-cases it (`C-B` is
    /// `C-b`), and the character `' '` becomes `Space`. Named keys keep `shift`.
    pub fn normalized(self) -> Key {
        let KeyBase::Char(mut c) = self.base else {
            return self;
        };
        if self.shift {
            c = c.to_ascii_uppercase();
        }
        if self.ctrl {
            c = c.to_ascii_lowercase();
        }
        let base = if c == ' ' {
            KeyBase::Named(NamedKey::Space)
        } else {
            KeyBase::Char(c)
        };
        Key {
            base,
            shift: false,
            ..self
        }
    }

    /// Beginner-friendly keycap labels, e.g. `["Ctrl", "b"]`, `["Shift", "↑"]`,
    /// `["Page Up"]`. Meta is shown as `Alt` (the app relabels per platform).
    /// An upper-case ASCII letter shows as `["Shift", "b"]`.
    pub fn keycaps(&self) -> Vec<String> {
        let mut caps = Vec::new();
        let mut shift = self.shift;
        if self.ctrl {
            caps.push("Ctrl".to_owned());
        }
        if self.meta {
            caps.push("Alt".to_owned());
        }
        let label = match self.base {
            KeyBase::Char(c) if c.is_ascii_uppercase() => {
                shift = true;
                c.to_ascii_lowercase().to_string()
            }
            KeyBase::Char(c) => c.to_string(),
            KeyBase::Named(NamedKey::BTab) => {
                shift = true;
                "Tab".to_owned()
            }
            KeyBase::Named(n) => match n {
                NamedKey::Escape => "Esc".to_owned(),
                NamedKey::BSpace => "Backspace".to_owned(),
                NamedKey::Up => "↑".to_owned(),
                NamedKey::Down => "↓".to_owned(),
                NamedKey::Left => "←".to_owned(),
                NamedKey::Right => "→".to_owned(),
                NamedKey::PPage => "Page Up".to_owned(),
                NamedKey::NPage => "Page Down".to_owned(),
                NamedKey::IC => "Insert".to_owned(),
                NamedKey::DC => "Delete".to_owned(),
                other => other.to_string(),
            },
        };
        if shift {
            caps.push("Shift".to_owned());
        }
        caps.push(label);
        caps
    }
}

/// Parse a space-separated key sequence such as `"C-b %"`.
pub fn parse_keys(s: &str) -> Result<Vec<Key>, KeyParseError> {
    s.split_whitespace().map(str::parse).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for s in [
            "C-b", "%", "\"", "Space", "Enter", "Escape", "S-Up", "C-Up", "M-1", "PPage", "F5",
            "B", "#", "C-o", "M-C-b", "C-Space", "M-Up", "S-Down",
        ] {
            assert_eq!(s.parse::<Key>().unwrap().to_string(), s);
        }
    }

    #[test]
    fn modifier_order_normalises() {
        assert_eq!("C-M-b".parse::<Key>().unwrap().to_string(), "M-C-b");
        assert_eq!(
            "C-M-b".parse::<Key>().unwrap(),
            "M-C-b".parse::<Key>().unwrap()
        );
    }

    #[test]
    fn aliases() {
        assert_eq!("C-B".parse::<Key>().unwrap().to_string(), "C-b");
        assert_eq!("^B".parse::<Key>().unwrap().to_string(), "C-b");
        assert_eq!("PageUp".parse::<Key>().unwrap().to_string(), "PPage");
    }

    #[test]
    fn keycaps() {
        assert_eq!("C-b".parse::<Key>().unwrap().keycaps(), vec!["Ctrl", "b"]);
        assert_eq!("%".parse::<Key>().unwrap().keycaps(), vec!["%"]);
        assert_eq!("S-Up".parse::<Key>().unwrap().keycaps(), vec!["Shift", "↑"]);
        assert_eq!("PPage".parse::<Key>().unwrap().keycaps(), vec!["Page Up"]);
        assert_eq!("Escape".parse::<Key>().unwrap().keycaps(), vec!["Esc"]);
        assert_eq!("M-1".parse::<Key>().unwrap().keycaps(), vec!["Alt", "1"]);
    }

    #[test]
    fn rejects_garbage() {
        for s in ["", "C-", "Nope", "C-M-"] {
            assert!(s.parse::<Key>().is_err());
        }
    }

    #[test]
    fn edge_cases_do_not_panic() {
        for s in [
            "^", "^^", "C--", "S-a", "F0", "F13", "F05", "é", "C-é", "^é", " ", "M-C-S-Up", "💥💥",
        ] {
            let _ = s.parse::<Key>();
        }
        assert_eq!("^".parse::<Key>().unwrap().to_string(), "^");
        assert_eq!("C--".parse::<Key>().unwrap().to_string(), "C--");
        assert!("F13".parse::<Key>().is_err());
        assert!("S-a".parse::<Key>().is_err());
    }

    #[test]
    fn from_char() {
        let k = |s: &str| Some(s.parse::<Key>().unwrap());
        assert_eq!(Key::from_char(' '), k("Space"));
        assert_eq!(Key::from_char('\t'), k("Tab"));
        assert_eq!(Key::from_char('\n'), k("Enter"));
        assert_eq!(Key::from_char('\r'), k("Enter"));
        assert_eq!(Key::from_char('a'), k("a"));
        assert_eq!(Key::from_char('%'), k("%"));
        assert_eq!(Key::from_char('é'), k("é"));
        for c in ['\u{0}', '\u{1b}', '\u{7f}', '\u{a0}'] {
            assert_eq!(Key::from_char(c), None, "{c:?}");
        }
    }

    #[test]
    fn normalized_matches_parsing() {
        let raw = |base, ctrl, shift| Key {
            base,
            ctrl,
            meta: false,
            shift,
        };
        let k = |s: &str| s.parse::<Key>().unwrap();
        let c = KeyBase::Char;
        assert_eq!(raw(c('B'), true, false).normalized(), k("C-b"));
        assert_eq!(raw(c('b'), false, true).normalized(), k("B"));
        assert_eq!(raw(c('b'), true, true).normalized(), k("C-b"));
        assert_eq!(raw(c('%'), false, true).normalized(), k("%"));
        assert_eq!(raw(c(' '), false, false).normalized(), k("Space"));
        assert_eq!(raw(c(' '), true, false).normalized(), k("C-Space"));
        let up = raw(KeyBase::Named(NamedKey::Up), false, true);
        assert_eq!(up.normalized(), k("S-Up"));
        for s in ["C-b", "B", "%", "M-C-b", "Space", "S-Up", "é"] {
            assert_eq!(k(s).normalized(), k(s), "{s}");
        }
    }

    #[test]
    fn sequences() {
        let k = parse_keys("C-b  %").unwrap();
        assert_eq!(k.len(), 2);
        assert!(parse_keys("C-b Nope").is_err());
        assert!(parse_keys("").unwrap().is_empty());
    }
}
