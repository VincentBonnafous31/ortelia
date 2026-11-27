//! The form of a class in `app.json` (`presentations[].form`), as read by serde, and its check
//! against the model.
//!
//! A form is a tree of elements placed in a grid: fields (an attribute and its widget), groups
//! (a framed title and a grid of their own), separators and fixed texts. The check resolves
//! every default (grid, spans, widgets), so that the presentation layer has no rule of the file
//! format left to know.

use ortelia_model::metamodel::{AttrId, AttrType, ClassId, Model};
use serde::Deserialize;

use crate::app::{ConfigError, PresentationDef};
use crate::pattern::DatePattern;

/// The range of the number of lines of a text area.
pub const TEXT_AREA_ROWS: std::ops::RangeInclusive<u32> = 1..=50;
/// The number of lines of a text area that declares none.
pub const DEFAULT_TEXT_AREA_ROWS: u32 = 4;

// --- The file ---
//
// Plain comments, not doc comments, for what concerns the code: the doc comments become the
// descriptions of the JSON Schema, shown by text editors. `PartialEq` without `Eq`: the weights
// of the columns are `f64`.

/// The form of a class: its elements, placed in a grid from left to right, then row by row.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FormDef {
    /// The columns of the grid. By default two: the labels, then the values, which take the
    /// width left.
    #[serde(default)]
    pub columns: Option<Vec<GridColumnDef>>,
    /// The fields, groups, separators and texts, in the order of the grid.
    pub elements: Vec<Element>,
}

/// A column of a grid.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GridColumnDef {
    /// The share of the width left that the column takes, a number positive or zero (0 by
    /// default: the column keeps the width of its content).
    #[serde(default)]
    pub grow: Option<f64>,
}

/// The place of a cell in its grid.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CellDef {
    /// The number of columns the cell takes, from 1 to the number of columns of the grid. By
    /// default 1, and the whole row for a group or a separator.
    #[serde(default)]
    pub span: Option<u32>,
    /// `true`: the cell starts a new row, unless the current row is still empty.
    #[serde(default)]
    pub new_row: bool,
}

/// An element of a form or of a group, chosen by `_type_`.
//
// An internally tagged enum, like `Content`. `GroupDef::elements` holds `Element` again: the
// type is recursive through the `Vec`, which needs no `Box`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(tag = "_type_", deny_unknown_fields)]
pub enum Element {
    /// An attribute of the object: its label, then its widget, in two cells.
    Field(FieldDef),
    /// A frame with a title, and a grid of its own for its elements.
    Group(GroupDef),
    /// A horizontal line.
    Separator(SeparatorDef),
    /// A fixed text.
    Text(TextDef),
}

/// A field: an attribute of the object, shown by a widget next to its label.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FieldDef {
    /// The name of an attribute of the class, inherited ones included.
    pub path: String,
    /// By default, the widget of the type of the attribute: a text field, a number field, a
    /// check box, a combo box, a date or date-time field.
    #[serde(default)]
    pub widget: Option<FieldWidget>,
    /// The key of the label (in `labels`); by default, the label of the attribute.
    #[serde(default)]
    pub label: Option<String>,
    /// The cell of the label.
    #[serde(default)]
    pub label_cell: Option<CellDef>,
    /// The cell of the widget.
    #[serde(default)]
    pub cell: Option<CellDef>,
}

/// A group: a frame with a title, and a grid of its own.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GroupDef {
    /// The key of the title (in `labels`); without it, the group has no title.
    #[serde(default)]
    pub label: Option<String>,
    /// The columns of the grid of the group; by default two, as for a form.
    #[serde(default)]
    pub columns: Option<Vec<GridColumnDef>>,
    #[serde(default)]
    pub elements: Vec<Element>,
    /// The cell of the group; by default, the whole row.
    #[serde(default)]
    pub cell: Option<CellDef>,
}

/// A separator: a horizontal line.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SeparatorDef {
    /// The cell of the separator; by default, the whole row.
    #[serde(default)]
    pub cell: Option<CellDef>,
}

/// A fixed text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TextDef {
    /// The key of the text, in `labels`.
    pub label: String,
    #[serde(default)]
    pub cell: Option<CellDef>,
}

/// The widget of a field, chosen by `_type_`. Each one accepts only the types of attribute it
/// suits.
//
// Structs without fields rather than unit variants, like `EmptyWidget`: a key of too many is
// refused, and an option added later does not change the shape of the JSON. The names of
// `_type_` are unique in the whole `app.json` (`TextField`, not `Text`, the fixed text).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(tag = "_type_", deny_unknown_fields)]
pub enum FieldWidget {
    /// A text on one line, for a text.
    TextField(NoOptions),
    /// A text on several lines, for a text.
    TextArea(TextAreaDef),
    /// A number, for an integer or a decimal.
    NumberField(NoOptions),
    /// A check box, for a boolean.
    CheckBox(NoOptions),
    /// A drop-down list of the literals, for an enum.
    ComboBox(NoOptions),
    /// One radio button per literal, for an enum.
    RadioGroup(NoOptions),
    /// A date.
    DateField(DateFieldDef),
    /// A date and a time.
    DateTimeField(DateFieldDef),
}

