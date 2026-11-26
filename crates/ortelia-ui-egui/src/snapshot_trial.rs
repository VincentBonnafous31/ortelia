//! TEMPORARY, for the snapshot platform trial only (docs/essais/captures-images.md): removed
//! once the platform is chosen, like the discarded variant of trial S1.
//!
//! The main window only shows a heading, too little to measure anything. This scene renders,
//! with egui widgets only, what tables and forms will render: texts of several sizes, a striped
//! grid with numbers and dates, a check box, a button, a text field, a separator and a scroll
//! area with a visible scroll bar.

use egui_kittest::Harness;

/// Renders the trial scene in a new harness, built like every snapshot harness.
fn trial_harness() -> Harness<'static> {
    let mut checked = true;
    let mut text = String::from("Dupont");
    let mut harness = crate::snapshot_tests::snapshot_harness().build_ui(move |ui| {
        trial_scene(ui, &mut checked, &mut text);
    });
    harness.run();
    harness
}

fn trial_scene(ui: &mut egui::Ui, checked: &mut bool, text: &mut String) {
    ui.heading("Trial scene");
    ui.label("Body text, with numbers 0123456789 and accents: éèàçù ÉÈ.");
    ui.horizontal(|ui| {
        ui.small("Small text");
        ui.monospace("Monospace 12.50 | 3.1416");
        ui.label(egui::RichText::new("Large text").size(22.0));
    });
    ui.separator();

    // The grid on the left, the controls on its right: everything fits in 640 × 400.
    ui.horizontal_top(|ui| {
        egui::Grid::new("trial_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                for header in ["Id", "Name", "Born", "Score"] {
                    ui.strong(header);
                }
                ui.end_row();
                for row in 1..=10_u32 {
                    ui.label(format!("{row}"));
                    ui.label(format!("Person {row}"));
                    ui.label(format!("19{:02}-{:02}-{:02}", 50 + row, row, 10 + row));
                    ui.label(format!("{:.2}", f64::from(row) * 7.25));
                    ui.end_row();
                }
            });
        ui.separator();

        ui.vertical(|ui| {
            ui.checkbox(checked, "Active");
            // Never clicked: the button is only drawn.
            let _ = ui.button("Save");
            ui.horizontal(|ui| {
                let label = ui.label("Name");
                // The accessibility check of egui_kittest wants a name for each text field.
                ui.add(egui::TextEdit::singleline(text).desired_width(120.0))
                    .labelled_by(label.id);
            });
            // egui's default scroll bars float and fade out when not hovered: a solid bar is
            // always drawn.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .max_height(80.0)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    for line in 1..=10 {
                        ui.label(format!("Scrolled line {line}"));
                    }
                });
        });
    });
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn trial_scene_snapshot() {
    trial_harness().snapshot("trial_scene");
}

/// Stability: two renders in a row give the same image, to the pixel.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn trial_scene_is_stable() {
    // Logs the adapter that egui_kittest picks (it prefers software renderers), for the
    // trial report; visible with `-- --nocapture`.
    let render_state = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        Default::default(),
    );
    println!("wgpu adapter: {:?}", render_state.adapter.get_info());

    let first = trial_harness()
        .render()
        .expect("the trial scene should render");
    let second = trial_harness()
        .render()
        .expect("the trial scene should render");
    assert_eq!(first.dimensions(), second.dimensions());
    let different = first
        .pixels()
        .zip(second.pixels())
        .filter(|(a, b)| a != b)
        .count();
    println!("trial scene stability: {different} different pixels");
    assert_eq!(different, 0, "two renders of the trial scene differ");
}
