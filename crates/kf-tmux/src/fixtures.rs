//! Named start states for cards. Every fixture has one session `0`, attached.

use crate::state::{Layout, Pane, PaneId, Server, Session, SplitDir, Window};

/// Names accepted by [`get`].
pub const NAMES: &[&str] = &[
    "one-pane",
    "two-panes-h",
    "two-panes-h-first",
    "two-panes-v",
    "two-panes-v-first",
    "four-panes",
    "three-windows",
];

const PROMPT: &str = "user@host:~$ ";

/// The fixture called `name`, or `None` if there is none.
pub fn get(name: &str) -> Option<Server> {
    let server = match name {
        "one-pane" => server(vec![one_pane_window(0, "bash", 0, shell())], 0, None),
        "two-panes-h" => two_panes(SplitDir::H, 1),
        "two-panes-h-first" => two_panes(SplitDir::H, 0),
        "two-panes-v" => two_panes(SplitDir::V, 1),
        "two-panes-v-first" => two_panes(SplitDir::V, 0),
        "four-panes" => four_panes(),
        "three-windows" => server(
            vec![
                one_pane_window(0, "bash", 0, shell()),
                one_pane_window(1, "logs", 1, logs()),
                one_pane_window(2, "editor", 2, editor()),
            ],
            1,
            Some(0),
        ),
        _ => return None,
    };
    Some(server)
}

fn lines(text: &[&str]) -> Vec<String> {
    text.iter().map(|s| (*s).to_owned()).collect()
}

fn shell() -> Vec<String> {
    lines(&[
        "user@host:~$ ls",
        "notes.txt  projects  scripts",
        "user@host:~$ cd projects",
        "user@host:~/projects$ git status",
        "On branch main",
        "nothing to commit, working tree clean",
        "user@host:~/projects$ ",
    ])
}

fn logs() -> Vec<String> {
    lines(&[
        "user@host:~$ tail -f /var/log/app.log",
        "12:00:01 INFO  server started on :8080",
        "12:00:04 INFO  GET /health 200",
        "12:00:09 WARN  slow request: GET /report (1.2s)",
        "12:00:12 INFO  GET /health 200",
    ])
}

fn editor() -> Vec<String> {
    lines(&[
        "fn main() {",
        "    println!(\"hello, tmux\");",
        "}",
        "~",
        "~",
        "\"main.rs\" 3L, 42B",
    ])
}

fn prompt_only() -> Vec<String> {
    vec![PROMPT.to_owned()]
}

fn pane(id: u32, lines: Vec<String>) -> Pane {
    Pane {
        id: PaneId(id),
        lines,
    }
}

fn split(dir: SplitDir, ids: &[u32]) -> Layout {
    Layout::Split {
        dir,
        children: ids.iter().map(|id| Layout::Pane(PaneId(*id))).collect(),
    }
}

fn one_pane_window(index: u32, name: &str, pane_id: u32, content: Vec<String>) -> Window {
    Window {
        index,
        name: name.to_owned(),
        layout: Layout::Pane(PaneId(pane_id)),
        panes: vec![pane(pane_id, content)],
        active: PaneId(pane_id),
        last_active: None,
        zoomed: false,
    }
}

fn server(windows: Vec<Window>, current: u32, last: Option<u32>) -> Server {
    let next_pane = windows
        .iter()
        .flat_map(|w| w.panes.iter().map(|p| p.id.0))
        .max()
        .map_or(0, |m| m.saturating_add(1));
    Server {
        sessions: vec![Session {
            name: "0".to_owned(),
            windows,
            current,
            last,
        }],
        attached: Some(0),
        next_pane,
        message: None,
    }
}

/// Two panes, p0 first (left/top) and p1 second; `active` is 0 or 1 and the
/// other pane is the last-active one, as after a split or a pane move.
fn two_panes(dir: SplitDir, active: u32) -> Server {
    let window = Window {
        index: 0,
        name: "bash".to_owned(),
        layout: split(dir, &[0, 1]),
        panes: vec![pane(0, shell()), pane(1, prompt_only())],
        active: PaneId(active),
        last_active: Some(PaneId(1 - active.min(1))),
        zoomed: false,
    };
    server(vec![window], 0, None)
}

/// 2×2: p0 top-left, p1 bottom-left, p2 top-right, p3 bottom-right (tmux pane
/// index order); active = bottom-right, last = top-left.
fn four_panes() -> Server {
    let window = Window {
        index: 0,
        name: "bash".to_owned(),
        layout: Layout::Split {
            dir: SplitDir::H,
            children: vec![split(SplitDir::V, &[0, 1]), split(SplitDir::V, &[2, 3])],
        },
        panes: vec![
            pane(0, shell()),
            pane(1, prompt_only()),
            pane(2, prompt_only()),
            pane(3, prompt_only()),
        ],
        active: PaneId(3),
        last_active: Some(PaneId(0)),
        zoomed: false,
    };
    server(vec![window], 0, None)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::layout::{geometry, Rect};
    use crate::state::PaneId;

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        w: 80,
        h: 23,
    };

    #[test]
    fn all_fixtures_are_consistent() {
        for name in NAMES {
            let s = get(name).unwrap();
            assert_eq!(s.attached, Some(0), "{name}");
            let session = &s.sessions[0];
            assert_eq!(session.name, "0");
            for w in &session.windows {
                let ids: Vec<PaneId> = geometry(&w.layout, AREA)
                    .iter()
                    .map(|(id, _)| *id)
                    .collect();
                let mut pane_ids: Vec<PaneId> = w.panes.iter().map(|p| p.id).collect();
                pane_ids.sort();
                let mut sorted = ids.clone();
                sorted.sort();
                assert_eq!(sorted, pane_ids, "{name}");
                assert!(ids.contains(&w.active), "{name}");
                assert!(s.next_pane > w.panes.iter().map(|p| p.id.0).max().unwrap());
            }
            assert!(session.current_window().is_some(), "{name}");
        }
        assert!(get("nope").is_none());
    }

    #[test]
    fn fixture_specifics() {
        let active = |n: &str| get(n).unwrap().sessions[0].current_window().unwrap().active;
        assert_eq!(active("two-panes-h"), PaneId(1));
        assert_eq!(active("two-panes-h-first"), PaneId(0));
        assert_eq!(active("two-panes-v"), PaneId(1));
        assert_eq!(active("two-panes-v-first"), PaneId(0));
        let four = get("four-panes").unwrap();
        let w = four.sessions[0].current_window().unwrap();
        assert_eq!(w.active, PaneId(3));
        assert_eq!(w.last_active, Some(PaneId(0)));
        assert_eq!(
            geometry(&w.layout, AREA)[3].1,
            Rect {
                x: 41,
                y: 12,
                w: 39,
                h: 11
            }
        );
        let three = get("three-windows").unwrap();
        let names: Vec<(u32, &str)> = three.sessions[0]
            .windows
            .iter()
            .map(|w| (w.index, w.name.as_str()))
            .collect();
        assert_eq!(names, vec![(0, "bash"), (1, "logs"), (2, "editor")]);
        assert_eq!(
            (three.sessions[0].current, three.sessions[0].last),
            (1, Some(0))
        );
        let one = get("one-pane").unwrap();
        assert_eq!(one.sessions[0].windows[0].name, "bash");
        assert_eq!(one.sessions[0].windows[0].panes.len(), 1);
    }
}
