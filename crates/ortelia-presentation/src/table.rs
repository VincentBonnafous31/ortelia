//! The table view: the objects of a list of the document, one row per object.
//!
//! [`TableView`] holds the whole state of a table (order and widths of the columns, sort,
//! filter) and its indexes, computed by [`TableView::prepare`] at each frame, only when what
//! they depend on changed. [`TableRef`] is what the user interface reads: the columns, the
//! number of rows, and the cells, computed on demand, row by row, for the visible rows only.
//!
//! The rows are indexed in three stages, each with the parameters it was computed with, like a
//! cache key:
//! 1. `base`: the objects of the list, in the order of the document (revision);
//! 2. `sorted`: positions in `base`, sorted (revision, sort);
//! 3. `rows`: `sorted`, filtered (revision, sort, filter).

use std::cmp::Ordering;

use ortelia_config::{
    Align, CheckedApp, CheckedColumn, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH, SortOrder,
};
use ortelia_model::document::Document;
use ortelia_model::metamodel::{AttrId, AttrType, ClassId, Model, RefId};
use ortelia_model::store::{ObjectId, Store};
use ortelia_model::value::Value;

use crate::fold::{fold, fold_into};
use crate::format::{Cell, Conventions, ValueFormat, default_align, default_width};
use crate::labels::LabelLookup;

/// The separator of the columns in the text of a row prepared for the filter: a word of the
/// filter, cut at the spaces, cannot hold it, so "each word in at least one column" is "each
/// word in the text of the row".
const COLUMN_SEPARATOR: char = '\u{1F}';

/// The criteria of a sort: attributes and directions, the first one first.
type SortCriteria = Vec<(AttrId, SortOrder)>;

/// What the user interface asks of a table.
#[derive(Debug, Clone, PartialEq)]
pub enum TableIntent {
    /// A click on the header of the column at `column` (display position); `add` for
    /// Shift+click, which adds the column to the sort or advances it.
    SortBy { column: usize, add: bool },
    /// The new text of the filter.
    SetFilter(String),
    /// The column at `from` goes to the position `to` (positions after the move).
    MoveColumn { from: usize, to: usize },
    /// The column at `column` was resized to `width` points.
    ResizeColumn { column: usize, width: f32 },
}

/// A column of a table.
#[derive(Debug, Clone)]
pub struct Column {
    attr: AttrId,
    attr_type: AttrType,
    header: String,
    align: Align,
    width: f32,
    format: ValueFormat,
}

impl Column {
    /// The label of the attribute, followed by its unit between parentheses.
    pub fn header(&self) -> &str {
        &self.header
    }

    pub fn align(&self) -> Align {
        self.align
    }

    /// In points.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// `true` if the column can hold text for the filter: not a column of check boxes.
    fn filterable(&self) -> bool {
        self.attr_type != AttrType::Boolean
    }
}

/// The state and the indexes of a table view.
#[derive(Debug)]
pub struct TableView {
    id: String,
    /// The containment references from the root to the list (all single but the last).
    data: Vec<RefId>,
    /// In the order shown.
    columns: Vec<Column>,
    /// The criteria, by attribute rather than position: moving a column does not change them.
    sort: SortCriteria,
    filter: String,
    /// Incremented at each move of a column: tells the interface that the columns changed
    /// places (egui keeps the widths by position).
    layout: u64,

    // --- Indexes ---
    revision: Option<u64>,
    base: Vec<ObjectId>,
    /// The folded keys of the text columns of the sort, by position in `base`; `None` for an
    /// absent value. For `revision`.
    text_keys: Vec<(AttrId, Vec<Option<Box<str>>>)>,
    /// The folded text of each row, for the filter; prepared at the first filter, for
    /// `revision`.
    filter_texts: Option<Vec<String>>,
    sorted: Vec<u32>,
    /// The criteria of `sorted`; `None` after a change of revision.
    sorted_for: Option<SortCriteria>,
    rows: Vec<u32>,
    /// The filter of `rows`; `None` after a new sort.
    rows_for: Option<String>,

    /// The number of computations of `rows`, for the tests.
    #[cfg(test)]
    row_computations: usize,
    /// The number of cells formatted by `cell`, for the tests. A `Cell` (interior mutability,
    /// like a `mutable` member in C++): `cell` only gets `&self`.
    #[cfg(test)]
    formatted_cells: std::cell::Cell<usize>,
}

