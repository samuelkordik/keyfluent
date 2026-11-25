//! Pane geometry and directional neighbours.

use crate::state::{Layout, PaneId, SplitDir};

/// A rectangle of cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    /// Whether (x, y) lies inside the rectangle.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        let (x, y) = (u32::from(x), u32::from(y));
        x >= u32::from(self.x)
            && x < u32::from(self.x) + u32::from(self.w)
            && y >= u32::from(self.y)
            && y < u32::from(self.y) + u32::from(self.h)
    }
}

/// Direction of a pane move (`select-pane -L/-R/-U/-D`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// Every pane's rectangle within `area`, in layout order (tmux pane index order).
///
/// A split of size `n` children puts one border cell between children and shares
/// the rest out so sizes differ by at most one, the earlier children getting the
/// extra cells (80 wide → 40 | border | 39). If the area is too small, children get
/// size 0. Never panics.
pub fn geometry(layout: &Layout, area: Rect) -> Vec<(PaneId, Rect)> {
    let mut out = Vec::new();
    place(layout, area, &mut out);
    out
}

fn place(layout: &Layout, area: Rect, out: &mut Vec<(PaneId, Rect)>) {
    match layout {
        Layout::Pane(id) => out.push((*id, area)),
        Layout::Split { dir, children } => {
            let total = match dir {
                SplitDir::H => area.w,
                SplitDir::V => area.h,
            };
            let mut offset: u16 = 0;
            for (child, size) in children.iter().zip(split_sizes(total, children.len())) {
                let rect = match dir {
                    SplitDir::H => Rect {
                        x: area.x.saturating_add(offset),
                        w: size,
                        ..area
                    },
                    SplitDir::V => Rect {
                        y: area.y.saturating_add(offset),
                        h: size,
                        ..area
                    },
                };
                place(child, rect, out);
                offset = offset.saturating_add(size).saturating_add(1);
            }
        }
    }
}

/// Sizes of `n` children sharing `total` cells with one border between each pair.
fn split_sizes(total: u16, n: usize) -> Vec<u16> {
    let Some(borders) = n.checked_sub(1) else {
        return Vec::new();
    };
    let avail = usize::from(total).saturating_sub(borders);
    let base = avail / n;
    let extra = avail % n;
    (0..n)
        .map(|i| u16::try_from(base + usize::from(i < extra)).unwrap_or(u16::MAX))
        .collect()
}

/// The pane next to `from` in direction `dir`, or `None` at the window edge
/// (see [`neighbour_wrapping`] for tmux's wrap-around). Among several adjacent
/// panes the one sharing the longest edge with `from` wins, the first in layout
/// order on a tie (tmux picks the most recently used; the model keeps no such
/// history).
pub fn neighbour(layout: &Layout, area: Rect, from: PaneId, dir: Dir) -> Option<PaneId> {
    find_neighbour(layout, area, from, dir, false)
}

/// Like [`neighbour`], but at the window edge the search continues from the
/// opposite edge, as tmux's `select-pane -L/-R/-U/-D` do: moving left from the
/// leftmost pane reaches the rightmost pane beside it. `None` only if `from` is
/// not in the layout or is the only pane in that row/column.
pub fn neighbour_wrapping(layout: &Layout, area: Rect, from: PaneId, dir: Dir) -> Option<PaneId> {
    find_neighbour(layout, area, from, dir, false)
        .or_else(|| find_neighbour(layout, area, from, dir, true))
}

/// Edges of a rectangle as `(left, right, top, bottom)`, right/bottom exclusive.
fn span(r: &Rect) -> (u32, u32, u32, u32) {
    (
        u32::from(r.x),
        u32::from(r.x) + u32::from(r.w),
        u32::from(r.y),
        u32::from(r.y) + u32::from(r.h),
    )
}

