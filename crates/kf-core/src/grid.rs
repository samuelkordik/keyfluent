//! Character cell grid: the render target the simulators draw into.

/// Semantic style of a cell; the UI maps roles to colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleRole {
    Normal,
    Dim,
    Border,
    ActiveBorder,
    StatusBar,
    StatusCurrent,
    StatusMessage,
    Prompt,
    Selection,
    Indicator,
}

/// One character cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub role: StyleRole,
}

impl Cell {
    const BLANK: Cell = Cell {
        ch: ' ',
        role: StyleRole::Normal,
    };
}

/// A `cols` x `rows` grid of cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellGrid {
    pub cols: u16,
    pub rows: u16,
    cells: Vec<Cell>,
}

impl CellGrid {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cells: vec![Cell::BLANK; usize::from(cols) * usize::from(rows)],
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.cols && y < self.rows {
            Some(usize::from(y) * usize::from(self.cols) + usize::from(x))
        } else {
            None
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).and_then(|i| self.cells.get(i))
    }

    /// Writes `s` starting at (x, y), clipping at the grid edge. Never panics.
    pub fn put_str(&mut self, x: u16, y: u16, s: &str, role: StyleRole) {
        for (i, ch) in s.chars().enumerate() {
            let Some(cx) = u16::try_from(i)
                .ok()
                .and_then(|i| x.checked_add(i))
                .filter(|cx| *cx < self.cols)
            else {
                break;
            };
            if let Some(cell) = self.index(cx, y).and_then(|i| self.cells.get_mut(i)) {
                *cell = Cell { ch, role };
            }
        }
    }

    /// The characters of row `y` (empty if out of range).
    pub fn row_text(&self, y: u16) -> String {
        (0..self.cols)
            .filter_map(|x| self.get(x, y))
            .map(|c| c.ch)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_str_clips_at_right_edge() {
        let mut g = CellGrid::new(80, 24);
        g.put_str(78, 0, "abcdef", StyleRole::Normal);
        assert!(g.row_text(0).ends_with("ab"));
        assert_eq!(g.row_text(0).chars().count(), 80);
        assert_eq!(g.get(80, 0), None);
    }

    #[test]
    fn put_str_out_of_range_is_noop() {
        let mut g = CellGrid::new(4, 2);
        g.put_str(9, 9, "x", StyleRole::Dim);
        g.put_str(u16::MAX, 0, "xyz", StyleRole::Dim);
        assert_eq!(g.row_text(0), "    ");
        assert_eq!(g.row_text(5), "");
    }

    #[test]
    fn roles_are_stored() {
        let mut g = CellGrid::new(5, 1);
        g.put_str(1, 0, "hi", StyleRole::Prompt);
        assert_eq!(g.get(1, 0).map(|c| c.role), Some(StyleRole::Prompt));
        assert_eq!(g.get(0, 0).map(|c| c.role), Some(StyleRole::Normal));
    }
}
