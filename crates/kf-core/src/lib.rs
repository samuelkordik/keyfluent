//! Keyfluent core: cards, grading and scheduling. No UI code.

pub mod bindings;
pub mod card;
pub mod grid;
pub mod key;

/// Crate version, shared by the whole workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a simulator did with one key press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyOutcome {
    /// The key was typed text: input to an open prompt (rename, the command
    /// prompt), a search or the shell line. Such keys don't count against the
    /// failure budget (spec §6.4). Answering a confirm (`y`/`n`) is not text
    /// entry: it counts as a binding key.
    pub text_entry: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
