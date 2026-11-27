//! The form view: one object of a class, in a grid of labels and read-only widgets.
//!
//! [`FormView`] is built once, at the opening: the form of the presentation of the class (or of
//! its nearest super class that has a presentation), or else one field per attribute; the
//! defaults resolved by `ortelia-config`, the labels looked up, the cells placed in their
//! [`Grid`]. At each frame, only the values of the object shown are read, by [`FormRef`].
//!
//! A field gives two cells, its label then its widget; a group gives one cell, which holds a
//! grid of its own.

use ortelia_config::Align;
use ortelia_config::{
    CheckedApp, CheckedCell, CheckedElement, CheckedField, CheckedGrid, CheckedWidget,
    default_grid, default_widget,
};
use ortelia_model::metamodel::{AttrId, ClassId, Model};
use ortelia_model::store::{ObjectId, Store};
use ortelia_model::value::Value;

use crate::format::{Conventions, ValueFormat, default_align};
use crate::grid::{CellSpec, Grid};
use crate::labels::{LabelLookup, with_unit};
use crate::nearest_presentation;

/// The state of a form view: its form, built at the opening, and the object it shows.
#[derive(Debug)]
pub struct FormView {
    id: String,
    /// The class of the view: an object of another class is not shown.
    class: ClassId,
    form: FormGrid,
    object: Option<ObjectId>,
}

/// A grid of a form: the grid of the cells, and what each cell holds, at the same index.
#[derive(Debug)]
pub struct FormGrid {
    pub grid: Grid,
    pub cells: Vec<FormCell>,
}

/// What a cell of a form holds.
#[derive(Debug)]
pub enum FormCell {
    /// The label of a field, followed by the unit of its attribute.
    Label(String),
    /// The widget of a field.
    Widget(FieldCell),
    /// A group: its title, and its own grid. A `Box`, as for `ViewContent::Table`: a grid is
    /// much larger than the other variants.
    Group {
        title: Option<String>,
        grid: Box<FormGrid>,
    },
    Separator,
    /// A fixed text.
    Text(String),
}

/// The kind of the widget of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    TextField,
    TextArea { rows: u32 },
    NumberField,
    CheckBox,
    ComboBox,
    RadioGroup,
    DateField,
    DateTimeField,
}

/// The widget of a field: the attribute it shows, and how.
#[derive(Debug)]
pub struct FieldCell {
    attr: AttrId,
    kind: FieldKind,
    /// The index of the cell of its label, in the same grid: the interface links the widget to
    /// it, for the accessibility.
    label_cell: usize,
    /// The alignment of its text: that of the type of its attribute, as in a table.
    align: Align,
    /// The pattern of a date, the labels of the literals of an enum.
    format: ValueFormat,
}

impl FieldCell {
    pub fn kind(&self) -> FieldKind {
        self.kind
    }

    /// The index of the cell of the label of the field, in its grid.
    pub fn label_cell(&self) -> usize {
        self.label_cell
    }

    /// The alignment of the text of a text, number or date field.
    pub fn align(&self) -> Align {
        self.align
    }

    /// The labels of the literals of an enum, in their order; empty for another type.
    pub fn choices(&self) -> &[String] {
        &self.format.literals
    }
}

/// The value a widget shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// The text of a text, number, date or date-time field; empty without value.
    Text(String),
    /// A check box; `None`: indeterminate, without value.
    Check(Option<bool>),
    /// The literal of an enum: its index, `None` without value, and its label, empty without
    /// value.
    Choice {
        selected: Option<usize>,
        text: String,
    },
}

impl FormView {
    /// The form view `id` of the class `class`. The labels are looked up once, here.
    pub(crate) fn new(
        app: &CheckedApp,
        id: &str,
        class: ClassId,
        model: &Model,
        lookup: &mut LabelLookup<'_>,
    ) -> FormView {
        let presentation = nearest_presentation(app, model, class);
        let deduced;
        let checked = match presentation.and_then(|p| p.form.as_ref()) {
            Some(form) => form,
            None => {
                deduced = deduced_form(model, class);
                &deduced
            }
        };
        let mut builder = Builder {
            model,
            class,
            conventions: Conventions::of(lookup.lang()),
            lookup,
        };
        FormView {
            id: id.to_owned(),
            class,
            form: builder.grid(checked),
            object: None,
        }
    }

    /// Shows `object`, or nothing.
    pub(crate) fn set_object(&mut self, object: Option<ObjectId>) {
        self.object = object;
    }

    /// The form, as the user interface reads it.
    pub fn read<'a>(&'a self, store: &'a Store) -> FormRef<'a> {
        // Checked at each read, that is at each frame: once removed, its identifier could name
        // another object.
        let model = store.model();
        let object = self.object.filter(|&obj| {
            store.is_alive(obj) && model.is_subclass_of(store.class_of(obj), self.class)
        });
        FormRef {
            view: self,
            store,
            object,
        }
    }
}

