//! The tmux card packs.

use kf_core::card::{load_cards, Card, CardError, Step};
use kf_core::key::Key;

use crate::goal::Goal;

const BASICS: &str = include_str!("../cards/basics.toml");

/// The Basics pack: splitting, moving between, zooming and closing panes,
/// and opening, switching, renaming and closing windows.
pub fn basics() -> Result<Vec<Card<Goal>>, CardError> {
    load_cards(BASICS)
}

/// A card's canonical answer as key presses: `Keys` steps as written, `Text`
/// steps one key per character ([`Key::from_char`]: `Space`, `Tab`, `Enter`
/// for space, tab and newline).
pub fn canonical_keys(card: &Card<Goal>) -> Vec<Key> {
    let mut out = Vec::new();
    for step in &card.canonical {
        match step {
            Step::Keys(keys) => out.extend_from_slice(keys),
            // Loading rejected characters without a key, so none are skipped.
            Step::Text(text) => out.extend(text.chars().filter_map(Key::from_char)),
        }
    }
    out
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

    #[test]
    fn text_tab_and_newline_become_named_keys() {
        let src = "[[card]]\nid = \"t\"\ncategory = \"c\"\nintent = \"i\"\nstart = \"one-pane\"\ngoal = {}\ncanonical = [{ text = \"a\\tb\\n\" }]\n";
        let cards: Vec<Card<Goal>> = load_cards(src).unwrap();
        let keys: Vec<String> = canonical_keys(&cards[0])
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(keys, ["a", "Tab", "b", "Enter"]);
    }
}
