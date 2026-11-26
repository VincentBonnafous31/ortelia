//! Layer 3: the presentation layer. It computes *what* to display (view-models, selection
//! broker, intents, labels, formats, session state, design mode) without knowing *how* it is
//! drawn. User interfaces (`ortelia-ui-*`) only read the view-models and draw them.
//!
//! This crate must never depend on a UI library: everything here is tested without a window.
//!
//! Each frame has three steps: [`AppState::prepare`] (brings the indexes up to date), the
//! drawing, which only reads the state (`&AppState`) and gathers intents, then
//! [`AppState::dispatch`] of each intent. Reading and writing apart: the interface never holds
//! a reference to the state while changing it, which is what the borrow checker of Rust asks.

pub mod fold;
pub mod format;
pub mod labels;
pub mod table;

use std::path::Path;

use ortelia_service::app::{AppContext, OpenError, SourceError, SourceStatus};

pub use format::Cell;
pub use ortelia_config::{Align, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH};
pub use table::{Column, TableIntent, TableRef, TableView};

use labels::LabelLookup;

/// The title of the window when no application could be opened.
const DEFAULT_TITLE: &str = "Ortelia";
/// The initial size of the window, when `app.json` does not give it.
const DEFAULT_SIZE: (f32, f32) = (1000.0, 700.0);
/// The number of lines a dialog shows at most; then, the number of the others.
const MAX_DIALOG_LINES: usize = 20;

// The texts of Ortelia itself, in French until increment 4 translates them. The details of the
// errors come from the lower layers, in English.
const TITLE_OPEN_FAILED: &str = "Impossible d'ouvrir l'application";
const TITLE_SOURCES_FAILED: &str = "Certaines données n'ont pas pu être chargées";
const TITLE_WARNINGS: &str = "Avertissements";
const BUTTON_QUIT: &str = "Quitter";
const BUTTON_CONTINUE: &str = "Continuer";
const BUTTON_CLOSE: &str = "Fermer";
/// The hint of the field of the filter of a table.
pub const FILTER_HINT: &str = "Filtrer";
/// The tooltip of the button that clears the filter.
pub const CLEAR_FILTER: &str = "Effacer le filtre";

/// What the user interface asks of the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Table(TableIntent),
    /// Opens the list of the warnings.
    ShowWarnings,
    /// Closes the dialog shown.
    CloseDialog,
    /// Closes the window, and ends the program.
    Quit,
}

/// The main window: its title and its initial size.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowViewModel {
    /// Title requested from the operating system for the native window.
    pub title: String,
    /// Initial size, in points.
    pub width: f32,
    pub height: f32,
}

/// What the main window shows.
pub enum MainContent<'a> {
    Table(TableRef<'a>),
    /// No application: only the dialog of the error.
    Nothing,
}

/// A modal dialog: a title, lines, and a button. It borrows its lines from the state, like
/// [`TableRef`]: nothing is copied at each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogViewModel<'a> {
    pub title: &'static str,
    /// Twenty at most.
    pub lines: &'a [String],
    /// The number of the other lines, as a text: `… et 5 autres`.
    pub more: Option<String>,
    pub button: &'static str,
    /// The intent of the button.
    pub button_intent: Intent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogKind {
    OpenFailed,
    SourcesFailed,
    Warnings,
}

/// The state of the application shown by the user interface.
#[derive(Debug)]
pub struct AppState {
    window: WindowViewModel,
    /// `None` if the opening failed.
    app: Option<OpenedApp>,
    dialog: Option<DialogKind>,
    /// The errors of the opening, one per line.
    open_errors: Vec<String>,
    /// The errors of the sources that failed, one per line.
    source_errors: Vec<String>,
    /// The warnings: configuration, missing labels, diagnostics of the sources.
    warnings: Vec<String>,
    /// The text of the status bar: the number of warnings, the diagnostics not listed
    /// included; `None` without warning.
    status_bar: Option<String>,
    quit_requested: bool,
}

#[derive(Debug)]
struct OpenedApp {
    context: AppContext,
    table: TableView,
}

impl AppState {
    /// Opens the application of the `app.json` at `path`. Cannot fail: an application that does
    /// not open gives the state "in error", with the dialog of its errors.
    pub fn open(path: &Path) -> AppState {
        match AppContext::open(path) {
            Ok(context) => AppState::opened(context),
            Err(error) => AppState::with_open_errors(open_error_lines(&error)),
        }
    }

    /// The state of an application that could not be opened, with its errors.
    pub fn with_open_errors(errors: Vec<String>) -> AppState {
        AppState {
            window: WindowViewModel {
                title: DEFAULT_TITLE.to_owned(),
                width: DEFAULT_SIZE.0,
                height: DEFAULT_SIZE.1,
            },
            app: None,
            dialog: Some(DialogKind::OpenFailed),
            open_errors: errors,
            source_errors: Vec::new(),
            warnings: Vec::new(),
            status_bar: None,
            quit_requested: false,
        }
    }

