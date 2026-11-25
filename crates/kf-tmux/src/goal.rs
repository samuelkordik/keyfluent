//! Card goals for the tmux pack: a target state of the attached session's
//! current window.

use serde::Deserialize;

use crate::commands::PANE_AREA;
use crate::layout::geometry;
use crate::state::{Layout, Server, SplitDir};

/// What the screen must look like for a card to be done. Every field that
/// is set must hold for the attached session's current window; unset fields
/// are ignored. When no session is attached, every set field is unmet.
/// (Copy-mode and session fields arrive in P4.)
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    /// Number of panes in the current window.
    pub panes: Option<usize>,
    /// The window's top-level layout is a split in this direction.
    pub split: Option<SplitDir>,
    /// Position of the active pane in layout (tmux pane index) order.
    pub active_pane: Option<usize>,
    /// The current window is zoomed (or not).
    pub zoomed: Option<bool>,
    /// Number of windows in the attached session.
    pub windows: Option<usize>,
    /// Index of the current window.
    pub current_window: Option<u32>,
    /// Name of the current window.
    pub window_name: Option<String>,
}

impl Goal {
    /// Whether every set field holds in `server`.
    pub fn met(&self, server: &Server) -> bool {
        let Some(session) = server.attached_session() else {
            return self == &Goal::default();
        };
        if !self.windows.is_none_or(|n| session.windows.len() == n) {
            return false;
        }
        let Some(w) = session.current_window() else {
            return self.panes.is_none()
                && self.split.is_none()
                && self.active_pane.is_none()
                && self.zoomed.is_none()
                && self.current_window.is_none()
                && self.window_name.is_none();
        };
        let active_pos = || {
            geometry(&w.layout, PANE_AREA)
                .iter()
                .position(|(id, _)| *id == w.active)
        };
        self.panes.is_none_or(|n| w.panes.len() == n)
            && self
                .split
                .is_none_or(|dir| matches!(w.layout, Layout::Split { dir: d, .. } if d == dir))
            && self.active_pane.is_none_or(|pos| active_pos() == Some(pos))
            && self.zoomed.is_none_or(|z| w.zoomed == z)
            && self.current_window.is_none_or(|i| w.index == i)
            && self.window_name.as_ref().is_none_or(|name| &w.name == name)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn goal(src: &str) -> Goal {
        toml::from_str(src).unwrap()
    }

    #[test]
    fn fields_read_the_current_window() {
        let s = fixtures::get("two-panes-h").unwrap();
        assert!(goal("panes = 2\nsplit = \"h\"\nactive_pane = 1").met(&s));
        assert!(!goal("split = \"v\"").met(&s));
        assert!(!goal("active_pane = 0").met(&s));
        assert!(
            goal("zoomed = false\nwindows = 1\ncurrent_window = 0\nwindow_name = \"bash\"").met(&s)
        );
        assert!(!goal("window_name = \"logs\"").met(&s));
        let s = fixtures::get("three-windows").unwrap();
        assert!(goal("windows = 3\ncurrent_window = 1\nwindow_name = \"logs\"").met(&s));
        assert!(!goal("current_window = 2").met(&s));
        assert!(!goal("split = \"h\"").met(&s));
    }

    #[test]
    fn active_pane_is_layout_position() {
        // four-panes: active is p3, last in layout order.
        let s = fixtures::get("four-panes").unwrap();
        assert!(goal("active_pane = 3").met(&s));
        assert!(!goal("active_pane = 0").met(&s));
    }

    #[test]
    fn detached_leaves_set_fields_unmet() {
        let mut s = fixtures::get("one-pane").unwrap();
        s.attached = None;
        assert!(!goal("panes = 1").met(&s));
        assert!(!goal("windows = 1").met(&s));
        assert!(goal("").met(&s));
    }

    #[test]
    fn unknown_fields_and_bad_split_are_rejected() {
        assert!(toml::from_str::<Goal>("pane = 2").is_err());
        assert!(toml::from_str::<Goal>("split = \"x\"").is_err());
    }
}
