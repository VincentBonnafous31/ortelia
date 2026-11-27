//! The layout of the main window: a tree of splits whose leaves are views, built from the
//! content of `window` in `app.json`. The presentation alone knows the shares of the splits: the
//! user interface draws the tree, and reports the separators that the user moved.
//!
//! A Rust `enum` with data plays the part of a sealed hierarchy in Java (`sealed interface
//! LayoutNode permits SplitNode, ViewNode`) or of a `std::variant` in C++: a `match` must handle
//! every case, and the compiler points at the cases forgotten when a variant is added (tab
//! stacks, later).

use ortelia_config::{CheckedApp, CheckedContent, CheckedView, Direction};
use ortelia_model::metamodel::Model;

use crate::labels::LabelLookup;
use crate::table::TableView;

/// A node of the layout tree.
#[derive(Debug)]
pub enum LayoutNode {
    Split(SplitNode),
    View(ViewNode),
}

/// A split: its children side by side or one above the other, each with its share of the space.
#[derive(Debug)]
pub struct SplitNode {
    id: String,
    direction: Direction,
    children: Vec<LayoutNode>,
    /// One per child, adding up to 1.
    parts: Vec<f32>,
}

/// A view: a widget, named by the identifier of the view.
#[derive(Debug)]
pub struct ViewNode {
    id: String,
    pub(crate) content: ViewContent,
}

/// What a view holds.
#[derive(Debug)]
pub(crate) enum ViewContent {
    /// A `Box`, like a `std::unique_ptr` in C++: the table, some 300 bytes, is stored apart,
    /// so that every node of the tree does not take its size (an enum is as large as its
    /// largest variant).
    Table(Box<TableView>),
    Empty,
}

impl LayoutNode {
    /// The layout of `content`, a content of the window of `app`, with its tables. The labels
    /// are looked up once, here.
    pub(crate) fn build(
        app: &CheckedApp,
        content: &CheckedContent,
        model: &Model,
        lookup: &mut LabelLookup<'_>,
    ) -> LayoutNode {
        match content {
            CheckedContent::View(CheckedView::Table(view)) => LayoutNode::View(ViewNode {
                id: view.id.clone(),
                content: ViewContent::Table(Box::new(TableView::new(app, view, model, lookup))),
            }),
            CheckedContent::View(CheckedView::Empty { id }) => LayoutNode::View(ViewNode {
                id: id.clone(),
                content: ViewContent::Empty,
            }),
            CheckedContent::Split(split) => LayoutNode::Split(SplitNode {
                id: split.id.clone(),
                direction: split.direction,
                children: split
                    .children
                    .iter()
                    .map(|child| LayoutNode::build(app, child, model, lookup))
                    .collect(),
                // The weights were checked: one per child, each finite and strictly positive.
                parts: normalize_parts(&split.weights, split.children.len())
                    .expect("weights checked by ortelia-config"),
            }),
        }
    }

    /// The views of the tree, depth first, in the order of `app.json`.
    pub fn views(&self) -> Vec<&ViewNode> {
        let mut views = Vec::new();
        self.collect_views(&mut views);
        views
    }