impl TableView {
    /// The table of the view of the window of `app`: the columns of the presentation of the
    /// class shown (or of its nearest super class that has one), or else its attributes. The
    /// labels are looked up once, here.
    pub fn new(app: &CheckedApp, model: &Model, lookup: &mut LabelLookup<'_>) -> TableView {
        let view = &app.window.view;
        let class = view.class;
        let conventions = Conventions::of(lookup.lang());
        // The class itself, then its ancestors, nearest first.
        let presentation = std::iter::once(class)
            .chain(model.class(class).ancestors())
            .find_map(|c| app.presentations.get(&c));
        let (defs, sort) = match presentation.and_then(|p| p.table.as_ref()) {
            Some(table) => (table.columns.clone(), table.sort.clone()),
            // Deduced from the model: the attributes, inherited ones first.
            None => (
                model
                    .class(class)
                    .attributes()
                    .iter()
                    .map(|a| CheckedColumn {
                        attr: a.id(),
                        width: None,
                        align: None,
                        format: None,
                    })
                    .collect(),
                Vec::new(),
            ),
        };
        let columns = defs
            .into_iter()
            .map(|spec| column(model, class, spec, conventions, lookup))
            .collect();
        TableView {
            id: view.id.clone(),
            data: view.data.clone(),
            columns,
            sort,
            filter: String::new(),
            layout: 0,
            revision: None,
            base: Vec::new(),
            text_keys: Vec::new(),
            filter_texts: None,
            sorted: Vec::new(),
            sorted_for: None,
            rows: Vec::new(),
            rows_for: None,
            #[cfg(test)]
            row_computations: 0,
            #[cfg(test)]
            formatted_cells: std::cell::Cell::new(0),
        }
    }

    /// Brings the indexes up to date: only what depends on a changed revision, sort or filter
    /// is computed again.
    pub fn prepare(&mut self, document: &Document) {
        let revision = document.revision();
        let store = document.store();
        if self.revision != Some(revision) {
            self.base = list(document, &self.data);
            self.revision = Some(revision);
            self.text_keys.clear();
            self.filter_texts = None;
            self.sorted_for = None;
        }
        // Each stage compares its key in place, and clears the key of the next one when it
        // computes again: nothing is cloned in a frame where nothing changed.
        if self.sorted_for.as_ref() != Some(&self.sort) {
            self.sort_rows(store);
            self.sorted_for = Some(self.sort.clone());
            self.rows_for = None;
        }
        if self.rows_for.as_ref() != Some(&self.filter) {
            self.filter_rows(store);
            self.rows_for = Some(self.filter.clone());
            #[cfg(test)]
            {
                self.row_computations += 1;
            }
        }
    }

    /// Applies an intent of the user interface. The indexes follow at the next `prepare`.
    pub fn apply(&mut self, intent: TableIntent) {
        match intent {
            TableIntent::SortBy { column, add } => {
                if let Some(attr) = self.columns.get(column).map(|c| c.attr) {
                    self.sort_by(attr, add);
                }
            }
            TableIntent::SetFilter(text) => self.filter = text,
            TableIntent::MoveColumn { from, to } => {
                if from < self.columns.len() && from != to {
                    let column = self.columns.remove(from);
                    let to = to.min(self.columns.len());
                    self.columns.insert(to, column);
                    self.layout += 1;
                }
            }
            TableIntent::ResizeColumn { column, width } => {
                if let Some(column) = self.columns.get_mut(column) {
                    // The bounds of `app.json`: a width kept here can be written there.
                    column.width = width.clamp(MIN_COLUMN_WIDTH as f32, MAX_COLUMN_WIDTH as f32);
                }
            }
        }
    }

    /// A click (`add` false) sorts on this column alone; Shift+click (`add` true) adds it at the
    /// end of the sort. Clicks on a sorted column cycle ascending, descending, no sort.
    fn sort_by(&mut self, attr: AttrId, add: bool) {
        let position = self.sort.iter().position(|&(a, _)| a == attr);
        let next = match position.map(|p| self.sort[p].1) {
            None => Some(SortOrder::Asc),
            Some(SortOrder::Asc) => Some(SortOrder::Desc),
            Some(SortOrder::Desc) => None,
        };
        if !add {
            self.sort = next.map(|order| vec![(attr, order)]).unwrap_or_default();
            return;
        }
        match (position, next) {
            (None, Some(order)) => self.sort.push((attr, order)),
            (Some(p), Some(order)) => self.sort[p].1 = order,
            (Some(p), None) => {
                self.sort.remove(p);
            }
            (None, None) => {}
        }
    }

