//! Repository tool, run with `cargo xtask <task>` (alias defined in .cargo/config.toml).
//!
//! Tasks:
//! - `check-layers`: checks the layer rule of the workspace (see `layers.rs`).

mod layers;

use std::collections::HashMap;
use std::process::ExitCode;

use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand};

use layers::{DepKind, Graph};

const USAGE: &str =
    "usage: cargo xtask <task>\n\ntasks:\n  check-layers   check the layer rule of the workspace";

fn main() -> ExitCode {
    // Skip the program name, like argv[0] in C.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["check-layers"] {
        check_layers()
    } else {
        eprintln!("{USAGE}");
        ExitCode::from(2)
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