    fn collect_views<'a>(&'a self, views: &mut Vec<&'a ViewNode>) {
        match self {
            LayoutNode::View(view) => views.push(view),
            LayoutNode::Split(split) => {
                for child in &split.children {
                    child.collect_views(views);
                }
            }
        }
    }

    /// The views of the tree, to change them. A mutable reference to each leaf: they do not
    /// overlap, so they can be held together, which the borrow checker accepts.
    pub(crate) fn views_mut(&mut self) -> Vec<&mut ViewNode> {
        let mut views = Vec::new();
        self.collect_views_mut(&mut views);
        views
    }

    fn collect_views_mut<'a>(&'a mut self, views: &mut Vec<&'a mut ViewNode>) {
        match self {
            LayoutNode::View(view) => views.push(view),
            LayoutNode::Split(split) => {
                for child in &mut split.children {
                    child.collect_views_mut(views);
                }
            }
        }
    }

    /// The splits of the tree, depth first, each before its children.
    pub fn splits(&self) -> Vec<&SplitNode> {
        let mut splits = Vec::new();
        self.collect_splits(&mut splits);
        splits
    }

    fn collect_splits<'a>(&'a self, splits: &mut Vec<&'a SplitNode>) {
        if let LayoutNode::Split(split) = self {
            splits.push(split);
            for child in &split.children {
                child.collect_splits(splits);
            }
        }
    }

    /// The view `id`, if the tree has one. Looked up by the interface for each view at each
    /// frame: a walk of the tree, without allocation.
    pub fn view(&self, id: &str) -> Option<&ViewNode> {
        match self {
            LayoutNode::View(view) => (view.id == id).then_some(view),
            LayoutNode::Split(split) => split.children.iter().find_map(|child| child.view(id)),
        }
    }

    pub(crate) fn view_mut(&mut self, id: &str) -> Option<&mut ViewNode> {
        match self {
            LayoutNode::View(view) => (view.id == id).then_some(view),
            LayoutNode::Split(split) => split
                .children
                .iter_mut()
                .find_map(|child| child.view_mut(id)),
        }
    }

    /// The split `id`, if the tree has one.
    pub fn split(&self, id: &str) -> Option<&SplitNode> {
        let LayoutNode::Split(split) = self else {
            return None;
        };
        if split.id == id {
            return Some(split);
        }
        split.children.iter().find_map(|child| child.split(id))
    }

    pub(crate) fn split_mut(&mut self, id: &str) -> Option<&mut SplitNode> {
        let LayoutNode::Split(split) = self else {
            return None;
        };
        if split.id == id {
            return Some(split);
        }
        split
            .children
            .iter_mut()
            .find_map(|child| child.split_mut(id))
    }

    /// Calls `f` on each split of the tree, depth first, each before its children. A list of
    /// mutable references, as for the views, is not possible: a split contains its children,
    /// so their references would overlap.
    pub(crate) fn for_each_split_mut(&mut self, f: &mut impl FnMut(&mut SplitNode)) {
        if let LayoutNode::Split(split) = self {
            f(split);
            for child in &mut split.children {
                child.for_each_split_mut(f);
            }
        }
    }
}

impl SplitNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// In the order of `app.json`.
    pub fn children(&self) -> &[LayoutNode] {
        &self.children
    }

    /// The share of each child, in the same order, adding up to 1.
    pub fn parts(&self) -> &[f32] {
        &self.parts
    }

    /// Replaces the shares by `values` (from the interface or the session state), made to add
    /// up to 1. `false`, and nothing changed, if they are not one per child, each finite and
    /// strictly positive.
    pub(crate) fn set_parts(&mut self, values: &[f32]) -> bool {
        let values: Vec<f64> = values.iter().copied().map(f64::from).collect();
        match normalize_parts(&values, self.children.len()) {
            Some(parts) => {
                self.parts = parts;
                true
            }
            None => false,
        }
    }
}

impl ViewNode {
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// `values` made into shares: each divided by their sum, so that they add up to 1. `None` if
/// there are not `expected` values, or if one is not a finite number strictly positive.
///
/// The one place where shares are computed: for the weights of `app.json`, for the shares
/// reported by the user interface, and for those of the session state. Computed in `f64`, and
/// first divided by the largest value, so that the sum cannot overflow, however large the
/// values. A share too small for an `f32` (a weight `1e-300` next to `1`) is kept at the
/// smallest positive `f32`, so that every share stays strictly positive.
pub(crate) fn normalize_parts(values: &[f64], expected: usize) -> Option<Vec<f32>> {
    if values.len() != expected || values.iter().any(|v| !(v.is_finite() && *v > 0.0)) {
        return None;
    }
    let largest = values.iter().copied().fold(0.0, f64::max);
    let sum: f64 = values.iter().map(|v| v / largest).sum();
    Some(
        values
            .iter()
            .map(|v| ((v / largest / sum) as f32).max(f32::MIN_POSITIVE))
            .collect(),
    )
}

#[cfg(test)]
mod tests;