    /// The table, as the user interface reads it.
    pub fn read<'a>(&'a self, document: &'a Document) -> TableRef<'a> {
        TableRef {
            table: self,
            store: document.store(),
        }
    }

    fn sort_rows(&mut self, store: &Store) {
        // `u32` positions: half the memory of `usize` for 100 000 rows.
        let mut order: Vec<u32> = (0..self.base.len())
            .map(|i| u32::try_from(i).expect("fewer than 2^32 rows"))
            .collect();
        if self.sort.is_empty() {
            self.sorted = order;
            return;
        }
        // The folded keys of the text columns, computed once per revision.
        for &(attr, _) in &self.sort {
            let is_text = store.model().attribute(attr).attr_type() == AttrType::String;
            if is_text && !self.text_keys.iter().any(|(a, _)| *a == attr) {
                let keys = self
                    .base
                    .iter()
                    .map(|&obj| match store.get(obj, attr) {
                        Value::Str(text) => Some(fold(text).into_boxed_str()),
                        _ => None,
                    })
                    .collect();
                self.text_keys.push((attr, keys));
            }
        }
        let criteria: Vec<Criterion<'_>> = self
            .sort
            .iter()
            .map(|&(attr, order)| Criterion {
                attr,
                descending: order == SortOrder::Desc,
                text_keys: self
                    .text_keys
                    .iter()
                    .find(|(a, _)| *a == attr)
                    .map(|(_, keys)| keys.as_slice()),
            })
            .collect();
        let base = &self.base;
        // `sort_by` is stable (a merge sort): at equality, the order of the list is kept.
        order.sort_by(|&a, &b| {
            criteria
                .iter()
                .map(|criterion| criterion.compare(store, base, a as usize, b as usize))
                .find(|o| o.is_ne())
                .unwrap_or(Ordering::Equal)
        });
        self.sorted = order;
    }

    fn filter_rows(&mut self, store: &Store) {
        let filter = fold(&self.filter);
        let words: Vec<&str> = filter.split_whitespace().collect();
        if words.is_empty() {
            self.rows = self.sorted.clone();
            return;
        }
        // The text of every row, prepared at the first filter of a revision.
        let texts = self.filter_texts.get_or_insert_with(|| {
            self.base
                .iter()
                .map(|&obj| {
                    let mut text = String::new();
                    for column in self.columns.iter().filter(|c| c.filterable()) {
                        if let Some(shown) = column.format.cell(store.get(obj, column.attr)).text()
                        {
                            fold_into(shown, &mut text);
                        }
                        text.push(COLUMN_SEPARATOR);
                    }
                    text
                })
                .collect()
        });
        self.rows = self
            .sorted
            .iter()
            .copied()
            .filter(|&i| {
                let text = &texts[i as usize];
                words.iter().all(|word| text.contains(word))
            })
            .collect();
    }
}

/// Builds a column: its header and the labels of its literals are looked up here, once.
fn column(
    model: &Model,
    class: ClassId,
    spec: CheckedColumn,
    conventions: Conventions,
    lookup: &mut LabelLookup<'_>,
) -> Column {
    let attribute = model.attribute(spec.attr);
    let attr_type = attribute.attr_type();
    let label = lookup.attribute(class, spec.attr);
    let header = match attribute.unit() {
        Some(unit) => format!("{label} ({unit})"),
        None => label,
    };
    let literals = match attr_type {
        AttrType::Enum(id) => (0..model.enum_type(id).literals().len())
            .map(|index| lookup.literal(id, index))
            .collect(),
        _ => Vec::new(),
    };
    Column {
        attr: spec.attr,
        attr_type,
        header,
        align: spec.align.unwrap_or(default_align(attr_type)),
        // At most 2 000 points (checked): exact as a float.
        width: spec.width.map_or(default_width(attr_type), |w| w as f32),
        format: ValueFormat {
            conventions,
            // `None`: the default pattern of the language, chosen by the format.
            pattern: spec.format,
            literals,
        },
    }
}

/// The objects of the list at `data`, from the root; empty if an intermediate object is
/// missing.
fn list(document: &Document, data: &[RefId]) -> Vec<ObjectId> {
    let store = document.store();
    let Some((&last, intermediates)) = data.split_last() else {
        return Vec::new();
    };
    let mut parent = document.root();
    for &reference in intermediates {
        match store.children(parent, reference).first() {
            Some(&child) => parent = child,
            None => return Vec::new(),
        }
    }
    store.children(parent, last).to_vec()
}

