//! Renders the attached client's view into a cell grid, like stock tmux 3.4.

use kf_core::grid::{CellGrid, StyleRole};

use crate::layout::{geometry, Rect};
use crate::state::{Pane, PaneId, Server, Session, Window};

/// Client-side state drawn over the status line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overlay {
    /// The prefix key was pressed: show ` ^B ` at the right end of the status line.
    pub prefix_pending: bool,
    /// An open prompt: (label, input), e.g. `("(rename-window) ", "logs")`.
    pub prompt: Option<(String, String)>,
    /// A message replacing the status line (takes precedence over `Server::message`).
    pub message: Option<String>,
}

/// tmux's default `status-left-length`.
const STATUS_LEFT_LENGTH: usize = 10;
const PREFIX_INDICATOR: &str = " ^B ";

/// Draws the attached session's current window and its status line into a
/// `cols` × `rows` grid (the last row is the status line). A detached server
/// shows `[exited]`, as tmux prints when the client leaves. Never panics.
pub fn render(server: &Server, overlay: &Overlay, cols: u16, rows: u16) -> CellGrid {
    let mut grid = CellGrid::new(cols, rows);
    let Some(status_y) = rows.checked_sub(1) else {
        return grid;
    };
    let Some(session) = server.attached_session() else {
        grid.put_str(0, 0, "[exited]", StyleRole::Normal);
        return grid;
    };
    if let Some(window) = session.current_window() {
        let area = Rect {
            x: 0,
            y: 0,
            w: cols,
            h: status_y,
        };
        draw_window(&mut grid, window, area);
    }
    draw_status_line(&mut grid, server, session, overlay, status_y);
    grid
}

fn draw_window(grid: &mut CellGrid, window: &Window, area: Rect) {
    if window.zoomed {
        if let Some(pane) = window.pane(window.active) {
            draw_pane(grid, pane, area);
        }
        return;
    }
    let geo = geometry(&window.layout, area);
    for (id, rect) in &geo {
        if let Some(pane) = window.pane(*id) {
            draw_pane(grid, pane, *rect);
        }
    }
    draw_borders(grid, &geo, window.active, area);
}

/// Draws the last lines of `pane` that fit, top-aligned and clipped to `rect`.
fn draw_pane(grid: &mut CellGrid, pane: &Pane, rect: Rect) {
    let skip = pane.lines.len().saturating_sub(usize::from(rect.h));
    for (y, line) in (rect.y..rect.y.saturating_add(rect.h)).zip(pane.lines.iter().skip(skip)) {
        for (x, ch) in (rect.x..rect.x.saturating_add(rect.w)).zip(line.chars()) {
            grid.put_char(x, y, ch, StyleRole::Normal);
        }
    }
}

/// Every cell of `area` outside all panes is a border. Its glyph joins the
/// neighbouring border cells (`│ ─ ┼ ├ ┤ ┬ ┴`); cells next to the active pane
/// use `ActiveBorder`.
fn draw_borders(grid: &mut CellGrid, geo: &[(PaneId, Rect)], active: PaneId, area: Rect) {
    let is_border = |x: i32, y: i32| -> bool {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return false;
        };
        area.contains(x, y) && !geo.iter().any(|(_, r)| r.contains(x, y))
    };
    let active_rect = geo.iter().find(|(id, _)| *id == active).map(|(_, r)| *r);
    let two_panes = two_pane_split(geo);
    for y in area.y..area.y.saturating_add(area.h) {
        for x in area.x..area.x.saturating_add(area.w) {
            let (xi, yi) = (i32::from(x), i32::from(y));
            if !is_border(xi, yi) {
                continue;
            }
            let glyph = border_glyph(
                is_border(xi, yi - 1),
                is_border(xi, yi + 1),
                is_border(xi - 1, yi),
                is_border(xi + 1, yi),
            );
            let is_active = active_rect.is_some_and(|a| {
                let first = a.x == area.x && a.y == area.y;
                match two_panes {
                    // tmux colours half the border when there are just two panes:
                    // the top half for the left pane, the bottom half for the right,
                    // the left half for the top pane, the right half for the bottom.
                    Some(TwoPanes::SideBySide) => {
                        let half = u32::from(y - area.y) <= u32::from(a.h) / 2;
                        half == first
                    }
                    Some(TwoPanes::Stacked) => {
                        let half = u32::from(x - area.x) <= u32::from(a.w) / 2;
                        half == first
                    }
                    None => touches(a, x, y),
                }
            });
            let role = if is_active {
                StyleRole::ActiveBorder
            } else {
                StyleRole::Border
            };
            grid.put_char(x, y, glyph, role);
        }
    }
}

