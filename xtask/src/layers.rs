//! The layer rule of the workspace (see docs/architecture-couches.md) and its check.
//!
//! This module knows nothing about `cargo metadata`: it works on a small dependency graph of
//! its own (`Graph`), which `main.rs` builds from the metadata and which tests build by hand.

use std::collections::VecDeque;
use std::fmt;

/// Workspace crates with a fixed layer. Crates named `ortelia-ui-*` are in layer 4.
const LAYERS: &[(&str, u8)] = &[
    ("ortelia-model", 0),
    ("ortelia-config", 1),
    ("ortelia-expr", 1),
    ("ortelia-service", 2),
    ("ortelia-presentation", 3),
    ("ortelia", 5),
];

/// Prefix of the user interface crates, all in layer 4.
const UI_CRATE_PREFIX: &str = "ortelia-ui-";
const UI_LAYER: u8 = 4;

/// Repository tools: workspace members that are not part of the application.
const TOOLS: &[&str] = &["xtask"];

/// UI libraries forbidden in layers 0 to 3, matched by name prefix
/// (`egui` also matches `egui_extras`, `egui-winit`, etc.).
const UI_LIBRARY_PREFIXES: &[&str] = &["egui", "eframe", "epaint", "winit", "gpui"];

/// Highest layer that must not depend on a UI library (the presentation layer).
const LAST_LAYER_WITHOUT_UI: u8 = 3;

/// Place of a workspace crate in the layer table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Layer(u8),
    Tool,
}

/// Looks a workspace crate up in the layer table; `None` if it is not classified.
fn place_of(name: &str) -> Option<Place> {
    if let Some(&(_, level)) = LAYERS.iter().find(|(n, _)| *n == name) {
        return Some(Place::Layer(level));
    }
    if name.starts_with(UI_CRATE_PREFIX) {
        return Some(Place::Layer(UI_LAYER));
    }
    if TOOLS.contains(&name) {
        return Some(Place::Tool);
    }
    None
}

