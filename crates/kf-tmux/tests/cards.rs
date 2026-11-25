//! Content tests for the Basics card pack: every card is reachable with its
//! canonical keys, and those keys are stock tmux 3.4 bindings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use kf_core::card::Step;
use kf_core::key::Key;
use kf_tmux::{fixtures, pack, stock, Tmux};

#[test]
fn every_card_reaches_its_goal_and_starts_unmet() {
    for card in pack::basics().unwrap() {
        let start = fixtures::get(&card.start)
            .unwrap_or_else(|| panic!("{}: unknown start fixture {:?}", card.id, card.start));
        let mut t = Tmux::new(start, stock::tables().clone());
        assert!(
            !card.goal.met(t.server()),
            "{}: goal already met at start",
            card.id
        );
        for k in pack::canonical_keys(&card) {
            t.key(&k);
        }
        assert!(
            card.goal.met(t.server()),
            "{}: canonical keys miss the goal",
            card.id
        );
    }
}

/// The first command a card's prefix binding must run. `Exact` = the whole
/// command line; `Wraps(cmd, inner)` = a `confirm-before` / `command-prompt`
/// wrapper whose last token runs `inner`.
enum Expect {
    Exact(&'static [&'static str]),
    Wraps(&'static str, &'static str),
}

fn expected(id: &str) -> Expect {
    use Expect::{Exact, Wraps};
    match id {
        "split-h" => Exact(&["split-window", "-h"]),
        "split-v" => Exact(&["split-window"]),
        "pane-left" => Exact(&["select-pane", "-L"]),
        "pane-right" => Exact(&["select-pane", "-R"]),
        "pane-up" => Exact(&["select-pane", "-U"]),
        "pane-down" => Exact(&["select-pane", "-D"]),
        "pane-next" => Exact(&["select-pane", "-t", ":.+"]),
        "pane-last" => Exact(&["last-pane"]),
        "pane-zoom" => Exact(&["resize-pane", "-Z"]),
        "pane-kill" => Wraps("confirm-before", "kill-pane"),
        "window-new" => Exact(&["new-window"]),
        "window-next" => Exact(&["next-window"]),
        "window-prev" => Exact(&["previous-window"]),
        "window-select" => Exact(&["select-window", "-t", ":=2"]),
        "window-last" => Exact(&["last-window"]),
        "window-rename" => Wraps("command-prompt", "rename-window"),
        "window-kill" => Wraps("confirm-before", "kill-window"),
        other => panic!("{other}: no expected command listed in this test"),
    }
}

/// Checks only keys that immediately follow the prefix key; keys answering a
/// confirm (`y`) or typed into a prompt (`C-u`, `Enter`) are not bindings.
#[test]
fn canonical_keys_are_stock_bindings() {
    let tables = stock::tables();
    let prefix: Key = "C-b".parse().unwrap();
    for card in pack::basics().unwrap() {
        let mut prefix_keys = Vec::new();
        for step in &card.canonical {
            if let Step::Keys(keys) = step {
                for pair in keys.windows(2) {
                    if let [first, second] = pair {
                        if *first == prefix {
                            prefix_keys.push(*second);
                        }
                    }
                }
            }
        }
        assert_eq!(
            prefix_keys.len(),
            1,
            "{}: expected exactly one prefix binding",
            card.id
        );
        let key = prefix_keys.first().unwrap();
        let binding = tables
            .lookup("prefix", key)
            .unwrap_or_else(|| panic!("{}: {key} is not bound in prefix", card.id));
        let first = &binding.commands.first().unwrap().0;
        match expected(&card.id) {
            Expect::Exact(words) => assert_eq!(first, words, "{}", card.id),
            Expect::Wraps(cmd, inner) => {
                assert_eq!(first.first().map(String::as_str), Some(cmd), "{}", card.id);
                assert!(
                    first.last().is_some_and(|t| t.contains(inner)),
                    "{}: {first:?} does not run {inner}",
                    card.id
                );
            }
        }
    }
}

#[test]
fn basics_has_17_cards_with_unique_ids() {
    let cards = pack::basics().unwrap();
    assert_eq!(cards.len(), 17);
    let ids: HashSet<&str> = cards.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 17);
    for c in &cards {
        assert!(!c.teach.trim().is_empty(), "{}: no teach text", c.id);
        assert!(
            matches!(c.category.as_str(), "panes" | "windows"),
            "{}",
            c.id
        );
    }
}

#[test]
fn canonical_keys_expand_text() {
    let cards = pack::basics().unwrap();
    let rename = cards.iter().find(|c| c.id == "window-rename").unwrap();
    let keys: Vec<String> = pack::canonical_keys(rename)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(keys, ["C-b", ",", "C-u", "l", "o", "g", "s", "Enter"]);
}
