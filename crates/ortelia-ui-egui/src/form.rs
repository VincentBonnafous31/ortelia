//! The drawing of a form view: its grid of labels and read-only widgets, in a scroll area.
//!
//! Drawing only: the cells, their places and the widths of the columns come from the
//! presentation (`FormGrid`, `Grid::widths`); the interface measures the minimum width of each
//! cell, then draws each one in the rectangle of its columns. Nothing is kept from one frame to
//! the next, and no intent is emitted: the widgets cannot change a value.
//!
//! Each cell is drawn in a child `Ui` placed by hand, as an absolutely positioned component in
//! a Swing panel without layout manager (`setBounds`): a child `Ui` takes no space in its
//! parent, so the space of the whole grid is reserved at the end, in one place.

use egui::{
    Align, Align2, Id, Layout, Rect, RichText, Role, Sense, TextStyle, TextWrapMode, UiBuilder,
    WidgetText,
};
use ortelia_presentation::{FieldCell, FieldKind, FieldValue, FormCell, FormGrid, FormRef};

/// The minimum width of a widget, by kind, in points: a floor when the view narrows, below
/// which the form scrolls sideways. Not measured: a value may be of any length.
const TEXT_MIN: f32 = 80.0;
const NUMBER_MIN: f32 = 60.0;
const COMBO_MIN: f32 = 100.0;
const DATE_MIN: f32 = 90.0;
const DATE_TIME_MIN: f32 = 150.0;
const TEXT_AREA_MIN: f32 = 120.0;

/// Draws a form view, in a scroll area of its own, within a node of role `Form` of the
/// accessibility tree: a screen reader, and the tests, find its widgets there, apart from those
/// of the other views.
pub(crate) fn draw_form_view(ui: &mut egui::Ui, form: FormRef<'_>) {
    let builder = UiBuilder::new().accessibility_role(Role::Form);
    ui.scope_builder(builder, |ui| {
        egui::ScrollArea::both()
            .id_salt(("form", form.id()))
            .auto_shrink(false)
            .show(ui, |ui| {
                let width = ui.available_width();
                draw_grid(ui, form, form.form(), width);
            });
    });
}

/// Draws `grid` from the cursor of `ui`, on the width `available`, and reserves its space.
fn draw_grid(ui: &mut egui::Ui, form: FormRef<'_>, grid: &FormGrid, available: f32) {
    let spacing = ui.spacing().item_spacing;
    let mins: Vec<f32> = grid.cells.iter().map(|cell| min_width(ui, cell)).collect();
    let widths = grid.grid.widths(&mins, available, spacing.x);
    let origin = ui.cursor().min;
    // The identifier of the label of each cell drawn, for the widget of its field.
    let mut labels: Vec<Option<Id>> = vec![None; grid.cells.len()];
    let mut top = origin.y;
    let mut bottom = origin.y;
    let mut row = 0;
    for (index, (cell, place)) in grid.cells.iter().zip(grid.grid.cells()).enumerate() {
        // The cells come row by row: a new row starts under the highest cell of the previous.
        if place.row != row {
            row = place.row;
            top = bottom + spacing.y;
        }
        let (left, width) = widths.span(place.col, place.span);
        let rect = Rect::from_min_max(
            egui::pos2(origin.x + left, top),
            egui::pos2(origin.x + left + width, f32::INFINITY),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .id_salt(index)
                .layout(Layout::top_down(Align::Min)),
        );
        labels[index] = draw_cell(&mut child, form, cell, &labels);
        bottom = bottom.max(child.min_rect().bottom());
    }
    let size = egui::vec2(widths.total, bottom - origin.y);
    ui.allocate_rect(Rect::from_min_size(origin, size), Sense::hover());
}

/// The width of `text` on one line, in the body font.
fn text_width(ui: &egui::Ui, text: impl Into<WidgetText>) -> f32 {
    text.into()
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Body,
        )
        .size()
        .x
}

/// The minimum width of a cell.
fn min_width(ui: &egui::Ui, cell: &FormCell) -> f32 {
    match cell {
        FormCell::Label(text) | FormCell::Text(text) => text_width(ui, text.as_str()),
        FormCell::Widget(field) => match field.kind() {
            FieldKind::TextField => TEXT_MIN,
            FieldKind::TextArea { .. } => TEXT_AREA_MIN,
            FieldKind::NumberField => NUMBER_MIN,
            FieldKind::CheckBox => ui.spacing().interact_size.x,
            FieldKind::ComboBox => COMBO_MIN,
            FieldKind::DateField => DATE_MIN,
            FieldKind::DateTimeField => DATE_TIME_MIN,
            // Each button: its circle, a space, its label; a space between two buttons.
            FieldKind::RadioGroup => {
                let spacing = ui.spacing();
                let button = spacing.icon_width + spacing.icon_spacing;
                let gaps = spacing.item_spacing.x * field.choices().len().saturating_sub(1) as f32;
                field
                    .choices()
                    .iter()
                    .map(|choice| button + text_width(ui, choice.as_str()))
                    .sum::<f32>()
                    + gaps
            }
        },
        FormCell::Group { title, grid } => {
            let spacing = ui.spacing().item_spacing.x;
            let mins: Vec<f32> = grid.cells.iter().map(|cell| min_width(ui, cell)).collect();
            // Without width available, the grid keeps its minimums: its total is its minimum.
            let inner = grid.grid.widths(&mins, 0.0, spacing).total;
            let title = title
                .as_deref()
                .map_or(0.0, |t| text_width(ui, RichText::new(t).strong()));
            inner.max(title) + group_frame(ui).total_margin().sum().x
        }
        FormCell::Separator => 0.0,
    }
}