enum TwoPanes {
    SideBySide,
    Stacked,
}

/// tmux's `screen_redraw_two_panes`: exactly two panes, and how they are split.
fn two_pane_split(geo: &[(PaneId, Rect)]) -> Option<TwoPanes> {
    match geo {
        [(_, a), (_, b)] if b.x != a.x => Some(TwoPanes::SideBySide),
        [(_, a), (_, b)] if b.y != a.y => Some(TwoPanes::Stacked),
        _ => None,
    }
}

/// Whether border cell (x, y) is on the ring of cells around rect `a`.
fn touches(a: Rect, x: u16, y: u16) -> bool {
    let (x, y) = (i64::from(x), i64::from(y));
    let (ax, ay) = (i64::from(a.x), i64::from(a.y));
    x >= ax - 1 && x <= ax + i64::from(a.w) && y >= ay - 1 && y <= ay + i64::from(a.h)
}

fn border_glyph(up: bool, down: bool, left: bool, right: bool) -> char {
    match (up, down, left, right) {
        (true, true, true, true) => '┼',
        (true, true, false, true) => '├',
        (true, true, true, false) => '┤',
        (false, true, true, true) => '┬',
        (true, false, true, true) => '┴',
        (false, true, false, true) => '┌',
        (false, true, true, false) => '┐',
        (true, false, false, true) => '└',
        (true, false, true, false) => '┘',
        (false, false, _, _) if left || right => '─',
        _ => '│',
    }
}

fn draw_status_line(
    grid: &mut CellGrid,
    server: &Server,
    session: &Session,
    overlay: &Overlay,
    y: u16,
) {
    let fill = |grid: &mut CellGrid, role: StyleRole| {
        for x in 0..grid.cols {
            grid.put_char(x, y, ' ', role);
        }
    };
    if let Some((label, input)) = &overlay.prompt {
        fill(grid, StyleRole::Prompt);
        grid.put_str(0, y, &format!("{label}{input}"), StyleRole::Prompt);
        return;
    }
    if let Some(message) = overlay.message.as_ref().or(server.message.as_ref()) {
        fill(grid, StyleRole::StatusMessage);
        grid.put_str(0, y, message, StyleRole::StatusMessage);
    } else {
        fill(grid, StyleRole::StatusBar);
        draw_window_list(grid, session, y);
    }
    if overlay.prefix_pending {
        let x = grid.cols.saturating_sub(4);
        grid.put_str(x, y, PREFIX_INDICATOR, StyleRole::Indicator);
    }
}

