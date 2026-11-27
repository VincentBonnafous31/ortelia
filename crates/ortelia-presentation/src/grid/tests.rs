//! Tests of the grid (specification `grid-layout`).

use super::*;

/// A cell of span `span`, on the current row.
fn cell(span: usize) -> CellSpec {
    CellSpec {
        span,
        new_row: false,
    }
}

/// A cell of span `span` that asks for a new row.
fn new_row(span: usize) -> CellSpec {
    CellSpec {
        span,
        new_row: true,
    }
}

/// The places of the cells, as `(row, col, span)`.
fn places(grid: &Grid) -> Vec<(usize, usize, usize)> {
    grid.cells()
        .iter()
        .map(|p| (p.row, p.col, p.span))
        .collect()
}

/// `true` if `a` and `b` are equal, to the rounding of `f32`.
fn close(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
}

/// Scenario "Colonne sans poids".
#[test]
fn column_without_weight() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1), cell(1)]);
    assert_eq!(grid.grow(), [0.0, 1.0]);
    let widths = grid.widths(&[50.0, 50.0], 300.0, 0.0);
    assert_eq!(widths.widths, [50.0, 250.0]);
}

/// Scenarios "Quatre cellules sur deux colonnes" and "Cellule sur trois colonnes".
#[test]
fn cells_in_a_flow() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1); 4]);
    assert_eq!(places(&grid), [(0, 0, 1), (0, 1, 1), (1, 0, 1), (1, 1, 1)]);
    assert_eq!(grid.rows(), 2);
    let grid = Grid::new(vec![0.0; 4], &[cell(1), cell(3)]);
    assert_eq!(places(&grid), [(0, 0, 1), (0, 1, 3)]);
    assert_eq!(grid.rows(), 1);
}

/// Scenarios "Ligne pleine" and "Cellule qui ne tient plus".
#[test]
fn automatic_new_row() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1); 3]);
    assert_eq!(places(&grid)[2], (1, 0, 1));
    let grid = Grid::new(vec![0.0; 4], &[cell(1), cell(1), cell(1), cell(2)]);
    assert_eq!(places(&grid), [(0, 0, 1), (0, 1, 1), (0, 2, 1), (1, 0, 2)]);
}

/// Scenarios "Nouvelle ligne après une cellule" and "Nouvelle ligne après une ligne pleine";
/// a first cell that asks for a new row does not leave an empty row either.
#[test]
fn new_row_asked() {
    let grid = Grid::new(vec![0.0; 4], &[cell(1), new_row(1)]);
    assert_eq!(places(&grid), [(0, 0, 1), (1, 0, 1)]);
    let grid = Grid::new(vec![0.0, 1.0], &[cell(2), new_row(1)]);
    assert_eq!(places(&grid), [(0, 0, 2), (1, 0, 1)]);
    assert_eq!(grid.rows(), 2);
    let grid = Grid::new(vec![0.0, 1.0], &[new_row(1)]);
    assert_eq!(places(&grid), [(0, 0, 1)]);
    // No cell, no row.
    assert_eq!(Grid::new(vec![0.0], &[]).rows(), 0);
}

/// Scenario "Colonne des libellés".
#[test]
fn minimum_width_of_the_columns() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1); 4]);
    let cell_min = [60.0, 100.0, 80.0, 120.0];
    assert_eq!(grid.min_widths(&cell_min, 8.0), [80.0, 120.0]);
    assert_eq!(grid.widths(&cell_min, 0.0, 8.0).total, 208.0);
    // A column without cell of span 1 has the minimum 0.
    let grid = Grid::new(vec![0.0, 0.0, 0.0], &[cell(1)]);
    assert_eq!(grid.min_widths(&[30.0], 0.0), [30.0, 0.0, 0.0]);
}

/// Scenarios "Différence donnée à la colonne qui s'élargit" and "Différence donnée à la
/// dernière colonne"; a cell that its columns hold already, and a lack shared by weights.
#[test]
fn cell_wider_than_its_columns() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1), new_row(2)]);
    assert_eq!(grid.min_widths(&[60.0, 300.0], 10.0), [60.0, 230.0]);
    let grid = Grid::new(vec![0.0, 0.0], &[cell(1), cell(1), cell(2)]);
    assert_eq!(grid.min_widths(&[50.0, 50.0, 200.0], 0.0), [50.0, 150.0]);
    assert_eq!(grid.min_widths(&[50.0, 50.0, 90.0], 0.0), [50.0, 50.0]);
    let grid = Grid::new(vec![1.0, 3.0], &[cell(2)]);
    assert_eq!(grid.min_widths(&[100.0], 0.0), [25.0, 75.0]);
}

