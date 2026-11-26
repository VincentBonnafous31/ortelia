//! Measure of the table view on a directory application, for the thresholds of the
//! specification (sort and filter under 200 ms, preparation of the filter under a second).
//!
//! ```text
//! cargo xtask gen-data --model annuaire --count 100000 --out ~/annuaire-100k
//! cargo run --release -p ortelia-presentation --example table-bench -- ~/annuaire-100k
//! ```
//!
//! Each step goes through the presentation as the interface does: an intent, then the
//! `prepare` of the next frame, which is what is timed. Always in release: a debug build is
//! ten times slower.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ortelia_presentation::{AppState, Intent, MainContent, TableIntent};

/// A step of the measure: its name, its time, and the rows shown after it (none before the
/// first `prepare`).
struct Step {
    name: &'static str,
    time: Duration,
    rows: Option<usize>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder] = args.as_slice() else {
        eprintln!("usage: table-bench <folder of an application, or its app.json>");
        return ExitCode::from(2);
    };
    let path = app_file(Path::new(folder));
    match measure(&path) {
        Ok(steps) => {
            print!("{}", report(&steps));
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("table-bench: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

/// The `app.json` of a folder, or the file given.
fn app_file(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("app.json")
    } else {
        path.to_owned()
    }
}

/// Opens the application, then times the steps; the errors of the opening otherwise.
fn measure(path: &Path) -> Result<Vec<Step>, Vec<String>> {
    let mut steps = Vec::new();

    let start = Instant::now();
    let mut state = AppState::open(path);
    let opening = start.elapsed();
    if !state.open_errors().is_empty() {
        return Err(state.open_errors().to_vec());
    }
    steps.push(Step {
        name: "opening (reading of the files included)",
        time: opening,
        rows: None,
    });

    // The sort of `app.json` (`nom` then `prenom`), with the folding of their texts.
    steps.push(timed(
        &mut state,
        "initial sort (nom, prenom; keys folded)",
        None,
    ));
    // A click on `nom`: sorted on it alone, descending (it was ascending), keys kept.
    let click = Intent::Table(TableIntent::SortBy {
        column: 0,
        add: false,
    });
    steps.push(timed(&mut state, "sort by nom", Some(click)));
    // The first character typed builds the text of each row, then filters.
    let first = Intent::Table(TableIntent::SetFilter("d".into()));
    steps.push(timed(
        &mut state,
        "preparation of the filter (\"d\")",
        Some(first),
    ));
    let dup = Intent::Table(TableIntent::SetFilter("dup".into()));
    steps.push(timed(&mut state, "filter \"dup\"", Some(dup)));
    Ok(steps)
}

/// Dispatches `intent`, then times the `prepare` that follows.
fn timed(state: &mut AppState, name: &'static str, intent: Option<Intent>) -> Step {
    if let Some(intent) = intent {
        state.dispatch(intent);
    }
    let start = Instant::now();
    state.prepare();
    let time = start.elapsed();
    Step {
        name,
        time,
        rows: Some(rows(state)),
    }
}

/// The number of rows shown.
fn rows(state: &AppState) -> usize {
    match state.content() {
        MainContent::Table(table) => table.row_count(),
        MainContent::Nothing => 0,
    }
}

/// The steps as a Markdown table.
fn report(steps: &[Step]) -> String {
    let mut text = String::from("| Step | Time (ms) | Rows |\n|---|---:|---:|\n");
    for step in steps {
        let rows = step.rows.map_or("–".to_owned(), |rows| rows.to_string());
        text += &format!(
            "| {} | {:.1} | {rows} |\n",
            step.name,
            step.time.as_secs_f64() * 1000.0,
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measure runs on `examples/annuaire` (the times of a debug build mean nothing).
    #[test]
    fn measure_of_the_directory_example() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/annuaire");
        let steps = measure(&app_file(&folder)).unwrap_or_else(|errors| panic!("{errors:?}"));
        let rows: Vec<usize> = steps.iter().filter_map(|step| step.rows).collect();
        // 10 persons; `d` is in `Dupont`, `Durand`, `Richard`… and `dup` only in `Dupont`.
        assert_eq!(rows, [10, 10, rows[2], 1]);
        assert!(rows[2] < 10, "{rows:?}");
        let report = report(&steps);
        assert_eq!(report.lines().count(), 2 + steps.len(), "{report}");
        assert!(report.contains("filter \"dup\""), "{report}");
    }

    #[test]
    fn opening_errors_are_returned() {
        let dir = tempfile::tempdir().expect("temp dir");
        let errors = measure(&dir.path().join("app.json")).err().expect("errors");
        assert!(errors[0].contains("app.json"), "{errors:?}");
    }
}