/// `[session] ` then each window as `#I:#W#{?window_flags,#{window_flags}, }`,
/// separated by a space (tmux 3.4 defaults).
fn draw_window_list(grid: &mut CellGrid, session: &Session, y: u16) {
    let left: String = format!("[{}] ", session.name)
        .chars()
        .take(STATUS_LEFT_LENGTH)
        .collect();
    grid.put_str(0, y, &left, StyleRole::StatusBar);
    let mut x = u16::try_from(left.chars().count()).unwrap_or(u16::MAX);
    let mut windows: Vec<&Window> = session.windows.iter().collect();
    windows.sort_by_key(|w| w.index);
    for (i, window) in windows.iter().enumerate() {
        if i > 0 {
            x = x.saturating_add(1);
        }
        let current = window.index == session.current;
        let mut flags = String::new();
        if current {
            flags.push('*');
        } else if session.last == Some(window.index) {
            flags.push('-');
        }
        if window.zoomed {
            flags.push('Z');
        }
        let label = format!("{}:{}{}", window.index, window.name, flags);
        let role = if current {
            StyleRole::StatusCurrent
        } else {
            StyleRole::StatusBar
        };
        grid.put_str(x, y, &label, role);
        let len = label.chars().count() + usize::from(flags.is_empty());
        x = x.saturating_add(u16::try_from(len).unwrap_or(u16::MAX));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::fixtures;
    use kf_core::grid::StyleRole;

    fn draw(name: &str, overlay: &Overlay) -> CellGrid {
        render(&fixtures::get(name).unwrap(), overlay, 80, 24)
    }

    fn role(g: &CellGrid, x: u16, y: u16) -> StyleRole {
        g.get(x, y).unwrap().role
    }

    fn ch(g: &CellGrid, x: u16, y: u16) -> char {
        g.get(x, y).unwrap().ch
    }

    #[test]
    fn three_windows_status_line() {
        let g = draw("three-windows", &Overlay::default());
        let status = g.row_text(23);
        assert!(
            status.starts_with("[0] 0:bash- 1:logs* 2:editor"),
            "{status:?}"
        );
        assert_eq!(role(&g, 0, 23), StyleRole::StatusBar);
        assert_eq!(role(&g, 79, 23), StyleRole::StatusBar);
        // "1:logs*" spans columns 12..=18.
        assert_eq!(role(&g, 11, 23), StyleRole::StatusBar);
        for x in 12..=18 {
            assert_eq!(role(&g, x, 23), StyleRole::StatusCurrent);
        }
        assert_eq!(role(&g, 19, 23), StyleRole::StatusBar);
    }

    #[test]
    fn flagless_windows_get_trailing_space() {
        let mut s = fixtures::get("three-windows").unwrap();
        s.sessions[0].last = None;
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(g.row_text(23).starts_with("[0] 0:bash  1:logs* 2:editor "));
    }

    #[test]
    fn two_panes_h_border() {
        let g = draw("two-panes-h", &Overlay::default());
        for y in 0..23 {
            assert_eq!(ch(&g, 40, y), '│');
            assert!(matches!(
                role(&g, 40, y),
                StyleRole::Border | StyleRole::ActiveBorder
            ));
        }
        // Two panes: tmux colours half the border, the bottom half for the right pane.
        assert_eq!(role(&g, 40, 0), StyleRole::Border);
        assert_eq!(role(&g, 40, 11), StyleRole::Border);
        assert_eq!(role(&g, 40, 12), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 40, 22), StyleRole::ActiveBorder);
        let g = draw("two-panes-h-first", &Overlay::default());
        assert_eq!(role(&g, 40, 11), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 40, 12), StyleRole::Border);
    }

    #[test]
    fn two_panes_v_border() {
        let g = draw("two-panes-v", &Overlay::default());
        assert_eq!(g.row_text(11), "─".repeat(80));
        assert_eq!(role(&g, 40, 11), StyleRole::Border);
        assert_eq!(role(&g, 41, 11), StyleRole::ActiveBorder);
        let g = draw("two-panes-v-first", &Overlay::default());
        assert_eq!(role(&g, 40, 11), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 41, 11), StyleRole::Border);
    }

    #[test]
    fn four_panes_junction_and_active_border() {
        let g = draw("four-panes", &Overlay::default());
        assert_eq!(ch(&g, 40, 11), '┼');
        assert_eq!(ch(&g, 0, 11), '─');
        assert_eq!(ch(&g, 40, 0), '│');
        // Active = bottom-right: its left and top borders are active.
        assert_eq!(role(&g, 40, 15), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 60, 11), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 40, 11), StyleRole::ActiveBorder);
        assert_eq!(role(&g, 40, 5), StyleRole::Border);
        assert_eq!(role(&g, 20, 11), StyleRole::Border);
    }

    #[test]
    fn t_junctions() {
        use crate::state::{Layout, PaneId, SplitDir};
        let mut s = fixtures::get("two-panes-h").unwrap();
        let w = &mut s.sessions[0].windows[0];
        w.layout = Layout::Split {
            dir: SplitDir::H,
            children: vec![
                Layout::Split {
                    dir: SplitDir::V,
                    children: vec![Layout::Pane(PaneId(0)), Layout::Pane(PaneId(2))],
                },
                Layout::Pane(PaneId(1)),
            ],
        };
        let g = render(&s, &Overlay::default(), 80, 24);
        assert_eq!(ch(&g, 40, 11), '┤');
        let w = &mut s.sessions[0].windows[0];
        w.layout = Layout::Split {
            dir: SplitDir::V,
            children: vec![
                Layout::Pane(PaneId(0)),
                Layout::Split {
                    dir: SplitDir::H,
                    children: vec![Layout::Pane(PaneId(1)), Layout::Pane(PaneId(2))],
                },
            ],
        };
        let g = render(&s, &Overlay::default(), 80, 24);
        assert_eq!(ch(&g, 40, 11), '┬');
    }

    #[test]
    fn pane_content_is_drawn_and_clipped() {
        let g = draw("one-pane", &Overlay::default());
        assert!(g.row_text(0).starts_with("user@host:~$ "));
        let mut s = fixtures::get("two-panes-h").unwrap();
        s.sessions[0].windows[0].panes[0].lines = vec!["x".repeat(100)];
        let g = render(&s, &Overlay::default(), 80, 24);
        assert_eq!(ch(&g, 39, 0), 'x');
        assert_eq!(ch(&g, 40, 0), '│');
        assert_eq!(ch(&g, 41, 0), 'u');
    }

    #[test]
    fn long_content_shows_last_lines() {
        let mut s = fixtures::get("one-pane").unwrap();
        s.sessions[0].windows[0].panes[0].lines = (0..30).map(|i| format!("line {i}")).collect();
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(g.row_text(0).starts_with("line 7 "));
        assert!(g.row_text(22).starts_with("line 29"));
    }

    #[test]
    fn zoomed_window_shows_only_active_pane() {
        let mut s = fixtures::get("two-panes-h").unwrap();
        s.sessions[0].windows[0].zoomed = true;
        s.sessions[0].windows[0].panes[1].lines = vec!["right pane".into()];
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(g.row_text(0).starts_with("right pane"));
        assert!(!g.row_text(5).contains('│'));
        assert!(
            g.row_text(23).starts_with("[0] 0:bash*Z "),
            "{:?}",
            g.row_text(23)
        );
    }

    #[test]
    fn prefix_indicator_at_right_end() {
        let o = Overlay {
            prefix_pending: true,
            ..Overlay::default()
        };
        let g = draw("one-pane", &o);
        assert!(g.row_text(23).ends_with(" ^B "));
        assert_eq!(role(&g, 77, 23), StyleRole::Indicator);
        let g = draw("one-pane", &Overlay::default());
        assert!(!g.row_text(23).contains("^B"));
    }

    #[test]
    fn prompt_replaces_status_line() {
        let o = Overlay {
            prompt: Some(("(rename-window) ".into(), "logs".into())),
            ..Overlay::default()
        };
        let g = draw("three-windows", &o);
        assert!(g.row_text(23).starts_with("(rename-window) logs "));
        assert_eq!(role(&g, 0, 23), StyleRole::Prompt);
        assert_eq!(role(&g, 79, 23), StyleRole::Prompt);
        assert!(!g.row_text(23).contains("[0]"));
    }

    #[test]
    fn message_replaces_status_line() {
        let o = Overlay {
            message: Some("unknown command: foo".into()),
            ..Overlay::default()
        };
        let g = draw("one-pane", &o);
        assert!(g.row_text(23).starts_with("unknown command: foo"));
        assert_eq!(role(&g, 0, 23), StyleRole::StatusMessage);
        assert_eq!(role(&g, 79, 23), StyleRole::StatusMessage);
        let mut s = fixtures::get("one-pane").unwrap();
        s.message = Some("from server".into());
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(g.row_text(23).starts_with("from server"));
    }

    #[test]
    fn long_session_name_is_truncated_like_status_left_length() {
        let mut s = fixtures::get("one-pane").unwrap();
        s.sessions[0].name = "averylongname".into();
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(
            g.row_text(23).starts_with("[averylong0:bash*"),
            "{:?}",
            g.row_text(23)
        );
    }

    #[test]
    fn detached_and_degenerate_sizes_do_not_panic() {
        let mut s = fixtures::get("four-panes").unwrap();
        for (c, r) in [
            (0, 0),
            (1, 1),
            (0, 24),
            (80, 0),
            (3, 2),
            (2, 3),
            (u16::MAX, 1),
        ] {
            let g = render(&s, &Overlay::default(), c, r);
            assert_eq!((g.cols, g.rows), (c, r));
        }
        s.attached = None;
        let g = render(&s, &Overlay::default(), 80, 24);
        assert!(g.row_text(0).starts_with("[exited]"));
        s.attached = Some(7);
        let _ = render(&s, &Overlay::default(), 80, 24);
    }
}