/// The frame of a group.
fn group_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::group(ui.style())
}

/// Draws a cell in its own `ui`; gives the identifier of a label, for its widget. `labels`:
/// those of the cells already drawn.
fn draw_cell(
    ui: &mut egui::Ui,
    form: FormRef<'_>,
    cell: &FormCell,
    labels: &[Option<Id>],
) -> Option<Id> {
    match cell {
        // In a box of the height of a widget, centred: aligned with the widgets of one line of
        // its row, at the top in front of a text area or a group.
        FormCell::Label(text) => {
            let size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
            let layout = Layout::left_to_right(Align::Center);
            let label = ui.allocate_ui_with_layout(size, layout, |ui| {
                ui.add(egui::Label::new(text.as_str()).extend())
            });
            return Some(label.inner.id);
        }
        FormCell::Widget(field) => {
            let label = labels[field.label_cell()];
            let response = draw_field(ui, field, form.value(field));
            if let Some(label) = label {
                response.labelled_by(label);
            }
        }
        FormCell::Group { title, grid } => {
            group_frame(ui).show(ui, |ui| {
                if let Some(title) = title {
                    ui.strong(title.as_str());
                }
                let width = ui.available_width();
                draw_grid(ui, form, grid, width);
            });
        }
        FormCell::Separator => {
            ui.separator();
        }
        FormCell::Text(text) => {
            ui.label(text.as_str());
        }
    }
    None
}

/// Draws the read-only widget of a field, with the value `value`; gives its response, to link
/// it to its label.
fn draw_field(ui: &mut egui::Ui, field: &FieldCell, value: FieldValue) -> egui::Response {
    let width = ui.available_width();
    match (field.kind(), value) {
        (FieldKind::CheckBox, FieldValue::Check(checked)) => {
            let mut shown = checked.unwrap_or(false);
            ui.add_enabled(
                false,
                egui::Checkbox::without_text(&mut shown).indeterminate(checked.is_none()),
            )
        }
        (FieldKind::ComboBox, FieldValue::Choice { text, .. }) => {
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt("combo")
                    .selected_text(text)
                    .width(width)
                    .show_ui(ui, |_ui| {})
                    .response
            })
            .inner
        }
        (FieldKind::RadioGroup, FieldValue::Choice { selected, .. }) => {
            // A group of the accessibility tree, which takes the label of the field.
            let builder = UiBuilder::new()
                .layout(Layout::left_to_right(Align::Center).with_main_wrap(true))
                .accessibility_role(Role::RadioGroup);
            ui.scope_builder(builder, |ui| {
                ui.disable();
                for (index, literal) in field.choices().iter().enumerate() {
                    // Disabled: the response, a click, is of no use.
                    let _ = ui.radio(selected == Some(index), literal.as_str());
                }
            })
            .response
        }
        (FieldKind::TextArea { rows }, FieldValue::Text(text)) => {
            // A `&str`: the text can be selected and copied, not changed.
            let mut text = text.as_str();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(rows as usize)
                    .desired_width(width)
                    .frame(read_only_frame(ui)),
            )
        }
        (_, FieldValue::Text(text)) => {
            let mut text = text.as_str();
            // The alignment of the presentation: numbers on the right, as in the table.
            let align = match field.align() {
                ortelia_presentation::Align::Left => Align2::LEFT_CENTER,
                ortelia_presentation::Align::Center => Align2::CENTER_CENTER,
                ortelia_presentation::Align::Right => Align2::RIGHT_CENTER,
            };
            ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(width)
                    .align(align)
                    .frame(read_only_frame(ui)),
            )
        }
        // The presentation gives each kind its value: no other pair.
        (_, _) => ui.label(""),
    }
}

/// The frame of a read-only text field: a thin border, the grey of the frames of the groups,
/// without background. egui draws such a field (over a `&str`) without background nor visible
/// border: the empty form would show its labels only. The text keeps its place: the margin is
/// that of egui, less the width of the border, as egui does for its own border.
fn read_only_frame(ui: &egui::Ui) -> egui::Frame {
    let visuals = &ui.visuals().widgets.noninteractive;
    let stroke = visuals.bg_stroke;
    let padding = ui.spacing().button_padding;
    egui::Frame::new()
        .stroke(stroke)
        .corner_radius(visuals.corner_radius)
        .inner_margin(egui::Margin::from(padding) - egui::Margin::from(stroke.width))
}

#[cfg(test)]
mod tests;