/// A widget without options.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NoOptions {}

/// The options of a text area.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TextAreaDef {
    /// The number of lines shown, from 1 to 50 (4 by default).
    #[serde(default)]
    pub rows: Option<u32>,
}

/// The options of a date or date-time field.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DateFieldDef {
    /// An LDML pattern: `dd/MM/yyyy`, `d MMMM yyyy HH:mm`… By default, the one of the language.
    #[serde(default)]
    pub format: Option<String>,
}

impl FieldWidget {
    /// The name of the widget, its `_type_`, for the messages.
    pub fn name(&self) -> &'static str {
        match self {
            FieldWidget::TextField(_) => "TextField",
            FieldWidget::TextArea(_) => "TextArea",
            FieldWidget::NumberField(_) => "NumberField",
            FieldWidget::CheckBox(_) => "CheckBox",
            FieldWidget::ComboBox(_) => "ComboBox",
            FieldWidget::RadioGroup(_) => "RadioGroup",
            FieldWidget::DateField(_) => "DateField",
            FieldWidget::DateTimeField(_) => "DateTimeField",
        }
    }
}

// --- The checked form ---

/// A grid, checked: at least one column, and its elements, every default resolved.
// `PartialEq` without `Eq`: the weights are `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedGrid {
    /// The weight of each column, finite and positive or zero.
    pub grow: Vec<f32>,
    pub elements: Vec<CheckedElement>,
}

/// An element of a form, checked.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckedElement {
    Field(CheckedField),
    Group {
        /// The key of the title.
        label: Option<String>,
        grid: CheckedGrid,
        cell: CheckedCell,
    },
    Separator {
        cell: CheckedCell,
    },
    Text {
        /// The key of the text.
        label: String,
        cell: CheckedCell,
    },
}

/// A field, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedField {
    pub attr: AttrId,
    /// Declared, or the default of the type of the attribute; it suits that type.
    pub widget: CheckedWidget,
    /// The key of the label; `None`: the label of the attribute.
    pub label: Option<String>,
    pub label_cell: CheckedCell,
    pub cell: CheckedCell,
}

/// The place of a cell, checked: `span` is between 1 and the number of columns of its grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedCell {
    pub span: usize,
    pub new_row: bool,
}

impl CheckedCell {
    /// A cell of one column, on the current row.
    pub const ONE: CheckedCell = CheckedCell {
        span: 1,
        new_row: false,
    };
}

/// The widget of a field, checked, with its options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedWidget {
    TextField,
    TextArea {
        rows: u32,
    },
    NumberField,
    CheckBox,
    ComboBox,
    RadioGroup,
    /// `None`: the default pattern of the language.
    Date(Option<DatePattern>),
    DateTime(Option<DatePattern>),
}

/// The widget of a field of type `attr_type` that declares none.
pub fn default_widget(attr_type: AttrType) -> CheckedWidget {
    match attr_type {
        AttrType::String => CheckedWidget::TextField,
        AttrType::Int | AttrType::Float => CheckedWidget::NumberField,
        AttrType::Boolean => CheckedWidget::CheckBox,
        AttrType::Enum(_) => CheckedWidget::ComboBox,
        AttrType::Date => CheckedWidget::Date(None),
        AttrType::DateTime => CheckedWidget::DateTime(None),
    }
}

/// `true` if `widget` can show a value of type `attr_type`. The one rule of compatibility, for
/// the check and for its test against [`default_widget`].
pub fn accepts(widget: &CheckedWidget, attr_type: AttrType) -> bool {
    match widget {
        CheckedWidget::TextField | CheckedWidget::TextArea { .. } => attr_type == AttrType::String,
        CheckedWidget::NumberField => matches!(attr_type, AttrType::Int | AttrType::Float),
        CheckedWidget::CheckBox => attr_type == AttrType::Boolean,
        CheckedWidget::ComboBox | CheckedWidget::RadioGroup => {
            matches!(attr_type, AttrType::Enum(_))
        }
        CheckedWidget::Date(_) => attr_type == AttrType::Date,
        CheckedWidget::DateTime(_) => attr_type == AttrType::DateTime,
    }
}

/// The grid of a form or a group without `columns`: the labels, then the values, which grow.
pub fn default_grid() -> Vec<f32> {
    vec![0.0, 1.0]
}

