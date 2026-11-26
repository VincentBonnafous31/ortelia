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
    window: WindowTracker,
}

impl OrteliaApp {
    fn new(state: AppState) -> Self {
        OrteliaApp {
            state,
            first_frame_logged: false,
            close_sent: false,
            window: WindowTracker::default(),
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
        // The size of the window, for the session state. Nothing to draw again: no repaint.
        if let Some(intent) = self.window.intent(ui.ctx(), &self.state) {
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

    /// Called once by eframe when the window closes, whatever the cause (button of the window,
    /// `ViewportCommand::Close`), after the end of the event loop; not when the process is
    /// killed. The only place where the session state is saved. (Without the `glow` feature of
    /// eframe, this method takes no argument.)
    fn on_exit(&mut self) {
        self.state.save_session();
    }
}

/// The largest difference, in points, between two sizes taken for the same.
const SIZE_TOLERANCE: f32 = 0.5;

/// Follows the size of the window, for the session state.
#[derive(Debug, Default)]
struct WindowTracker {
    /// Set at the first action of the user in the window. Before it, the first frames may show
    /// a provisional size, before the system applies the size asked (640 × 480 seen on WSLg): a
    /// closing just after would save it.
    user_seen: bool,
}

impl WindowTracker {
    /// The intent of a new size of the window, if it has one: see [`window_intent`].
    fn intent(&mut self, ctx: &egui::Context, state: &AppState) -> Option<Intent> {
        // `viewport_rect`, not `ViewportInfo::inner_rect`, which is always `None` on Wayland.
        // It is in points of egui, zoom included (Ctrl+plus): times the zoom, the size of the
        // window, which `with_inner_size` takes at the next opening (zoom 1 then). Read before
        // `input`: no lock of the context taken inside another.
        let size = ctx.viewport_rect().size() * ctx.zoom_factor();
        ctx.input(|input| {
            self.user_seen = self.user_seen || input.events.iter().any(is_user_action);
            if !self.user_seen {
                return None;
            }
            window_intent(size, input.viewport(), state.reported_window_size())
        })
    }
}

/// `true` for an event that only the user causes: the pointer, the wheel, the keyboard. Not the
/// gain of the focus, which the opening of the window may give.
fn is_user_action(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::PointerMoved(_)
            | egui::Event::PointerButton { .. }
            | egui::Event::MouseWheel { .. }
            | egui::Event::Key { .. }
            | egui::Event::Text(_)
            | egui::Event::Touch { .. }
    )
}

/// The intent of the size `size` of the window, in points, if it is to be signalled: not when
/// the window is maximized, in full screen or minimized (an unknown state, `None`, counts as
/// not), nor when its surface is null (a minimized window may measure 0 × 0 on Windows without
/// `minimized` being known), nor when it differs by less than [`SIZE_TOLERANCE`] from the last
/// size signalled, `reported`.
fn window_intent(
    size: egui::Vec2,
    viewport: &egui::ViewportInfo,
    reported: (f32, f32),
) -> Option<Intent> {
    let set = |flag: Option<bool>| flag == Some(true);
    if set(viewport.maximized) || set(viewport.fullscreen) || set(viewport.minimized) {
        return None;
    }
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let (width, height) = reported;
    if (size.x - width).abs() < SIZE_TOLERANCE && (size.y - height).abs() < SIZE_TOLERANCE {
        return None;
    }
    Some(Intent::WindowResized {
        width: size.x,
        height: size.y,
    })
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
