//! The modal dialogs and the status bar.

use egui::{Id, Layout};
use ortelia_presentation::{DialogViewModel, Intent};

/// Draws a modal dialog; gives the intent of its button when it is clicked, or the closing of
/// the dialog when the user presses Escape or clicks outside (the presentation decides whether
/// it closes).
pub(crate) fn draw_dialog(ctx: &egui::Context, dialog: &DialogViewModel<'_>) -> Option<Intent> {
    let mut intent = None;
    let response = egui::Modal::new(Id::unique("dialog")).show(ctx, |ui| {
        ui.set_max_width(560.0);
        ui.heading(dialog.title);
        ui.separator();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for line in dialog.lines {
                    ui.label(line);
                }
                if let Some(more) = &dialog.more {
                    ui.label(more);
                }
            });
        ui.separator();
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(dialog.button).clicked() {
                intent = Some(dialog.button_intent.clone());
            }
        });
    });
    if intent.is_none() && response.should_close() {
        intent = Some(Intent::CloseDialog);
    }
    intent
}

/// Draws the status bar: the button of the number of warnings, which opens their list.
pub(crate) fn draw_status_bar(ui: &mut egui::Ui, text: &str) -> Option<Intent> {
    let mut intent = None;
    egui::Panel::bottom("status_bar").show(ui, |ui| {
        if ui.button(text).clicked() {
            intent = Some(Intent::ShowWarnings);
        }
    });
    intent
}
