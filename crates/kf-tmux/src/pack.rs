//! The tmux card packs.

use kf_core::card::{load_cards, Card, CardError, Step};
use kf_core::key::{Key, KeyBase, NamedKey};

use crate::goal::Goal;

const BASICS: &str = include_str!("../cards/basics.toml");

/// The Basics pack: splitting, moving between, zooming and closing panes,
/// and opening, switching, renaming and closing windows.
pub fn basics() -> Result<Vec<Card<Goal>>, CardError> {
    load_cards(BASICS)
}

/// A card's canonical answer as key presses: `Keys` steps as written, `Text`
/// steps one key per character (`Space` for a space).
pub fn canonical_keys(card: &Card<Goal>) -> Vec<Key> {
    let mut out = Vec::new();
    for step in &card.canonical {
        match step {
            Step::Keys(keys) => out.extend_from_slice(keys),
            Step::Text(text) => out.extend(text.chars().map(char_key)),
        }
    }
    out
}

fn char_key(c: char) -> Key {
    let base = if c == ' ' {
        KeyBase::Named(NamedKey::Space)
    } else {
        KeyBase::Char(c)
    };
    Key {
        base,
        ctrl: false,
        meta: false,
        shift: false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn text_space_becomes_space_key() {
        let src = "[[card]]\nid = \"t\"\ncategory = \"c\"\nintent = \"i\"\nstart = \"one-pane\"\ngoal = {}\ncanonical = [\"C-b ,\", { text = \"a b\" }]\n";
        let cards: Vec<Card<Goal>> = load_cards(src).unwrap();
        let keys: Vec<String> = canonical_keys(&cards[0])
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(keys, ["C-b", ",", "a", "Space", "b"]);
    }
}
