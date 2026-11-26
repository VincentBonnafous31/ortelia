//! Layer 4: the egui user interface. It only draws the view-models of `ortelia-presentation`
//! (and, from increment 1, turns clicks and keys into intents). No display logic lives here.

use ortelia_presentation::{AppState, MainWindowViewModel};

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
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(&state.main_window().title),
        ..Default::default()
    }
}

/// The eframe application: owns the presentation state and draws it at each frame.
struct OrteliaApp {
    state: AppState,
    /// Set after the first frame, so that "main window shown" is logged only once.
    first_frame_logged: bool,
}

impl OrteliaApp {
    fn new(state: AppState) -> Self {
        OrteliaApp {
            state,
            first_frame_logged: false,
        }
    }
}

impl eframe::App for OrteliaApp {
    /// Called by eframe at each frame (immediate mode: the whole UI is redrawn each time).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        draw_main_window(ui, self.state.main_window());
        if !self.first_frame_logged {
            // The smoke test waits for this line: it proves that a frame was really drawn.
            tracing::info!("main window shown");
            self.first_frame_logged = true;
        }
    }
}

/// Draws the main window. Drawing only: the texts come from the view-model.
fn draw_main_window(ui: &mut egui::Ui, vm: &MainWindowViewModel) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(&vm.heading);
        });
    });
}

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use std::io;
    use std::sync::{Arc, Mutex, MutexGuard};

    /// Serialises the tests that run `OrteliaApp`.
    ///
    /// `tracing` caches, for each `info!` call site, whether a subscriber is interested in it.
    /// While a single subscriber exists, that interest is taken from the subscriber of the
    /// thread that reaches the call site first: if a test without subscriber reaches "main
    /// window shown" first, the cache says "nobody is interested" and the event is lost for
    /// the test that captures it. Running these tests one at a time avoids that.
    static APP_TESTS: Mutex<()> = Mutex::new(());

    /// Also taken by the snapshot of the whole application (`snapshot_tests`).
    pub(crate) fn lock_app_tests() -> MutexGuard<'static, ()> {
        // A test that panicked while holding the lock "poisons" it; the lock is still usable.
        APP_TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn native_window_title_comes_from_presentation() {
        let options = native_options(&AppState::new());
        assert_eq!(options.viewport.title.as_deref(), Some("Ortelia"));
    }

    #[test]
    fn heading_is_displayed() {
        let _lock = lock_app_tests();
        let state = AppState::new();
        // The text itself is tested by the presentation: here, only that it is drawn.
        let expected = state.main_window().heading.clone();
        let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(state));
        harness.run();
        // Panics if no widget has this label in the accessibility tree.
        harness.get_by_label(&expected);
    }

    /// In-memory log output shared between the subscriber and the test.
    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl io::Write for SharedBuffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn first_frame_is_logged_once() {
        let _lock = lock_app_tests();
        let buffer = SharedBuffer::default();
        let writer = buffer.clone();
        // A closure returning a writer is a `MakeWriter` for tracing-subscriber.
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        // Installed for this thread only: no global subscriber shared with other tests.
        tracing::subscriber::with_default(subscriber, || {
            let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(AppState::new()));
            harness.run();
            harness.run_steps(5);
        });
        let log = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert_eq!(log.matches("main window shown").count(), 1, "{log}");
    }
}
