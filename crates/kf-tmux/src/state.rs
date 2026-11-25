//! The tmux server model: server → sessions → windows → panes.

/// A pane's id, unique within the server (tmux `%N`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u32);

/// One pane and its fake content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub id: PaneId,
    /// Content lines, oldest first; the last lines that fit are shown.
    pub lines: Vec<String>,
}

/// Split direction. `H` = side by side (`split-window -h`), `V` = stacked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitDir {
    H,
    V,
}

/// Layout tree of a window. Children of a split get equal-ish sizes with
/// one border column/row between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layout {
    Pane(PaneId),
    Split {
        dir: SplitDir,
        children: Vec<Layout>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub index: u32,
    pub name: String,
    pub layout: Layout,
    pub panes: Vec<Pane>,
    pub active: PaneId,
    pub last_active: Option<PaneId>,
    pub zoomed: bool,
}

impl Window {
    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub windows: Vec<Window>,
    /// Index (`Window::index`, not position) of the current window.
    pub current: u32,
    /// Index of the last (previously current) window.
    pub last: Option<u32>,
}

impl Session {
    pub fn window(&self, index: u32) -> Option<&Window> {
        self.windows.iter().find(|w| w.index == index)
    }

    pub fn current_window(&self) -> Option<&Window> {
        self.window(self.current)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    pub sessions: Vec<Session>,
    /// Position in `sessions` of the attached session; `None` = detached.
    pub attached: Option<usize>,
    /// Id for the next new pane.
    pub next_pane: u32,
    /// Message shown on the status line (`display-message`, errors).
    pub message: Option<String>,
}

impl Server {
    pub fn attached_session(&self) -> Option<&Session> {
        self.attached.and_then(|i| self.sessions.get(i))
    }
}