fn is_ui_library(name: &str) -> bool {
    UI_LIBRARY_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Kind of a dependency, as declared in a `Cargo.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    /// `[dependencies]`
    Normal,
    /// `[dev-dependencies]`: tests, examples, benchmarks.
    Dev,
    /// `[build-dependencies]`: the build script.
    Build,
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DepKind::Normal => "dependency",
            DepKind::Dev => "dev-dependency",
            DepKind::Build => "build-dependency",
        };
        f.write_str(text)
    }
}

/// A dependency graph: each node is a package (a workspace member or a third-party crate).
///
/// Nodes are identified by their index in `nodes`, not by name: two versions of the same
/// third-party crate are two different nodes.
#[derive(Debug, Default)]
pub struct Graph {
    nodes: Vec<Node>,
}

#[derive(Debug)]
struct Node {
    name: String,
    /// True for a workspace member, false for a third-party crate.
    member: bool,
    deps: Vec<Dep>,
}

#[derive(Debug, Clone, Copy)]
struct Dep {
    node: usize,
    kind: DepKind,
}

impl Graph {
    pub fn new() -> Self {
        Graph::default()
    }

    /// Adds a workspace member and returns its index.
    pub fn add_member(&mut self, name: &str) -> usize {
        self.add_node(name, true)
    }

    /// Adds a third-party crate and returns its index.
    pub fn add_external(&mut self, name: &str) -> usize {
        self.add_node(name, false)
    }

    fn add_node(&mut self, name: &str, member: bool) -> usize {
        self.nodes.push(Node {
            name: name.to_string(),
            member,
            deps: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// Records that node `from` depends on node `to`.
    pub fn add_dep(&mut self, from: usize, to: usize, kind: DepKind) {
        self.nodes[from].deps.push(Dep { node: to, kind });
    }

    fn name(&self, node: usize) -> &str {
        &self.nodes[node].name
    }
}

/// A breach of the layer rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A workspace member missing from the layer table.
    Unclassified { krate: String },
    /// A dependency towards a workspace crate of the same or a higher layer.
    NotLower {
        from: String,
        from_level: u8,
        to: String,
        to_level: u8,
        kind: DepKind,
    },
    /// An application crate that depends on a repository tool.
    DependsOnTool {
        from: String,
        to: String,
        kind: DepKind,
    },
    /// A crate of layers 0 to 3 that reaches a UI library. `path` goes from the crate to the
    /// library, both included.
    UiLibrary { from: String, path: Vec<String> },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::Unclassified { krate } => write!(
                f,
                "{krate} is not in any layer: add it to the layer table in xtask/src/layers.rs \
                 (or declare it as a tool)"
            ),
            Violation::NotLower {
                from,
                from_level,
                to,
                to_level,
                kind,
            } => write!(
                f,
                "{from} (layer {from_level}) has a {kind} on {to} (layer {to_level}): \
                 a crate may only depend on lower layers"
            ),
            Violation::DependsOnTool { from, to, kind } => write!(
                f,
                "{from} has a {kind} on the tool crate {to}: application crates must not \
                 depend on repository tools"
            ),
            Violation::UiLibrary { from, path } => {
                let library = path.last().map(String::as_str).unwrap_or_default();
                write!(
                    f,
                    "{from} depends on the UI library {library} ({}): layers 0 to \
                     {LAST_LAYER_WITHOUT_UI} must not depend on a UI library",
                    path.join(" -> ")
                )
            }
        }
    }
}

/// Checks the whole graph and returns every violation found (empty if the rule holds).
pub fn check(graph: &Graph) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if !node.member {
            continue;
        }
        match place_of(&node.name) {
            None => violations.push(Violation::Unclassified {
                krate: node.name.clone(),
            }),
            // Tools may depend on anything.
            Some(Place::Tool) => {}
            Some(Place::Layer(level)) => {
                check_internal_deps(graph, index, level, &mut violations);
                if level <= LAST_LAYER_WITHOUT_UI {
                    check_no_ui_library(graph, index, &mut violations);
                }
            }
        }
    }
    violations
}

/// Rules on the dependencies between workspace crates: lower layers only, no tools.
fn check_internal_deps(graph: &Graph, from: usize, level: u8, out: &mut Vec<Violation>) {
    for dep in &graph.nodes[from].deps {
        let target = &graph.nodes[dep.node];
        if !target.member {
            continue;
        }
        match place_of(&target.name) {
            // An unclassified target is reported on its own.
            None => {}
            Some(Place::Tool) => out.push(Violation::DependsOnTool {
                from: graph.name(from).to_string(),
                to: target.name.clone(),
                kind: dep.kind,
            }),
            // A match arm with a condition (`if` guard): only same or higher layers match.
            Some(Place::Layer(target_level)) if target_level >= level => {
                out.push(Violation::NotLower {
                    from: graph.name(from).to_string(),
                    from_level: level,
                    to: target.name.clone(),
                    to_level: target_level,
                    kind: dep.kind,
                });
            }
            Some(Place::Layer(_)) => {}
        }
    }
}

/// Breadth-first search from `start` towards UI libraries. The parent of each visited node is
/// remembered, so that the dependency path can be printed. One violation is reported per UI
/// library reached. The search goes neither past a UI library (`egui -> epaint` would only
/// repeat the same problem) nor into another workspace crate of layers 0 to 3, which is
/// checked on its own: a problem is reported once, where it is, and not again by every
/// crate above it.
fn check_no_ui_library(graph: &Graph, start: usize, out: &mut Vec<Violation>) {
    // parent[n] = Some(p) once node n has been reached from node p.
    let mut parent: Vec<Option<usize>> = vec![None; graph.nodes.len()];
    let mut visited = vec![false; graph.nodes.len()];
    let mut queue = VecDeque::new();
    visited[start] = true;
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        for dep in &graph.nodes[node].deps {
            // The dev-dependencies of a dependency are not compiled when building `start`:
            // they only count for `start` itself.
            if node != start && dep.kind == DepKind::Dev {
                continue;
            }
            if visited[dep.node] {
                continue;
            }
            visited[dep.node] = true;
            parent[dep.node] = Some(node);
            if is_ui_library(graph.name(dep.node)) {
                out.push(Violation::UiLibrary {
                    from: graph.name(start).to_string(),
                    path: path_to(graph, &parent, dep.node),
                });
            } else if !is_checked_on_its_own(graph, dep.node) {
                queue.push_back(dep.node);
            }
        }
    }
}

/// True for a workspace crate of layers 0 to 3, whose own search covers its dependencies.
fn is_checked_on_its_own(graph: &Graph, node: usize) -> bool {
    let node = &graph.nodes[node];
    node.member
        && matches!(place_of(&node.name), Some(Place::Layer(level)) if level <= LAST_LAYER_WITHOUT_UI)
}

/// Rebuilds the path from the search start to `end` by following the parents back.
fn path_to(graph: &Graph, parent: &[Option<usize>], end: usize) -> Vec<String> {
    let mut path = vec![graph.name(end).to_string()];
    let mut current = end;
    while let Some(previous) = parent[current] {
        path.push(graph.name(previous).to_string());
        current = previous;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The internal graph of this change: every crate depends on the layers right below it.
    fn workspace() -> (Graph, Members) {
        let mut g = Graph::new();
        let m = Members {
            model: g.add_member("ortelia-model"),
            config: g.add_member("ortelia-config"),
            expr: g.add_member("ortelia-expr"),
            service: g.add_member("ortelia-service"),
            presentation: g.add_member("ortelia-presentation"),
            ui_egui: g.add_member("ortelia-ui-egui"),
            app: g.add_member("ortelia"),
            xtask: g.add_member("xtask"),
        };
        g.add_dep(m.config, m.model, DepKind::Normal);
        g.add_dep(m.expr, m.model, DepKind::Normal);
        g.add_dep(m.service, m.model, DepKind::Normal);
        g.add_dep(m.service, m.config, DepKind::Normal);
        g.add_dep(m.service, m.expr, DepKind::Normal);
        g.add_dep(m.presentation, m.service, DepKind::Normal);
        g.add_dep(m.ui_egui, m.presentation, DepKind::Normal);
        g.add_dep(m.app, m.presentation, DepKind::Normal);
        g.add_dep(m.app, m.ui_egui, DepKind::Normal);
        (g, m)
    }

    struct Members {
        model: usize,
        config: usize,
        expr: usize,
        service: usize,
        presentation: usize,
        ui_egui: usize,
        app: usize,
        xtask: usize,
    }

    #[test]
    fn workspace_of_this_change_is_valid() {
        let (g, _) = workspace();
        assert_eq!(check(&g), vec![]);
    }

    #[test]
    fn dependency_on_lower_layer_is_accepted() {
        let (mut g, m) = workspace();
        g.add_dep(m.presentation, m.model, DepKind::Normal);
        assert_eq!(check(&g), vec![]);
    }

    #[test]
    fn dependency_on_higher_layer_is_refused() {
        let (mut g, m) = workspace();
        g.add_dep(m.service, m.presentation, DepKind::Normal);
        let violations = check(&g);
        assert_eq!(
            violations,
            vec![Violation::NotLower {
                from: "ortelia-service".into(),
                from_level: 2,
                to: "ortelia-presentation".into(),
                to_level: 3,
                kind: DepKind::Normal,
            }]
        );
        let message = violations[0].to_string();
        assert!(message.contains("ortelia-service (layer 2)"), "{message}");
        assert!(
            message.contains("ortelia-presentation (layer 3)"),
            "{message}"
        );
    }

    #[test]
    fn dependency_on_same_layer_is_refused() {
        let (mut g, m) = workspace();
        g.add_dep(m.config, m.expr, DepKind::Normal);
        let violations = check(&g);
        assert_eq!(violations.len(), 1);
        let message = violations[0].to_string();
        assert!(message.contains("ortelia-config"), "{message}");
        assert!(message.contains("ortelia-expr"), "{message}");
    }

    #[test]
    fn dev_dependency_on_higher_layer_is_refused() {
        let (mut g, m) = workspace();
        g.add_dep(m.model, m.service, DepKind::Dev);
        let violations = check(&g);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            Violation::NotLower {
                kind: DepKind::Dev,
                ..
            }
        ));
        assert!(violations[0].to_string().contains("dev-dependency"));
    }

    #[test]
    fn dependency_on_tool_is_refused() {
        let (mut g, m) = workspace();
        g.add_dep(m.service, m.xtask, DepKind::Normal);
        let violations = check(&g);
        assert_eq!(
            violations,
            vec![Violation::DependsOnTool {
                from: "ortelia-service".into(),
                to: "xtask".into(),
                kind: DepKind::Normal,
            }]
        );
    }

    #[test]
    fn unclassified_crate_is_reported() {
        let (mut g, m) = workspace();
        let geo = g.add_member("ortelia-geo");
        g.add_dep(geo, m.model, DepKind::Normal);
        let violations = check(&g);
        assert_eq!(
            violations,
            vec![Violation::Unclassified {
                krate: "ortelia-geo".into()
            }]
        );
        assert!(
            violations[0]
                .to_string()
                .contains("ortelia-geo is not in any layer")
        );
    }

    #[test]
    fn direct_dependency_on_egui_is_refused() {
        let (mut g, m) = workspace();
        let egui = g.add_external("egui");
        g.add_dep(m.presentation, egui, DepKind::Normal);
        let violations = check(&g);
        assert_eq!(
            violations,
            vec![Violation::UiLibrary {
                from: "ortelia-presentation".into(),
                path: vec!["ortelia-presentation".into(), "egui".into()],
            }]
        );
        let message = violations[0].to_string();
        assert!(message.contains("ortelia-presentation"), "{message}");
        assert!(message.contains("egui"), "{message}");
    }

    #[test]
    fn transitive_dependency_on_egui_is_refused_with_its_path() {
        let (mut g, m) = workspace();
        let foo = g.add_external("foo");
        let egui = g.add_external("egui");
        g.add_dep(m.service, foo, DepKind::Normal);
        g.add_dep(foo, egui, DepKind::Normal);
        let violations = check(&g);
        // Reported once, for ortelia-service, and not again for ortelia-presentation above it.
        assert_eq!(
            violations,
            vec![Violation::UiLibrary {
                from: "ortelia-service".into(),
                path: vec!["ortelia-service".into(), "foo".into(), "egui".into()],
            }]
        );
        let message = violations[0].to_string();
        assert!(
            message.contains("ortelia-service -> foo -> egui"),
            "{message}"
        );
    }

    #[test]
    fn ui_crates_may_depend_on_egui() {
        let (mut g, m) = workspace();
        let egui = g.add_external("egui");
        let eframe = g.add_external("eframe");
        let winit = g.add_external("winit");
        g.add_dep(eframe, egui, DepKind::Normal);
        g.add_dep(eframe, winit, DepKind::Normal);
        g.add_dep(m.ui_egui, egui, DepKind::Normal);
        g.add_dep(m.ui_egui, eframe, DepKind::Normal);
        // The executable reaches egui through the UI crate: also allowed.
        assert_eq!(check(&g), vec![]);
    }

    #[test]
    fn dev_dependency_on_egui_is_refused() {
        let (mut g, m) = workspace();
        let kittest = g.add_external("egui_kittest");
        g.add_dep(m.presentation, kittest, DepKind::Dev);
        let violations = check(&g);
        assert_eq!(
            violations,
            vec![Violation::UiLibrary {
                from: "ortelia-presentation".into(),
                path: vec!["ortelia-presentation".into(), "egui_kittest".into()],
            }]
        );
    }

    #[test]
    fn dev_dependencies_of_dependencies_are_not_followed() {
        // ortelia-ui-egui is not checked on its own (layer 4), so a search can go through it.
        // Its dev-dependencies are not compiled when building a crate that depends on it.
        let (mut g, m) = workspace();
        let kittest = g.add_external("egui_kittest");
        g.add_dep(m.ui_egui, kittest, DepKind::Dev);
        g.add_dep(m.service, m.ui_egui, DepKind::Normal);
        let violations = check(&g);
        // Only the forbidden dependency itself is reported, not a path to egui_kittest.
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(&violations[0], Violation::NotLower { .. }));
    }

    #[test]
    fn several_violations_are_reported_together() {
        let (mut g, m) = workspace();
        let egui = g.add_external("egui");
        g.add_dep(m.service, m.presentation, DepKind::Normal);
        g.add_dep(m.model, egui, DepKind::Normal);
        let violations = check(&g);
        assert!(
            violations.iter().any(
                |v| matches!(v, Violation::NotLower { from, .. } if from == "ortelia-service")
            )
        );
        assert!(
            violations
                .iter()
                .any(|v| matches!(v, Violation::UiLibrary { from, .. } if from == "ortelia-model"))
        );
    }
}