/// Scenarios "Colonne des valeurs élargie", "Deux colonnes de poids différents" and "Aucune
/// colonne qui s'élargit".
#[test]
fn available_width_shared() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1), cell(1)]);
    let widths = grid.widths(&[80.0, 120.0], 400.0, 8.0);
    assert_eq!(widths.widths, [80.0, 312.0]);
    assert_eq!(widths.positions, [0.0, 88.0]);
    assert_eq!(widths.total, 400.0);
    assert_eq!(widths.span(0, 2), (0.0, 400.0));
    assert_eq!(widths.span(1, 1), (88.0, 312.0));

    let grid = Grid::new(vec![1.0, 3.0], &[cell(1), cell(1)]);
    assert_eq!(
        grid.widths(&[50.0, 50.0], 300.0, 0.0).widths,
        [100.0, 200.0]
    );

    let grid = Grid::new(vec![0.0, 0.0], &[cell(1), cell(1)]);
    let widths = grid.widths(&[80.0, 100.0], 500.0, 10.0);
    assert_eq!(widths.widths, [80.0, 100.0]);
    assert_eq!(widths.total, 190.0);
}

/// Scenario "Vue trop étroite".
#[test]
fn available_width_too_small() {
    let grid = Grid::new(vec![0.0, 1.0], &[cell(1), cell(1)]);
    let widths = grid.widths(&[80.0, 150.0], 200.0, 10.0);
    assert_eq!(widths.widths, [80.0, 150.0]);
    assert_eq!(widths.total, 240.0);
}

/// On a few grids and widths: the widths are never below the minimums, and the grid takes the
/// whole width available when a column grows and the width suffices.
#[test]
fn widths_never_below_the_minimums() {
    let grids = [
        (
            vec![0.0, 1.0],
            vec![cell(1), cell(1), cell(2), cell(1), cell(1)],
        ),
        (
            vec![0.0, 1.0, 0.0, 2.0],
            vec![cell(2), cell(2), cell(1), cell(3), new_row(4)],
        ),
        (
            vec![0.0, 0.0, 0.0],
            vec![cell(1), cell(2), cell(3), cell(1)],
        ),
        (vec![0.5], vec![cell(1), new_row(1)]),
    ];
    for (grow, specs) in grids {
        let grid = Grid::new(grow, &specs);
        let cell_min: Vec<f32> = (0..specs.len()).map(|i| 20.0 + 37.0 * i as f32).collect();
        for spacing in [0.0, 6.0] {
            let mins = grid.min_widths(&cell_min, spacing);
            // Each cell fits in its columns, spacings included.
            for (place, min) in grid.cells().iter().zip(&cell_min) {
                let width: f32 = mins[place.col..place.col + place.span].iter().sum::<f32>()
                    + spacing * (place.span - 1) as f32;
                assert!(width + 1e-3 >= *min, "{place:?}: {width} < {min}");
            }
            let min_total = grid.widths(&cell_min, 0.0, spacing).total;
            for available in [0.0, 100.0, 400.0, 1000.0] {
                let widths = grid.widths(&cell_min, available, spacing);
                assert!(
                    widths.widths.iter().zip(&mins).all(|(w, m)| w + 1e-3 >= *m),
                    "{widths:?} {mins:?}"
                );
                let grows = grid.grow().iter().any(|&g| g > 0.0);
                if grows && available >= min_total {
                    assert!((widths.total - available).abs() < 1e-2, "{widths:?}");
                } else {
                    assert!(close(&widths.widths, &mins), "{widths:?}");
                }
            }
        }
    }
}

/// Weights each within the range of an `f32`, whose sum is not: the widths stay finite.
#[test]
fn huge_weights_stay_finite() {
    let grid = Grid::new(vec![f32::MAX, f32::MAX], &[cell(2)]);
    let widths = grid.widths(&[300.0], 100.0, 0.0);
    assert!(widths.widths.iter().all(|w| w.is_finite()), "{widths:?}");
    assert!(widths.total.is_finite(), "{widths:?}");
}