/// Checks the form of the presentation of `class`, and resolves its defaults. Every error is
/// reported; `None` if one was found.
pub(crate) fn check_form(
    model: &Model,
    class: ClassId,
    form: &FormDef,
    errors: &mut Vec<ConfigError>,
) -> Option<CheckedGrid> {
    let error_count = errors.len();
    let mut checker = FormChecker {
        model,
        class,
        errors,
    };
    let grid = checker.grid(form.columns.as_deref(), &form.elements, "form");
    (errors.len() == error_count).then_some(grid)
}

/// The keys of labels cited by the forms of `presentations` (titles of groups, texts, labels of
/// fields), on the definitions as read: a key stays known even if its form has another error.
pub(crate) fn form_label_keys(presentations: &[PresentationDef]) -> Vec<&str> {
    fn collect<'a>(elements: &'a [Element], keys: &mut Vec<&'a str>) {
        for element in elements {
            match element {
                Element::Field(field) => keys.extend(field.label.as_deref()),
                Element::Group(group) => {
                    keys.extend(group.label.as_deref());
                    collect(&group.elements, keys);
                }
                Element::Text(text) => keys.push(&text.label),
                Element::Separator(_) => {}
            }
        }
    }
    let mut keys = Vec::new();
    for form in presentations.iter().filter_map(|p| p.form.as_ref()) {
        collect(&form.elements, &mut keys);
    }
    keys
}

/// The state of the check of one form: what every step needs, so that the recursion passes
/// only the grid and the location.
struct FormChecker<'a> {
    model: &'a Model,
    class: ClassId,
    errors: &'a mut Vec<ConfigError>,
}

