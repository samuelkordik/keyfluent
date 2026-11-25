//! Content tests for the Basics card pack: every card is reachable with its
//! canonical keys, and those keys are stock tmux 3.4 bindings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use kf_core::bindings::Binding;
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
        "window-select" => Exact(&["select-window", "-t", ":=4"]),
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

/// The prefix-table key of a card whose route is one binding: `C-b <key>`,
/// optionally followed by a confirm answer `y`. `None` for longer routes.
fn single_binding(steps: &[Step]) -> Option<Key> {
    let prefix: Key = "C-b".parse().unwrap();
    let y: Key = "y".parse().unwrap();
    let mut keys = Vec::new();
    for step in steps {
        match step {
            Step::Keys(k) => keys.extend_from_slice(k),
            Step::Text(_) => return None,
        }
    }
    match keys.as_slice() {
        [p, k] | [p, k, _] if *p == prefix && keys.get(2).is_none_or(|a| *a == y) => Some(*k),
        _ => None,
    }
}

/// Other bindings that may also solve a card (Ruling 16). Only one kind: on
/// window-next / window-prev / window-last, the number key of the window you
/// end up in (`select-window -t :=N`) is a legitimate alternative route, as
/// every window can be reached by its number. P2's grader caps such a route
/// at Good and names the key the card practises. No true aliases (two keys
/// bound to the identical command) exist for the Basics cards.
fn allowed_alternative(card_id: &str, b: &Binding) -> bool {
    let number_key = b.commands.first().is_some_and(|c| {
        matches!(c.0.as_slice(), [cmd, t, target]
            if cmd == "select-window" && t == "-t" && target.starts_with(":="))
    });
    number_key && matches!(card_id, "window-next" | "window-prev" | "window-last")
}

/// A card must teach its own binding: no other single stock prefix binding,
/// pressed once from the start state, may reach the goal. A confirm that
/// opens is answered `y`; a binding that opens an input prompt is skipped;
/// unsimulated commands fail and leave the state as it was.
#[test]
fn no_other_single_binding_solves_a_card() {
    let tables = stock::tables();
    let prefix: Key = "C-b".parse().unwrap();
    let y: Key = "y".parse().unwrap();
    let mut checked = 0;
    let mut solved_by = Vec::new();
    for card in pack::basics().unwrap() {
        let Some(own) = single_binding(&card.canonical) else {
            continue;
        };
        checked += 1;
        for b in tables.bindings("prefix") {
            if b.key == own || allowed_alternative(&card.id, b) {
                continue;
            }
            let mut t = Tmux::new(fixtures::get(&card.start).unwrap(), tables.clone());
            t.key(&prefix);
            t.key(&b.key);
            if t.in_text_entry() {
                continue;
            }
            // Answers a confirm if one opened; otherwise `y` is unbound in root.
            t.key(&y);
            if card.goal.met(t.server()) {
                solved_by.push(format!("{}: C-b {} {:?}", card.id, b.key, b.commands));
            }
        }
    }
    assert!(
        solved_by.is_empty(),
        "cards also solved by another binding:\n{}",
        solved_by.join("\n")
    );
    assert_eq!(checked, 16, "cards with a single-binding route");
}

/// The simulator and `Card::binding_key_count` agree on which canonical keys
/// are binding keys (text_entry = false): prompt editing/submit keys and
/// confirm answers count, inserted characters don't (Rulings 14 and 17).
#[test]
fn binding_key_count_matches_simulator() {
    for card in pack::basics().unwrap() {
        let mut t = Tmux::new(fixtures::get(&card.start).unwrap(), stock::tables().clone());
        let binding_keys = pack::canonical_keys(&card)
            .iter()
            .filter(|k| !t.key(k).text_entry)
            .count();
        assert_eq!(binding_keys, card.binding_key_count(), "{}", card.id);
    }
}
