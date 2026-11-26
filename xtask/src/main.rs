//! Repository tool, run with `cargo xtask <task>` (alias defined in .cargo/config.toml).
//!
//! Tasks:
//! - `check-layers`: checks the layer rule of the workspace (see `layers.rs`);
//! - `coverage`: the test coverage of each crate, from a summary of cargo-llvm-cov (see
//!   `coverage.rs`);
//! - `gen-data`: writes a data file for the S1 storage trial (see `gen_data.rs`), or, with
//!   `--model annuaire`, a directory application of a given number of persons (see
//!   `gen_annuaire.rs`);
//! - `gen-schemas`: writes the JSON Schemas of the configuration files (see `schemas.rs`).

mod coverage;
mod gen_annuaire;
mod gen_data;
mod layers;
mod schemas;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand};

use layers::{DepKind, Graph};

const USAGE: &str = "usage: cargo xtask <task>

tasks:
  check-layers   check the layer rule of the workspace
  coverage FILE  print the test coverage of each crate as a Markdown table, from the summary
                 written by `cargo llvm-cov report --json --summary-only --output-path FILE`
  gen-data       write a data file for the S1 storage trial, or a directory application with
                 --model annuaire (see `cargo xtask gen-data --help`)
  gen-schemas    write the JSON Schemas of app.json and of the model files in schemas/;
                 with --check, only check that they are up to date";

fn main() -> ExitCode {
    // Skip the program name, like argv[0] in C.
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check-layers") if args.len() == 1 => check_layers(),
        Some("coverage") if args.len() == 2 => coverage(Path::new(&args[1])),
        Some("gen-data") => gen_data(&args[1..]),
        Some("gen-schemas") if args.len() == 1 => gen_schemas(),
        Some("gen-schemas") if args.len() == 2 && args[1] == "--check" => check_schemas(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn coverage(summary: &Path) -> ExitCode {
    let export = match std::fs::read_to_string(summary)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
    {
        Ok(export) => export,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", summary.display());
            return ExitCode::FAILURE;
        }
    };
    let metadata = match MetadataCommand::new().no_deps().exec() {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("error: cannot read the workspace metadata: {error}");
            return ExitCode::FAILURE;
        }
    };
    // Each crate with the folder of its Cargo.toml, by name.
    let mut crates: Vec<(String, PathBuf)> = metadata
        .workspace_packages()
        .into_iter()
        .filter_map(|package| {
            let dir = package.manifest_path.parent()?;
            Some((package.name.to_string(), dir.as_std_path().to_owned()))
        })
        .collect();
    crates.sort();
    match coverage::summarize(&export, &crates) {
        Ok(rows) => {
            print!("{}", coverage::render(&rows));
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {}: {message}", summary.display());
            ExitCode::FAILURE
        }
    }
}

fn gen_data(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}\n\n{}", gen_data::USAGE, gen_annuaire::USAGE);
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--model") {
        return gen_annuaire(args);
    }
    let options = match gen_data::parse_args(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("error: {message}\n\n{}", gen_data::USAGE);
            return ExitCode::from(2);
        }
    };
    match gen_data::run(&options) {
        Ok(size) => {
            println!("gen-data: {} ({size} bytes)", options.out.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: cannot write {}: {error}", options.out.display());
            ExitCode::FAILURE
        }
    }
}

fn gen_annuaire(args: &[String]) -> ExitCode {
    let options = match gen_annuaire::parse_args(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("error: {message}\n\n{}", gen_annuaire::USAGE);
            return ExitCode::from(2);
        }
    };
    match gen_annuaire::run(&options) {
        Ok(size) => {
            let persons = gen_annuaire::persons_file(&options.out);
            println!(
                "gen-data: {} ({} persons, {size} bytes)",
                persons.display(),
                options.count
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: cannot write {}: {error}", options.out.display());
            ExitCode::FAILURE
        }
    }
}

fn gen_schemas() -> ExitCode {
    match schemas::write(&schemas::repository_root()) {
        Ok(paths) => {
            for path in paths {
                println!("gen-schemas: {path}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: cannot write the schemas: {error}");
            ExitCode::FAILURE
        }
    }
}

fn check_schemas() -> ExitCode {
    let stale = schemas::stale(&schemas::repository_root());
    for path in &stale {
        println!("error: {path} is missing or out of date: run `cargo xtask gen-schemas`");
    }
    if stale.is_empty() {
        println!("gen-schemas: schemas up to date");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn check_layers() -> ExitCode {
    // Runs `cargo metadata` on the workspace that contains the current directory.
    // All features: an optional dependency enabled by a feature is checked too.
    let metadata = match MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()
    {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("error: cannot read the workspace metadata: {error}");
            return ExitCode::FAILURE;
        }
    };
    let graph = match graph_from_metadata(&metadata) {
        Ok(graph) => graph,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let violations = layers::check(&graph);
    for violation in &violations {
        println!("error: {violation}");
    }
    if violations.is_empty() {
        println!("check-layers: no violation");
        ExitCode::SUCCESS
    } else {
        println!("check-layers: {} violation(s)", violations.len());
        ExitCode::FAILURE
    }
}

/// Converts the resolved dependency graph of `cargo metadata` into the tool's own graph.
///
/// The resolved graph covers every target platform: a Windows-only dependency is also checked
/// when the tool runs on Linux.
fn graph_from_metadata(metadata: &Metadata) -> Result<Graph, String> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("cargo metadata returned no dependency graph")?;

    // One node per package, found by its opaque package id.
    let mut graph = Graph::new();
    let mut index_of = HashMap::new();
    for package in &metadata.packages {
        let name = package.name.to_string();
        let index = if metadata.workspace_members.contains(&package.id) {
            graph.add_member(&name)
        } else {
            graph.add_external(&name)
        };
        index_of.insert(&package.id, index);
    }

    for node in &resolve.nodes {
        let from = index_of[&node.id];
        for dep in &node.deps {
            let to = index_of[&dep.pkg];
            // A dependency can be declared several times (normal and dev, or per platform):
            // keep each kind once.
            let mut kinds: Vec<DepKind> = Vec::new();
            for info in &dep.dep_kinds {
                let kind = match info.kind {
                    DependencyKind::Development => DepKind::Dev,
                    DependencyKind::Build => DepKind::Build,
                    // `Unknown` only exists for kinds added by future Cargo versions.
                    _ => DepKind::Normal,
                };
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
            for kind in kinds {
                graph.add_dep(from, to, kind);
            }
        }
    }
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real workspace respects its layers (scenario "Le workspace livré respecte ses couches").
    #[test]
    fn real_workspace_has_no_violation() {
        let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml");
        let metadata = MetadataCommand::new()
            .manifest_path(manifest)
            .features(CargoOpt::AllFeatures)
            .exec()
            .expect("cargo metadata");
        let graph = graph_from_metadata(&metadata).expect("graph");
        let violations = layers::check(&graph);
        assert!(violations.is_empty(), "{violations:#?}");
    }
}