/// The best pane beside `from` in direction `dir`. With `wrap`, "beside" means
/// touching the opposite edge of `area` instead of `from`'s own edge.
fn find_neighbour(
    layout: &Layout,
    area: Rect,
    from: PaneId,
    dir: Dir,
    wrap: bool,
) -> Option<PaneId> {
    let geo = geometry(layout, area);
    let (_, f) = geo.iter().find(|(id, _)| *id == from)?;
    let (fx0, fx1, fy0, fy1) = span(f);
    let (ax0, ax1, ay0, ay1) = span(&area);
    let mut best: Option<(PaneId, u32)> = None;
    for (id, r) in geo.iter().filter(|(id, _)| *id != from) {
        let (x0, x1, y0, y1) = span(r);
        let rows = y1.min(fy1).saturating_sub(y0.max(fy0));
        let cols = x1.min(fx1).saturating_sub(x0.max(fx0));
        let (adjacent, overlap) = match (dir, wrap) {
            (Dir::Left, false) => (x1 + 1 == fx0, rows),
            (Dir::Right, false) => (fx1 + 1 == x0, rows),
            (Dir::Up, false) => (y1 + 1 == fy0, cols),
            (Dir::Down, false) => (fy1 + 1 == y0, cols),
            (Dir::Left, true) => (x1 == ax1, rows),
            (Dir::Right, true) => (x0 == ax0, rows),
            (Dir::Up, true) => (y1 == ay1, cols),
            (Dir::Down, true) => (y0 == ay0, cols),
        };
        if adjacent && overlap > 0 && best.is_none_or(|(_, o)| overlap > o) {
            best = Some((*id, overlap));
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::state::{Layout, PaneId, SplitDir};

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        w: 80,
        h: 23,
    };

    fn layout(name: &str) -> Layout {
        let s = fixtures::get(name).unwrap();
        s.sessions[0].current_window().unwrap().layout.clone()
    }

    #[test]
    fn two_panes_h_geometry() {
        assert_eq!(
            geometry(&layout("two-panes-h"), AREA),
            vec![
                (
                    PaneId(0),
                    Rect {
                        x: 0,
                        y: 0,
                        w: 40,
                        h: 23
                    }
                ),
                (
                    PaneId(1),
                    Rect {
                        x: 41,
                        y: 0,
                        w: 39,
                        h: 23
                    }
                ),
            ]
        );
    }

    #[test]
    fn two_panes_v_geometry() {
        assert_eq!(
            geometry(&layout("two-panes-v"), AREA),
            vec![
                (
                    PaneId(0),
                    Rect {
                        x: 0,
                        y: 0,
                        w: 80,
                        h: 11
                    }
                ),
                (
                    PaneId(1),
                    Rect {
                        x: 0,
                        y: 12,
                        w: 80,
                        h: 11
                    }
                ),
            ]
        );
    }

    #[test]
    fn three_way_split_sizes_differ_by_at_most_one() {
        let l = Layout::Split {
            dir: SplitDir::H,
            children: (0..3).map(|i| Layout::Pane(PaneId(i))).collect(),
        };
        let g = geometry(&l, AREA);
        let ws: Vec<u16> = g.iter().map(|(_, r)| r.w).collect();
        let xs: Vec<u16> = g.iter().map(|(_, r)| r.x).collect();
        assert_eq!(ws, vec![26, 26, 26]);
        assert_eq!(xs, vec![0, 27, 54]);
        let g = geometry(
            &l,
            Rect {
                x: 0,
                y: 0,
                w: 81,
                h: 23,
            },
        );
        let ws: Vec<u16> = g.iter().map(|(_, r)| r.w).collect();
        assert_eq!(ws, vec![27, 26, 26]);
    }

    #[test]
    fn tiny_area_does_not_panic() {
        let l = Layout::Split {
            dir: SplitDir::V,
            children: (0..10).map(|i| Layout::Pane(PaneId(i))).collect(),
        };
        let g = geometry(
            &l,
            Rect {
                x: 0,
                y: 0,
                w: 3,
                h: 2,
            },
        );
        assert_eq!(g.len(), 10);
        let g = geometry(
            &l,
            Rect {
                x: u16::MAX,
                y: u16::MAX,
                w: u16::MAX,
                h: u16::MAX,
            },
        );
        assert_eq!(g.len(), 10);
        let empty = Layout::Split {
            dir: SplitDir::H,
            children: vec![],
        };
        assert!(geometry(&empty, AREA).is_empty());
    }

    #[test]
    fn four_panes_neighbours() {
        let l = layout("four-panes");
        let tl = PaneId(0);
        assert_eq!(neighbour(&l, AREA, tl, Dir::Right), Some(PaneId(2)));
        assert_eq!(neighbour(&l, AREA, tl, Dir::Down), Some(PaneId(1)));
        assert_eq!(neighbour(&l, AREA, tl, Dir::Left), None);
        assert_eq!(neighbour(&l, AREA, tl, Dir::Up), None);
        assert_eq!(neighbour(&l, AREA, PaneId(3), Dir::Up), Some(PaneId(2)));
        assert_eq!(neighbour(&l, AREA, PaneId(3), Dir::Left), Some(PaneId(1)));
        assert_eq!(neighbour(&l, AREA, PaneId(9), Dir::Left), None);
    }

    #[test]
    fn neighbour_prefers_largest_overlap() {
        // Left pane full height; right column split in two: from the left
        // pane, Right reaches the top-right pane (equal overlap, first wins).
        let l = Layout::Split {
            dir: SplitDir::H,
            children: vec![
                Layout::Pane(PaneId(0)),
                Layout::Split {
                    dir: SplitDir::V,
                    children: vec![Layout::Pane(PaneId(1)), Layout::Pane(PaneId(2))],
                },
            ],
        };
        assert_eq!(neighbour(&l, AREA, PaneId(0), Dir::Right), Some(PaneId(1)));
        assert_eq!(neighbour(&l, AREA, PaneId(2), Dir::Left), Some(PaneId(0)));
    }

    #[test]
    fn neighbour_wraps_at_the_edge() {
        let l = layout("two-panes-h");
        assert_eq!(neighbour(&l, AREA, PaneId(1), Dir::Right), None);
        assert_eq!(
            neighbour_wrapping(&l, AREA, PaneId(1), Dir::Right),
            Some(PaneId(0))
        );
        assert_eq!(
            neighbour_wrapping(&l, AREA, PaneId(0), Dir::Left),
            Some(PaneId(1))
        );
        // No pane above or below in a side-by-side split.
        assert_eq!(neighbour_wrapping(&l, AREA, PaneId(0), Dir::Up), None);
        let four = layout("four-panes");
        assert_eq!(
            neighbour_wrapping(&four, AREA, PaneId(3), Dir::Down),
            Some(PaneId(2))
        );
        assert_eq!(
            neighbour_wrapping(&four, AREA, PaneId(1), Dir::Left),
            Some(PaneId(3))
        );
        assert_eq!(
            neighbour_wrapping(&layout("one-pane"), AREA, PaneId(0), Dir::Left),
            None
        );
        assert_eq!(neighbour_wrapping(&four, AREA, PaneId(9), Dir::Left), None);
    }
}