impl FormChecker<'_> {
    fn class_name(&self) -> String {
        self.model.class(self.class).name().to_owned()
    }

    /// Checks a grid and its elements, at `location` (`form`, `form.elements[0]`). Gives a grid
    /// even with errors, which the caller then drops.
    fn grid(
        &mut self,
        columns: Option<&[GridColumnDef]>,
        elements: &[Element],
        location: &str,
    ) -> CheckedGrid {
        let grow = match columns {
            None => default_grid(),
            Some(columns) => {
                if columns.is_empty() {
                    self.errors.push(ConfigError::NoGridColumn {
                        class: self.class_name(),
                        location: location.to_owned(),
                    });
                }
                columns
                    .iter()
                    .enumerate()
                    .map(|(index, column)| {
                        let grow = column.grow.unwrap_or(0.0);
                        // Checked as the `f32` the grid computes with: a weight beyond its range
                        // (1e39) would become infinite there.
                        let weight = grow as f32;
                        if !(weight.is_finite() && grow >= 0.0) {
                            self.errors.push(ConfigError::GrowValue {
                                class: self.class_name(),
                                location: location.to_owned(),
                                index,
                            });
                        }
                        weight
                    })
                    .collect()
            }
        };
        // Without column, the spans of the elements cannot be checked against the grid: an
        // error is already reported.
        let width = (!grow.is_empty()).then_some(grow.len());
        let elements = elements
            .iter()
            .enumerate()
            .filter_map(|(index, element)| {
                let location = format!("{location}.elements[{index}]");
                self.element(element, width, &location)
            })
            .collect();
        CheckedGrid { grow, elements }
    }

    /// Checks an element of a grid of `columns` columns.
    fn element(
        &mut self,
        element: &Element,
        columns: Option<usize>,
        location: &str,
    ) -> Option<CheckedElement> {
        // A group and a separator take the whole row by default, a field and a text a column.
        let whole_row = columns.unwrap_or(1);
        match element {
            Element::Field(field) => self
                .field(field, columns, location)
                .map(CheckedElement::Field),
            Element::Group(group) => {
                let cell = self.cell(group.cell.as_ref(), whole_row, columns, location, "cell");
                let grid = self.grid(group.columns.as_deref(), &group.elements, location);
                Some(CheckedElement::Group {
                    label: group.label.clone(),
                    grid,
                    cell,
                })
            }
            Element::Separator(separator) => Some(CheckedElement::Separator {
                cell: self.cell(
                    separator.cell.as_ref(),
                    whole_row,
                    columns,
                    location,
                    "cell",
                ),
            }),
            Element::Text(text) => Some(CheckedElement::Text {
                label: text.label.clone(),
                cell: self.cell(text.cell.as_ref(), 1, columns, location, "cell"),
            }),
        }
    }

    /// Checks a cell: its span, `default` if it declares none, is between 1 and `columns`.
    /// `key` names it in the messages (`cell`, `labelCell`).
    fn cell(
        &mut self,
        cell: Option<&CellDef>,
        default: usize,
        columns: Option<usize>,
        location: &str,
        key: &str,
    ) -> CheckedCell {
        let Some(cell) = cell else {
            return CheckedCell {
                span: default,
                new_row: false,
            };
        };
        let span = cell.span.map_or(default, |span| span as usize);
        let too_wide = columns.is_some_and(|columns| span > columns);
        if span == 0 || too_wide {
            self.errors.push(ConfigError::SpanOutOfGrid {
                class: self.class_name(),
                location: format!("{location}.{key}"),
                span,
                columns: columns.unwrap_or(0),
            });
        }
        CheckedCell {
            span,
            new_row: cell.new_row,
        }
    }

    /// Checks a field: its attribute, its widget and its two cells.
    fn field(
        &mut self,
        field: &FieldDef,
        columns: Option<usize>,
        location: &str,
    ) -> Option<CheckedField> {
        let label_cell = self.cell(field.label_cell.as_ref(), 1, columns, location, "labelCell");
        let cell = self.cell(field.cell.as_ref(), 1, columns, location, "cell");
        let Some(attr) = self.model.class(self.class).attribute(&field.path) else {
            self.errors.push(ConfigError::UnknownField {
                class: self.class_name(),
                location: location.to_owned(),
                path: field.path.clone(),
            });
            return None;
        };
        let attr_type = self.model.attribute(attr).attr_type();
        let widget = match &field.widget {
            None => default_widget(attr_type),
            Some(def) => {
                let widget = self.widget(def, &field.path, location)?;
                if !accepts(&widget, attr_type) {
                    self.errors.push(ConfigError::WidgetMismatch {
                        class: self.class_name(),
                        location: location.to_owned(),
                        path: field.path.clone(),
                        widget: def.name(),
                        attr_type: self.model.type_name(attr_type).to_owned(),
                    });
                }
                widget
            }
        };
        Some(CheckedField {
            attr,
            widget,
            label: field.label.clone(),
            label_cell,
            cell,
        })
    }

    /// Checks the options of a widget: the number of lines of a text area, the pattern of a
    /// date. Its compatibility with the attribute is checked apart.
    fn widget(&mut self, def: &FieldWidget, path: &str, location: &str) -> Option<CheckedWidget> {
        let pattern = |checker: &mut Self, format: &Option<String>, with_time: bool| {
            let Some(pattern) = format else {
                return Some(None);
            };
            match DatePattern::parse(pattern, with_time) {
                Ok(parsed) => Some(Some(parsed)),
                Err(error) => {
                    checker.errors.push(ConfigError::FieldPattern {
                        class: checker.class_name(),
                        location: location.to_owned(),
                        path: path.to_owned(),
                        pattern: pattern.clone(),
                        error: Box::new(error),
                    });
                    None
                }
            }
        };
        Some(match def {
            FieldWidget::TextField(_) => CheckedWidget::TextField,
            FieldWidget::TextArea(area) => {
                let rows = area.rows.unwrap_or(DEFAULT_TEXT_AREA_ROWS);
                if !TEXT_AREA_ROWS.contains(&rows) {
                    self.errors.push(ConfigError::TextAreaRows {
                        class: self.class_name(),
                        location: location.to_owned(),
                        rows,
                    });
                    return None;
                }
                CheckedWidget::TextArea { rows }
            }
            FieldWidget::NumberField(_) => CheckedWidget::NumberField,
            FieldWidget::CheckBox(_) => CheckedWidget::CheckBox,
            FieldWidget::ComboBox(_) => CheckedWidget::ComboBox,
            FieldWidget::RadioGroup(_) => CheckedWidget::RadioGroup,
            FieldWidget::DateField(date) => {
                CheckedWidget::Date(pattern(self, &date.format, false)?)
            }
            FieldWidget::DateTimeField(date) => {
                CheckedWidget::DateTime(pattern(self, &date.format, true)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each default widget suits the type it is the default of.
    #[test]
    fn default_widgets_are_accepted() {
        let model = ortelia_model::metamodel::Model::compile(&[
            ortelia_model::metamodel::ModelFile::parse(
                r#"{ "formatVersion": "0.1", "packages": [ { "name": "p",
                     "enums": [ { "name": "E", "literals": ["A"] } ] } ] }"#,
            )
            .expect("parse"),
        ])
        .expect("compile");
        let enum_type = AttrType::Enum(model.enum_id("E").expect("E"));
        let types = [
            AttrType::String,
            AttrType::Int,
            AttrType::Float,
            AttrType::Boolean,
            AttrType::Date,
            AttrType::DateTime,
            enum_type,
        ];
        for attr_type in types {
            assert!(
                accepts(&default_widget(attr_type), attr_type),
                "{attr_type:?}"
            );
        }
        // And no other type: a check box is a boolean only.
        let accepted: Vec<AttrType> = types
            .into_iter()
            .filter(|&t| accepts(&CheckedWidget::CheckBox, t))
            .collect();
        assert_eq!(accepted, [AttrType::Boolean]);
    }
}
