//! `cargo xtask coverage <file>`: the test coverage of each workspace crate, as a Markdown table.
//!
//! The input is the summary written by
//! `cargo llvm-cov report --json --summary-only --output-path <file>`: one entry per source file,
//! with its counts of lines, functions and regions (a region is a block of code that LLVM counts
//! separately: a branch of an `if`, a `match` arm, a closure…). This module adds up the files of
//! each crate's `src/` folder; the test folders (`tests/`) and the examples are left out, since
//! they are the tests themselves.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Covered and total count of one measure (lines, functions or regions).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub covered: u64,
    pub count: u64,
}

impl Counts {
    fn add(&mut self, other: Counts) {
        self.covered += other.covered;
        self.count += other.count;
    }

    /// `87.5 % (7/8)`, or a dash when there is nothing to cover.
    fn cell(self) -> String {
        if self.count == 0 {
            return "–".to_owned();
        }
        // Counts stay far below 2^53: the conversion to f64 is exact.
        let percent = self.covered as f64 * 100.0 / self.count as f64;
        format!("{percent:.1} % ({}/{})", self.covered, self.count)
    }
}

/// The coverage of one crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub lines: Counts,
    pub functions: Counts,
    pub regions: Counts,
}

/// Adds up the files of the summary by crate. `crates` gives each crate's name and folder (the
/// folder of its `Cargo.toml`); the rows come in the same order. A file outside every `src/`
/// folder is ignored.
pub fn summarize(export: &Value, crates: &[(String, PathBuf)]) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = crates
        .iter()
        .map(|(name, _)| Row {
            name: name.clone(),
            lines: Counts::default(),
            functions: Counts::default(),
            regions: Counts::default(),
        })
        .collect();

    let data = export["data"]
        .as_array()
        .ok_or("not a coverage summary of llvm-cov: no `data` array")?;
    for file in data
        .iter()
        .flat_map(|d| d["files"].as_array().into_iter().flatten())
    {
        let filename = file["filename"]
            .as_str()
            .ok_or("a file of the summary has no `filename`")?;
        let path = Path::new(filename);
        // The folders of two crates cannot be nested (`src/` of one would have to contain the
        // `Cargo.toml` of the other), so at most one crate matches.
        let Some(index) = crates
            .iter()
            .position(|(_, dir)| path.starts_with(dir.join("src")))
        else {
            continue;
        };
        let summary = &file["summary"];
        let row = &mut rows[index];
        row.lines.add(counts(&summary["lines"], filename)?);
        row.functions.add(counts(&summary["functions"], filename)?);
        row.regions.add(counts(&summary["regions"], filename)?);
    }
    Ok(rows)
}

fn counts(measure: &Value, filename: &str) -> Result<Counts, String> {
    let field = |name: &str| {
        measure[name]
            .as_u64()
            .ok_or_else(|| format!("{filename}: missing count `{name}` in the summary"))
    };
    Ok(Counts {
        covered: field("covered")?,
        count: field("count")?,
    })
}

/// The Markdown table: one row per crate, then the total.
pub fn render(rows: &[Row]) -> String {
    let mut total = Row {
        name: "**total**".to_owned(),
        lines: Counts::default(),
        functions: Counts::default(),
        regions: Counts::default(),
    };
    for row in rows {
        total.lines.add(row.lines);
        total.functions.add(row.functions);
        total.regions.add(row.regions);
    }

    let mut text = String::from(
        "## Test coverage by crate\n\n\
         Code of each crate's `src/` folder run by `cargo test --workspace` (the test modules \
         written in `src/` count as covered code).\n\n\
         | crate | lines | functions | regions |\n\
         |---|--:|--:|--:|\n",
    );
    for row in rows.iter().chain([&total]) {
        // Writing to a String cannot fail.
        let _ = writeln!(
            text,
            "| {} | {} | {} | {} |",
            row.name,
            row.lines.cell(),
            row.functions.cell(),
            row.regions.cell()
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file(name: &str, lines: (u64, u64), functions: (u64, u64), regions: (u64, u64)) -> Value {
        let measure = |(covered, count): (u64, u64)| json!({ "covered": covered, "count": count });
        json!({
            "filename": name,
            "summary": {
                "lines": measure(lines),
                "functions": measure(functions),
                "regions": measure(regions),
            }
        })
    }

    fn crates() -> Vec<(String, PathBuf)> {
        vec![
            ("a".to_owned(), PathBuf::from("/w/crates/a")),
            ("ab".to_owned(), PathBuf::from("/w/crates/ab")),
        ]
    }

    #[test]
    fn adds_up_the_src_files_of_each_crate() {
        let export = json!({ "data": [{ "files": [
            file("/w/crates/a/src/lib.rs", (6, 10), (1, 2), (3, 4)),
            file("/w/crates/a/src/sub/mod.rs", (4, 10), (1, 1), (1, 4)),
            // A crate whose name starts like another one is not mixed with it.
            file("/w/crates/ab/src/lib.rs", (1, 1), (1, 1), (1, 1)),
            // The tests, the examples and the dependencies are left out.
            file("/w/crates/a/tests/it.rs", (5, 5), (1, 1), (1, 1)),
            file("/w/crates/a/examples/demo.rs", (5, 5), (1, 1), (1, 1)),
            file("/home/u/.cargo/registry/src/x/lib.rs", (5, 5), (1, 1), (1, 1)),
        ]}]});
        let rows = summarize(&export, &crates()).unwrap();
        assert_eq!(rows[0].name, "a");
        assert_eq!(
            rows[0].lines,
            Counts {
                covered: 10,
                count: 20
            }
        );
        assert_eq!(
            rows[0].functions,
            Counts {
                covered: 2,
                count: 3
            }
        );
        assert_eq!(
            rows[0].regions,
            Counts {
                covered: 4,
                count: 8
            }
        );
        assert_eq!(
            rows[1].lines,
            Counts {
                covered: 1,
                count: 1
            }
        );
    }

    #[test]
    fn renders_one_row_per_crate_and_the_total() {
        let export = json!({ "data": [{ "files": [
            file("/w/crates/a/src/lib.rs", (7, 8), (1, 2), (0, 3)),
        ]}]});
        let rows = summarize(&export, &crates()).unwrap();
        let table = render(&rows);
        assert!(table.contains("| a | 87.5 % (7/8) | 50.0 % (1/2) | 0.0 % (0/3) |\n"));
        // A crate without code has nothing to cover.
        assert!(table.contains("| ab | – | – | – |\n"));
        assert!(table.ends_with("| **total** | 87.5 % (7/8) | 50.0 % (1/2) | 0.0 % (0/3) |\n"));
    }

    #[test]
    fn rejects_a_file_that_is_not_a_summary() {
        assert!(summarize(&json!({ "type": "other" }), &crates()).is_err());
        let missing = json!({ "data": [{ "files": [
            { "filename": "/w/crates/a/src/lib.rs", "summary": {} },
        ]}]});
        assert!(summarize(&missing, &crates()).is_err());
    }
}