/// The form deduced from the model: one field per attribute, inherited ones first, with the
/// widget of its type, on the default grid. Then a single path of code, as for a declared form.
fn deduced_form(model: &Model, class: ClassId) -> CheckedGrid {
    CheckedGrid {
        grow: default_grid(),
        elements: model
            .class(class)
            .attributes()
            .iter()
            .map(|attribute| {
                CheckedElement::Field(CheckedField {
                    attr: attribute.id(),
                    widget: default_widget(attribute.attr_type()),
                    label: None,
                    label_cell: CheckedCell::ONE,
                    cell: CheckedCell::ONE,
                })
            })
            .collect(),
    }
}

/// What the flattening of a checked grid needs.
struct Builder<'a, 'b> {
    model: &'a Model,
    class: ClassId,
    conventions: Conventions,
    lookup: &'a mut LabelLookup<'b>,
}

impl Builder<'_, '_> {
    /// Flattens a checked grid into cells: two per field, one per other element.
    fn grid(&mut self, checked: &CheckedGrid) -> FormGrid {
        let mut cells = Vec::new();
        let mut specs = Vec::new();
        let spec = |cell: &CheckedCell| CellSpec {
            span: cell.span,
            new_row: cell.new_row,
        };
        for element in &checked.elements {
            match element {
                CheckedElement::Field(field) => {
                    let label_cell = cells.len();
                    cells.push(FormCell::Label(self.label(field)));
                    specs.push(spec(&field.label_cell));
                    cells.push(FormCell::Widget(self.field(field, label_cell)));
                    specs.push(spec(&field.cell));
                }
                CheckedElement::Group { label, grid, cell } => {
                    let title = label.as_deref().map(|key| self.lookup.key(key));
                    cells.push(FormCell::Group {
                        title,
                        grid: Box::new(self.grid(grid)),
                    });
                    specs.push(spec(cell));
                }
                CheckedElement::Separator { cell } => {
                    cells.push(FormCell::Separator);
                    specs.push(spec(cell));
                }
                CheckedElement::Text { label, cell } => {
                    cells.push(FormCell::Text(self.lookup.key(label)));
                    specs.push(spec(cell));
                }
            }
        }
        FormGrid {
            grid: Grid::new(checked.grow.clone(), &specs),
            cells,
        }
    }

    /// The label of a field: the text of its key, or the label of its attribute, then the unit.
    fn label(&mut self, field: &CheckedField) -> String {
        let label = match &field.label {
            Some(key) => self.lookup.key(key),
            None => self.lookup.attribute(self.class, field.attr),
        };
        with_unit(label, self.model.attribute(field.attr).unit())
    }

    /// The widget of a field; the labels of the literals of an enum are looked up here.
    fn field(&mut self, field: &CheckedField, label_cell: usize) -> FieldCell {
        let (kind, pattern) = match &field.widget {
            CheckedWidget::TextField => (FieldKind::TextField, None),
            CheckedWidget::TextArea { rows } => (FieldKind::TextArea { rows: *rows }, None),
            CheckedWidget::NumberField => (FieldKind::NumberField, None),
            CheckedWidget::CheckBox => (FieldKind::CheckBox, None),
            CheckedWidget::ComboBox => (FieldKind::ComboBox, None),
            CheckedWidget::RadioGroup => (FieldKind::RadioGroup, None),
            CheckedWidget::Date(pattern) => (FieldKind::DateField, pattern.clone()),
            CheckedWidget::DateTime(pattern) => (FieldKind::DateTimeField, pattern.clone()),
        };
        let attr_type = self.model.attribute(field.attr).attr_type();
        FieldCell {
            attr: field.attr,
            kind,
            label_cell,
            align: default_align(attr_type),
            format: ValueFormat {
                conventions: self.conventions,
                pattern,
                literals: self.lookup.literals(attr_type),
            },
        }
    }
}

/// A form, as the user interface reads it: the form and the store of the values.
#[derive(Clone, Copy)]
pub struct FormRef<'a> {
    view: &'a FormView,
    store: &'a Store,
    /// The object shown, checked once by `FormView::read`.
    object: Option<ObjectId>,
}

impl<'a> FormRef<'a> {
    /// The identifier of the view: a key for the state the interface keeps.
    pub fn id(&self) -> &'a str {
        &self.view.id
    }

    /// The grid of the form.
    pub fn form(&self) -> &'a FormGrid {
        &self.view.form
    }

    /// The object shown: `None` without object, or if it was removed from the document, or if
    /// it is not of the class of the view.
    pub fn object(&self) -> Option<ObjectId> {
        self.object
    }

    /// The value shown by the widget of a field, read now in the store: the empty value of the
    /// widget without object.
    pub fn value(&self, field: &FieldCell) -> FieldValue {
        let value = self
            .object
            .map_or(Value::Unset, |obj| self.store.get(obj, field.attr));
        match field.kind {
            FieldKind::CheckBox => FieldValue::Check(match value {
                Value::Bool(b) => Some(b),
                _ => None,
            }),
            FieldKind::ComboBox | FieldKind::RadioGroup => FieldValue::Choice {
                selected: match value {
                    Value::Enum(index) => usize::try_from(index).ok(),
                    _ => None,
                },
                text: field.format.text(value, false),
            },
            FieldKind::TextArea { .. } => FieldValue::Text(field.format.text(value, true)),
            _ => FieldValue::Text(field.format.text(value, false)),
        }
    }
}

#[cfg(test)]
mod tests;
