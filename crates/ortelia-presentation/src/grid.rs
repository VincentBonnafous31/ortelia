//! A flow grid: cells placed from left to right, row by row, in columns of which some grow, and
//! the widths of the columns computed from the minimum widths of the cells and the width
//! available.
//!
//! The grid knows nothing of what its cells hold, nor of fonts: the user interface measures each
//! cell and gives its minimum width, indexed like the cells. The contents live with the user of
//! the grid (the form), in a vector of the same order. A `Grid<T>` that holds them would be the
//! "generic class" answer of Java; two vectors of the same index are enough, and the grid serves
//! any widget as it is.

/// What a cell asks of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpec {
    /// The number of columns, from 1 to the number of columns of the grid.
    pub span: usize,
    /// Starts a new row, unless the current row is still empty.
    pub new_row: bool,
}

/// The place of a cell: its row, its first column, and the number of its columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub row: usize,
    pub col: usize,
    pub span: usize,
}

/// The cells of a grid, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// The weight of each column, finite and positive or zero; at least one column.
    grow: Vec<f32>,
    /// In the order of the cells.
    cells: Vec<Placement>,
}

/// The widths of the columns of a grid, in points.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnWidths {
    /// One per column.
    pub widths: Vec<f32>,
    /// The left edge of each column, from the left edge of the grid, spacings included.
    pub positions: Vec<f32>,
    /// The width of the whole grid, spacings included.
    pub total: f32,
}

impl ColumnWidths {
    /// The left edge and the width of `span` columns from `col`, spacings between them included.
    pub fn span(&self, col: usize, span: usize) -> (f32, f32) {
        let last = col + span - 1;
        let left = self.positions[col];
        (left, self.positions[last] + self.widths[last] - left)
    }
}

impl Grid {
    /// Places the cells in a flow, in one pass: a cell goes to the next row if it asks for a new
    /// row and the current row has a cell already, or if it does not fit in what is left of the
    /// current row. The spans are between 1 and the number of columns (checked by
    /// `ortelia-config`): a cell always fits in an empty row.
    pub fn new(grow: Vec<f32>, specs: &[CellSpec]) -> Grid {
        let columns = grow.len();
        let (mut row, mut col) = (0, 0);
        let cells = specs
            .iter()
            .map(|spec| {
                let span = spec.span.clamp(1, columns.max(1));
                if (spec.new_row && col > 0) || col + span > columns {
                    row += 1;
                    col = 0;
                }
                let placement = Placement { row, col, span };
                col += span;
                placement
            })
            .collect();
        Grid { grow, cells }
    }

    /// The cells, placed, in their order.
    pub fn cells(&self) -> &[Placement] {
        &self.cells
    }

    /// The number of rows: the row of the last cell, plus one.
    pub fn rows(&self) -> usize {
        self.cells.last().map_or(0, |last| last.row + 1)
    }

    pub fn columns(&self) -> usize {
        self.grow.len()
    }

    /// The weight of each column.
    pub fn grow(&self) -> &[f32] {
        &self.grow
    }

    /// The minimum width of each column, from `cell_min`, the minimum width of each cell (in
    /// the order of the cells), and `spacing`, the space between two columns:
    /// 1. the largest minimum of its cells of span 1;
    /// 2. then, for each cell of a larger span, in order, what it lacks is added to the columns
    ///    of its span that grow, in proportion to their weights, or else to its last column.
    pub fn min_widths(&self, cell_min: &[f32], spacing: f32) -> Vec<f32> {
        let mut mins = vec![0.0_f32; self.columns()];
        for (cell, &min) in self.cells.iter().zip(cell_min) {
            if cell.span == 1 {
                mins[cell.col] = mins[cell.col].max(min);
            }
        }
        for (cell, &min) in self.cells.iter().zip(cell_min) {
            if cell.span < 2 {
                continue;
            }
            let range = cell.col..cell.col + cell.span;
            let current: f32 =
                mins[range.clone()].iter().sum::<f32>() + spacing * (cell.span - 1) as f32;
            let lack = min - current;
            if lack <= 0.0 {
                continue;
            }
            let weight: f32 = self.grow[range.clone()].iter().sum();
            if weight > 0.0 {
                for c in range {
                    // The ratio first: below 1, it never overflows, even if the sum of the weights
                    // is infinite (each weight is finite, not their sum).
                    mins[c] += lack * (self.grow[c] / weight);
                }
            } else {
                mins[cell.col + cell.span - 1] += lack;
            }
        }
        mins
    }

    /// The widths of the columns for the width `available`: their minimums (see
    /// [`Grid::min_widths`]), and the width left, if any, shared by the columns that grow, in
    /// proportion to their weights. Without such a column, or without width left, each column
    /// keeps its minimum, and the grid may be wider than `available`.
    pub fn widths(&self, cell_min: &[f32], available: f32, spacing: f32) -> ColumnWidths {
        let mut widths = self.min_widths(cell_min, spacing);
        let spacings = spacing * self.columns().saturating_sub(1) as f32;
        let left = available - widths.iter().sum::<f32>() - spacings;
        let weight: f32 = self.grow.iter().sum();
        if left > 0.0 && weight > 0.0 {
            for (width, grow) in widths.iter_mut().zip(&self.grow) {
                *width += left * (grow / weight);
            }
        }
        let mut positions = Vec::with_capacity(widths.len());
        let mut x = 0.0;
        for width in &widths {
            positions.push(x);
            x += width + spacing;
        }
        ColumnWidths {
            total: widths.iter().sum::<f32>() + spacings,
            widths,
            positions,
        }
    }
}

#[cfg(test)]
mod tests;
