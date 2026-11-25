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
    /// The key was typed into an open prompt (command prompt, rename,
    /// confirm, search). Such keys don't count against the failure budget
    /// (spec §6.4).
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
