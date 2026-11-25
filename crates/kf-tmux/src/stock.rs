//! Stock tmux 3.4 key tables (`tmux -f /dev/null list-keys`, Ubuntu 24.04).

use std::sync::OnceLock;

use kf_core::bindings::{parse_list_keys, KeyTables};

/// Raw `list-keys` output of a stock tmux 3.4 server.
const LIST_KEYS: &str = include_str!("../data/tmux-3.4-list-keys.txt");

/// The stock tmux 3.4 key tables, parsed once.
pub fn tables() -> &'static KeyTables {
    static TABLES: OnceLock<KeyTables> = OnceLock::new();
    TABLES.get_or_init(|| parse_list_keys(LIST_KEYS).0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use kf_core::bindings::{parse_block, CommandLine};
    use kf_core::key::Key;

    fn first(key: &str) -> Vec<CommandLine> {
        let key: Key = key.parse().unwrap();
        tables().lookup("prefix", &key).unwrap().commands.clone()
    }

    #[test]
    fn stock_has_basics() {
        assert_eq!(first("%")[0].0, vec!["split-window", "-h"]);
        assert_eq!(first("\"")[0].0, vec!["split-window"]);
        assert_eq!(first("c")[0].0, vec!["new-window"]);
        let x = first("x");
        assert_eq!(x.len(), 1);
        assert_eq!(x[0].0.first().unwrap(), "confirm-before");
        assert_eq!(x[0].0.last().unwrap(), "kill-pane");
        let comma = first(",");
        assert_eq!(comma[0].0[0], "command-prompt");
        let block = parse_block(comma[0].0.last().unwrap()).unwrap();
        assert_eq!(block[0].0, vec!["rename-window", "%%"]);
    }

    #[test]
    fn stock_parses_cleanly() {
        let (t, w) = parse_list_keys(LIST_KEYS);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            t.tables().collect::<Vec<_>>(),
            vec!["copy-mode", "copy-mode-vi", "prefix"]
        );
        // The stock root table holds only mouse bindings.
        assert!(t.lookup("root", &"C-b".parse().unwrap()).is_none());
        assert!(t.skipped_mouse() > 0);
        assert_eq!(tables(), &t);
    }

    #[test]
    fn stock_keys_for_split() {
        assert_eq!(
            tables().keys_for("prefix", &["split-window", "-h"]),
            vec!["%".parse::<Key>().unwrap()]
        );
    }
}
