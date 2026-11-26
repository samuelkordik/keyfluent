//! Keyfluent core: cards, grading and scheduling. No UI code.

pub mod bindings;
pub mod card;
pub mod grade;
pub mod grid;
pub mod key;

pub mod prefs;
pub mod schedule;
pub mod session;
pub mod speed;
pub mod stats;
pub mod store;

/// Crate version, shared by the whole workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a simulator did with one key press.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyOutcome {
    /// The key was typed text: a character inserted into an open prompt
    /// (rename, the command prompt), a search or the shell line. Such keys
    /// don't count against the failure budget (spec §6.4). Keys that edit,
    /// submit or cancel a prompt (`C-u`, `BSpace`, `Enter`, `Escape`, `C-c`)
    /// and confirm answers (`y`/`n`) are not text entry: they count as
    /// binding keys, like the `Keys` steps of a canonical route.
    pub text_entry: bool,
    /// The key-table binding this key ran (directly, after the prefix, or as
    /// a `-r` repeat); `None` for prompt keys, the prefix key itself and
    /// unbound keys.
    pub ran: Option<RanBinding>,
}

/// A key-table binding that a key press ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RanBinding {
    /// The table it is bound in, e.g. `prefix` or `root`.
    pub table: String,
    /// The key it is bound to.
    pub key: key::Key,
    /// The name of its first command as written in the table, e.g.
    /// `split-window` (a wrapper such as `confirm-before` names itself).
    pub command: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