    fn opened(context: AppContext) -> AppState {
        let app = context.app();
        let model = context.document().model();
        let lang = &app.languages[0];
        let mut lookup = LabelLookup::new(&app.labels, model, lang);
        let table = TableView::new(app, model, &mut lookup);

        let mut warnings: Vec<String> = context.warnings().iter().map(|w| w.to_string()).collect();
        let mut warning_count = warnings.len();
        for key in lookup.missing() {
            warnings.push(format!("missing label `{key}` ({lang})"));
            warning_count += 1;
        }
        let mut source_errors = Vec::new();
        for source in context.sources() {
            let id = &source.id;
            match &source.status {
                SourceStatus::Loaded {
                    diagnostics,
                    diagnostic_count,
                    ..
                } => {
                    warnings.extend(diagnostics.iter().map(|d| format!("source `{id}`: {d}")));
                    if *diagnostic_count > diagnostics.len() {
                        let others = diagnostic_count - diagnostics.len();
                        warnings.push(format!("source `{id}`: {others} more diagnostics"));
                    }
                    warning_count += diagnostic_count;
                }
                SourceStatus::Failed(SourceError::Data(failure)) => {
                    source_errors.extend(
                        failure
                            .errors
                            .iter()
                            .map(|e| format!("source `{id}`: {}", e.user_message())),
                    );
                    if failure.error_count > failure.errors.len() {
                        let others = failure.error_count - failure.errors.len();
                        source_errors.push(format!("source `{id}`: {others} more errors"));
                    }
                }
                SourceStatus::Failed(error) => {
                    source_errors.push(format!("source `{id}`: {error}"));
                }
            }
        }

        let window = &app.window;
        AppState {
            window: WindowViewModel {
                title: window.title.clone(),
                width: window.width.map_or(DEFAULT_SIZE.0, |w| w as f32),
                height: window.height.map_or(DEFAULT_SIZE.1, |h| h as f32),
            },
            dialog: (!source_errors.is_empty()).then_some(DialogKind::SourcesFailed),
            app: Some(OpenedApp { context, table }),
            open_errors: Vec::new(),
            source_errors,
            warnings,
            status_bar: match warning_count {
                0 => None,
                1 => Some("⚠ 1 avertissement".to_owned()),
                n => Some(format!("⚠ {n} avertissements")),
            },
            quit_requested: false,
        }
    }

    /// The user interface only gets a shared reference: like a `const&` in C++, it cannot
    /// modify the view-model, which therefore needs no setters.
    pub fn window(&self) -> &WindowViewModel {
        &self.window
    }

    /// Brings the indexes up to date, at the beginning of a frame.
    pub fn prepare(&mut self) {
        if let Some(app) = &mut self.app {
            app.table.prepare(app.context.document());
        }
    }

    /// What the window shows.
    pub fn content(&self) -> MainContent<'_> {
        match &self.app {
            Some(app) => MainContent::Table(app.table.read(app.context.document())),
            None => MainContent::Nothing,
        }
    }

    /// The dialog shown over the window, if any.
    pub fn dialog(&self) -> Option<DialogViewModel<'_>> {
        let (title, lines, button, button_intent) = match self.dialog? {
            DialogKind::OpenFailed => (
                TITLE_OPEN_FAILED,
                &self.open_errors,
                BUTTON_QUIT,
                Intent::Quit,
            ),
            DialogKind::SourcesFailed => (
                TITLE_SOURCES_FAILED,
                &self.source_errors,
                BUTTON_CONTINUE,
                Intent::CloseDialog,
            ),
            DialogKind::Warnings => (
                TITLE_WARNINGS,
                &self.warnings,
                BUTTON_CLOSE,
                Intent::CloseDialog,
            ),
        };
        let others = lines.len().saturating_sub(MAX_DIALOG_LINES);
        Some(DialogViewModel {
            title,
            lines: &lines[..lines.len().min(MAX_DIALOG_LINES)],
            more: match others {
                0 => None,
                1 => Some("… et 1 autre".to_owned()),
                n => Some(format!("… et {n} autres")),
            },
            button,
            button_intent,
        })
    }

    /// The text of the status bar: the number of warnings; `None` without warning, and then
    /// no status bar.
    pub fn status_bar(&self) -> Option<&str> {
        self.status_bar.as_deref()
    }

    /// Applies an intent of the user interface.
    pub fn dispatch(&mut self, intent: Intent) {
        match intent {
            Intent::Table(intent) => {
                if let Some(app) = &mut self.app {
                    app.table.apply(intent);
                }
            }
            Intent::ShowWarnings => {
                if self.dialog.is_none() && self.status_bar.is_some() {
                    self.dialog = Some(DialogKind::Warnings);
                }
            }
            Intent::CloseDialog => {
                // The dialog of an opening that failed only closes with the window.
                if self.dialog != Some(DialogKind::OpenFailed) {
                    self.dialog = None;
                }
            }
            Intent::Quit => self.quit_requested = true,
        }
    }

    /// `true` once the user asked to quit: the interface closes the window.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// The errors of an opening that failed, for the standard error and the log.
    pub fn open_errors(&self) -> &[String] {
        &self.open_errors
    }

    /// The exit code of the program: 1 if the application could not be opened, 0 otherwise,
    /// whatever the way the window is closed.
    pub fn exit_code(&self) -> u8 {
        if self.app.is_some() { 0 } else { 1 }
    }
}

/// The errors of an opening, one per line, each with the file in cause.
fn open_error_lines(error: &OpenError) -> Vec<String> {
    match error {
        OpenError::App { file, errors } => errors
            .iter()
            .map(|e| format!("{}: {e}", file.display()))
            .collect(),
        other => vec![other.to_string()],
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
