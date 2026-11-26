//! The drawing of a table view: the filter bar, the headers (sort, drag and drop), the cells.
//!
//! Drawing only: the texts, the order and the widths come from the presentation, and every
//! interaction becomes an intent, returned to the caller.

use egui::{Align2, Id, Layout, Role, Sense, WidgetInfo};
use egui_extras::{Column, TableBuilder};
use ortelia_presentation::{
    Align, CLEAR_FILTER, Cell, FILTER_HINT, Intent, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH,
    TableIntent, TableRef,
};

/// The height of the header and of a row, in points.
const HEADER_HEIGHT: f32 = 24.0;
const ROW_HEIGHT: f32 = 20.0;
/// A width change below this is a rounding of egui, not a resize.
const WIDTH_TOLERANCE: f32 = 0.5;

/// The payload of the drag of a header: the position of its column.
#[derive(Debug, Clone, Copy)]
struct ColumnDrag(usize);

/// Draws a table view: its filter bar, then the table.
pub(crate) fn draw_table_view(ui: &mut egui::Ui, table: TableRef<'_>) -> Vec<Intent> {
    let mut intents = draw_filter_bar(ui, table);
    ui.add_space(4.0);
    // Columns wider than the window scroll sideways; the table scrolls its rows itself.
    egui::ScrollArea::horizontal().show(ui, |ui| intents.extend(draw_table(ui, table)));
    intents
}

/// The field of the filter, its clear button, and the counter of the rows on the right.
fn draw_filter_bar(ui: &mut egui::Ui, table: TableRef<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    ui.horizontal(|ui| {
        // The text of the presentation, edited in a copy: the change goes back as an intent.
        let mut text = table.filter().to_owned();
        let response = ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text(FILTER_HINT)
                .desired_width(240.0),
        );
        if response.changed() {
            intents.push(Intent::Table(TableIntent::SetFilter(text)));
        }
        if !table.filter().is_empty() && ui.button("✖").on_hover_text(CLEAR_FILTER).clicked() {
            intents.push(Intent::Table(TableIntent::SetFilter(String::new())));
        }
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(table.counter());
        });
    });
    intents
}

/// Draws the table: only the visible rows are asked of the presentation.
fn draw_table(ui: &mut egui::Ui, table: TableRef<'_>) -> Vec<Intent> {
    // The resizes of a frame use the positions of the columns before a move of the same frame:
    // they are dispatched first, the intents of the headers (sort, move) after them.
    let mut intents = Vec::new();
    let mut header_intents = Vec::new();
    let columns = table.columns();
    let id = Id::unique(("table", table.id()));

    // egui keeps the widths by position: after a move of columns, its widths are forgotten and
    // it starts again from the widths of the presentation, which followed their columns.
    let layout_key = id.with("layout");
    let layout_changed = ui
        .data(|d| d.get_temp::<u64>(layout_key))
        .is_some_and(|seen| seen != table.layout());
    ui.data_mut(|d| d.insert_temp(layout_key, table.layout()));

    let mut builder = TableBuilder::new(ui)
        .id_salt(id)
        .striped(true)
        .resizable(true)
        .cell_layout(Layout::left_to_right(egui::Align::Center));
    for column in columns {
        builder = builder.column(
            Column::initial(column.width())
                // The bounds of the presentation: egui never clamps a width it does not know.
                .range(MIN_COLUMN_WIDTH as f32..=MAX_COLUMN_WIDTH as f32)
                .resizable(true)
                .clip(true),
        );
    }
    if layout_changed {
        builder.reset();
    }

    builder
        .header(HEADER_HEIGHT, |mut header| {
            for column in 0..columns.len() {
                header.col(|ui| header_intents.extend(draw_header(ui, table, column)));
            }
        })
        .body(|body| {
            // The widths after a resize by the user, compared to those of the presentation.
            for (column, (&width, def)) in body.widths().iter().zip(columns).enumerate() {
                if (width - def.width()).abs() > WIDTH_TOLERANCE {
                    intents.push(Intent::Table(TableIntent::ResizeColumn { column, width }));
                }
            }
            body.rows(ROW_HEIGHT, table.row_count(), |mut row| {
                let index = row.index();
                for (column, def) in columns.iter().enumerate() {
                    row.col(|ui| draw_cell(ui, table.cell(index, column), def));
                }
            });
        });
    intents.extend(header_intents);
    intents
}

/// A header: its label and sort indicator; a click sorts, a drag moves the column.
fn draw_header(ui: &mut egui::Ui, table: TableRef<'_>, column: usize) -> Vec<Intent> {
    let mut intents = Vec::new();
    let header = table.columns()[column].header();
    let indicator = table.sort_indicator(column);
    // The whole cell; the resize handle, added later by the table at its right edge, wins
    // over it there.
    let rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::ColumnHeader, true, header);
        info.current_text_value = indicator.clone();
        info
    });

    let painter = ui.painter();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let color = ui.visuals().strong_text_color();
    let margin = 4.0;
    painter.text(
        rect.left_center() + egui::vec2(margin, 0.0),
        Align2::LEFT_CENTER,
        header,
        font.clone(),
        color,
    );
    if let Some(indicator) = &indicator {
        painter.text(
            rect.right_center() - egui::vec2(margin, 0.0),
            Align2::RIGHT_CENTER,
            indicator,
            font,
            color,
        );
    }

    if response.clicked() {
        intents.push(Intent::Table(TableIntent::SortBy {
            column,
            add: ui.input(|i| i.modifiers.shift),
        }));
    }
    response.dnd_set_drag_payload(ColumnDrag(column));
    // The column dragged takes the place of the one it is dropped on.
    if let Some(dragged) = response.dnd_hover_payload::<ColumnDrag>()
        && dragged.0 != column
    {
        let x = if dragged.0 > column {
            rect.left()
        } else {
            rect.right()
        };
        let stroke = ui.visuals().selection.stroke;
        ui.painter()
            .vline(x, rect.y_range(), egui::Stroke::new(2.0, stroke.color));
    }
    if let Some(dragged) = response.dnd_release_payload::<ColumnDrag>()
        && dragged.0 != column
    {
        intents.push(Intent::Table(TableIntent::MoveColumn {
            from: dragged.0,
            to: column,
        }));
    }
    intents
}

/// A cell: a text aligned as its column says, or a check box that cannot be changed, named by
/// its column for the accessibility tree.
fn draw_cell(ui: &mut egui::Ui, cell: Cell, column: &ortelia_presentation::Column) {
    let layout = match column.align() {
        // The layout of the cells of the table: no child `Ui` needed.
        Align::Left => {
            draw_cell_content(ui, cell, column);
            return;
        }
        // The main alignment of a left-to-right layout is only applied when it is justified: the
        // widget then takes the whole cell, and its content is centred in it.
        Align::Center => Layout::centered_and_justified(egui::Direction::LeftToRight),
        Align::Right => Layout::right_to_left(egui::Align::Center),
    };
    ui.with_layout(layout, |ui| draw_cell_content(ui, cell, column));
}

fn draw_cell_content(ui: &mut egui::Ui, cell: Cell, column: &ortelia_presentation::Column) {
    match cell {
        Cell::Empty => {}
        Cell::Text(text) => {
            ui.add(egui::Label::new(text).truncate());
        }
        Cell::Check(mut checked) => {
            let response = ui.add_enabled(false, egui::Checkbox::without_text(&mut checked));
            response.widget_info(|| {
                WidgetInfo::selected(Role::CheckBox, false, checked, column.header())
            });
        }
    }
}
