//! Layer 4: the egui user interface. It only draws the view-models of `ortelia-presentation`
//! and turns clicks and keys into intents. No display logic lives here.
//!
//! Each frame: the presentation prepares its indexes, the window is drawn from the state (read
//! only), and the intents gathered while drawing are dispatched to the state afterwards.

mod dialogs;
mod table;

use ortelia_presentation::{AppState, Intent, MainContent};

/// Opens the main window and runs the event loop until the window is closed.
///
/// `Box<dyn Error>` plays the role of `throws Exception` in Java: the executable only prints
/// the error and does not need to know eframe's error type.
pub fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let options = native_options(&state);
    // The application name is also the key eframe uses for its own settings.
    eframe::run_native(
        "Ortelia",
        options,
        Box::new(|_creation_context| Ok(Box::new(OrteliaApp::new(state)))),
    )?;
    Ok(())
}

/// Options of the native window, computed from the presentation.
fn native_options(state: &AppState) -> eframe::NativeOptions {
    let window = state.window();
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&window.title)
            .with_inner_size([window.width, window.height]),
        ..Default::default()
    }
}

/// The eframe application: owns the presentation state and draws it at each frame.
struct OrteliaApp {
    state: AppState,
    /// Set after the first frame, so that "main window shown" is logged only once.
    first_frame_logged: bool,
    /// Set once the closing of the window was asked of the system.
    close_sent: bool,
}

impl OrteliaApp {
    fn new(state: AppState) -> Self {
        OrteliaApp {
            state,
            first_frame_logged: false,
            close_sent: false,
        }
    }
}

impl eframe::App for OrteliaApp {
    /// Called by eframe at each frame (immediate mode: the whole UI is redrawn each time).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.state.prepare();
        let intents = draw_main_window(ui, &self.state);
        if !intents.is_empty() {
            // The state changes after the drawing: the next frame shows it.
            ui.ctx().request_repaint();
        }
        for intent in intents {
            self.state.dispatch(intent);
        }
        if self.state.quit_requested() && !self.close_sent {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            self.close_sent = true;
        }
        if !self.first_frame_logged {
            // The smoke test waits for this line: it proves that a frame was really drawn.
            tracing::info!("main window shown");
            self.first_frame_logged = true;
        }
    }
}

/// Draws the main window: the status bar, the content, and the dialog over them. Drawing only:
/// returns the intents of the user.
fn draw_main_window(ui: &mut egui::Ui, state: &AppState) -> Vec<Intent> {
    let mut intents = Vec::new();
    // A bottom panel is added before the central panel, which takes the remaining space.
    if let Some(text) = state.status_bar() {
        intents.extend(dialogs::draw_status_bar(ui, text));
    }
    egui::CentralPanel::default().show(ui, |ui| match state.content() {
        MainContent::Table(table) => intents.extend(table::draw_table_view(ui, table)),
        MainContent::Nothing => {}
    });
    if let Some(dialog) = state.dialog() {
        intents.extend(dialogs::draw_dialog(ui.ctx(), &dialog));
    }
    intents
}

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod tests;
