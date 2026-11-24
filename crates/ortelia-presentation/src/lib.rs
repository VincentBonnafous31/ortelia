//! Layer 3: the presentation layer. It computes *what* to display (view-models, selection
//! broker, intents, labels, formats, session state, design mode) without knowing *how* it is
//! drawn. User interfaces (`ortelia-ui-*`) only read the view-models and draw them.
//!
//! This crate must never depend on a UI library: everything here is tested without a window.

/// Application state shown by the user interface.
///
/// Increment 1 will add `dispatch(intent)`, a revision number and the view-models of the views.
#[derive(Debug)]
pub struct AppState {
    main_window: MainWindowViewModel,
}

impl AppState {
    pub fn new() -> Self {
        // CARGO_PKG_VERSION is set by Cargo at compile time (like a -D define passed to a C
        // compiler). All crates share the workspace version, so it is the application version.
        let version = env!("CARGO_PKG_VERSION");
        AppState {
            main_window: MainWindowViewModel {
                title: "Ortelia".to_string(),
                heading: format!("Ortelia {version}"),
            },
        }
    }

    /// The user interface only gets a shared reference: like a `const&` in C++, it cannot
    /// modify the view-model, which therefore needs no getters.
    pub fn main_window(&self) -> &MainWindowViewModel {
        &self.main_window
    }
}

// `Default` is the idiomatic Rust name for a no-argument constructor; clippy asks for it
// whenever a type has `new()` without arguments.
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Texts of the main window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainWindowViewModel {
    /// Title requested from the operating system for the native window.
    pub title: String,
    /// Text shown in the centre of the window.
    pub heading: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_window_texts() {
        let state = AppState::new();
        let vm = state.main_window();
        assert_eq!(vm.title, "Ortelia");
        assert_eq!(vm.heading, format!("Ortelia {}", env!("CARGO_PKG_VERSION")));
    }
}
