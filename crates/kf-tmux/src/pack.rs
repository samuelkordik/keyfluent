//! The tmux card packs.

use kf_core::card::{load_cards, Card, CardError, Step};
use kf_core::key::Key;
use kf_core::RanBinding;

use crate::goal::Goal;
use crate::sim::Tmux;
use crate::{fixtures, stock};

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

/// The bindings a card's canonical answer runs: replays the canonical keys
/// from the card's start fixture with the stock tables and collects every
/// binding that ran. Empty if the start fixture is unknown.
pub fn practiced_bindings(card: &Card<Goal>) -> Vec<RanBinding> {
    let Some(server) = fixtures::get(&card.start) else {
        return Vec::new();
    };
    let mut tmux = Tmux::new(server, stock::tables().clone());
    canonical_keys(card)
        .iter()
        .filter_map(|k| tmux.key(k).ran)
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn card(id: &str) -> Card<Goal> {
        basics().unwrap().into_iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn practiced_split_h() {
        let p = practiced_bindings(&card("split-h"));
        assert_eq!(p.len(), 1);
        assert_eq!(
            (
                p[0].table.as_str(),
                p[0].key.to_string(),
                p[0].command.as_str()
            ),
            ("prefix", "%".to_string(), "split-window")
        );
    }

    #[test]
    fn practiced_window_rename_starts_with_command_prompt() {
        let p = practiced_bindings(&card("window-rename"));
        assert_eq!(p[0].command, "command-prompt");
    }

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
