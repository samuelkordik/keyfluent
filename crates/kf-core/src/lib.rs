//! Keyfluent core: cards, grading and scheduling. No UI code.

pub mod key;

/// Crate version, shared by the whole workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