/// A criterion of the sort, ready to compare two rows.
struct Criterion<'a> {
    attr: AttrId,
    descending: bool,
    /// The folded keys, for a text column.
    text_keys: Option<&'a [Option<Box<str>>]>,
}

impl Criterion<'_> {
    /// Compares the rows at the positions `a` and `b` of `base`: absent values last, whatever
    /// the direction.
    fn compare(&self, store: &Store, base: &[ObjectId], a: usize, b: usize) -> Ordering {
        let ordering = match self.text_keys {
            Some(keys) => match (&keys[a], &keys[b]) {
                (Some(x), Some(y)) => x.cmp(y),
                (x, y) => return absent_last(x.is_none(), y.is_none()),
            },
            None => {
                let (x, y) = (store.get(base[a], self.attr), store.get(base[b], self.attr));
                match (x, y) {
                    (Value::Unset, _) | (_, Value::Unset) => {
                        return absent_last(x == Value::Unset, y == Value::Unset);
                    }
                    (Value::Int(x), Value::Int(y)) => x.cmp(&y),
                    (Value::Float(x), Value::Float(y)) => x.total_cmp(&y),
                    (Value::Bool(x), Value::Bool(y)) => x.cmp(&y),
                    (Value::Enum(x), Value::Enum(y)) => x.cmp(&y),
                    (Value::Date(x), Value::Date(y)) => x.cmp(&y),
                    (Value::DateTime(x), Value::DateTime(y)) => x.cmp(&y),
                    // Two values of one attribute have the same type; the texts are compared by
                    // their folded keys, above.
                    _ => Ordering::Equal,
                }
            }
        };
        if self.descending {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

/// The order of two values of which one at least is absent: the absent one last.
fn absent_last(a_absent: bool, b_absent: bool) -> Ordering {
    match (a_absent, b_absent) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => Ordering::Equal,
    }
}

/// A table, as the user interface reads it: the state of the table and the store of its
/// values. Prepared by [`TableView::prepare`] for this frame.
#[derive(Clone, Copy)]
pub struct TableRef<'a> {
    table: &'a TableView,
    store: &'a Store,
}

impl<'a> TableRef<'a> {
    /// The identifier of the view: a key for the state the interface keeps.
    pub fn id(&self) -> &'a str {
        &self.table.id
    }

    /// The columns, in the order shown.
    pub fn columns(&self) -> &'a [Column] {
        &self.table.columns
    }

    /// The indicator of the sort of the column at `column`: `⏶` (ascending) or `⏷`, followed
    /// by its rank when several columns are sorted; `None` if it is not sorted.
    pub fn sort_indicator(&self, column: usize) -> Option<String> {
        let attr = self.table.columns.get(column)?.attr;
        let sort = &self.table.sort;
        let rank = sort.iter().position(|&(a, _)| a == attr)?;
        let arrow = match sort[rank].1 {
            SortOrder::Asc => '⏶',
            SortOrder::Desc => '⏷',
        };
        Some(if sort.len() > 1 {
            format!("{arrow} {}", rank + 1)
        } else {
            arrow.to_string()
        })
    }

    /// The number of rows shown.
    pub fn row_count(&self) -> usize {
        self.table.rows.len()
    }

    /// The number of objects of the list.
    pub fn total_count(&self) -> usize {
        self.table.base.len()
    }

    /// The counter of the rows shown: `3 / 10`.
    pub fn counter(&self) -> String {
        format!("{} / {}", self.row_count(), self.total_count())
    }

    /// The cell at the row `row` (shown) and the column `column`, formatted now.
    pub fn cell(&self, row: usize, column: usize) -> Cell {
        let (Some(&position), Some(column)) =
            (self.table.rows.get(row), self.table.columns.get(column))
        else {
            return Cell::Empty;
        };
        #[cfg(test)]
        self.table
            .formatted_cells
            .set(self.table.formatted_cells.get() + 1);
        let obj = self.table.base[position as usize];
        column.format.cell(self.store.get(obj, column.attr))
    }

    /// The text of the filter.
    pub fn filter(&self) -> &'a str {
        &self.table.filter
    }

    /// The number of the layout: changes when columns change places.
    pub fn layout(&self) -> u64 {
        self.table.layout
    }
}

#[cfg(test)]
mod tests;
