//! Helpers of the tests of the presentation: the applications of `examples/`, applications
//! written in temporary folders, and the lock of the tests that reach a log call site.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ortelia_service::app::AppContext;
use tempfile::TempDir;

use crate::{AppState, MainContent, TableRef};

/// `tracing` caches, per call site, whether a subscriber is interested: a test without
/// subscriber that reaches "missing label" first could hide the event from the test that
/// captures the log. The tests that may reach it run one at a time.
static LOG_TESTS: Mutex<()> = Mutex::new(());

pub fn lock_log_tests() -> MutexGuard<'static, ()> {
    // A test that panicked while holding the lock "poisons" it; the lock is still usable.
    LOG_TESTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

pub fn example(path: &str) -> String {
    let path = examples().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The state of the application of `examples/annuaire`, without session state, prepared.
pub fn open_example() -> AppState {
    let mut state = AppState::open(&examples().join("annuaire/app.json"));
    state.prepare();
    state
}

/// The table of the window of an open application.
pub fn table(state: &AppState) -> TableRef<'_> {
    match state.content() {
        MainContent::Table(table) => table,
        MainContent::Nothing => panic!("no table"),
    }
}

/// The application of `examples/annuaire`.
pub fn directory() -> AppContext {
    AppContext::open(&examples().join("annuaire/app.json")).expect("open the directory")
}

/// A folder with the given files, `(name, content)`.
pub fn folder(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temporary folder");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write");
    }
    dir
}

/// The directory application in a temporary folder, with another `app.json` and another
/// `personnes.json`.
pub fn directory_with(app: &str, persons: &str) -> TempDir {
    folder(&[
        ("app.json", app),
        (
            "annuaire.model.json",
            &example("annuaire/annuaire.model.json"),
        ),
        ("personnes.json", persons),
    ])
}

/// `count` persons, numbered, as the text of a `personnes.json`.
pub fn numbered_persons(count: usize) -> String {
    let persons: Vec<String> = (0..count)
        .map(|i| {
            format!(
                r#"{{ "nom": "Nom {i}", "prenom": "Prénom {i}", "date_naissance": "1985-03-12",
                     "nombre_enfants": {}, "taille": 1.7, "contact_autorise": true,
                     "situation": "MARIE", "inscription": "2024-01-15T09:30:00Z" }}"#,
                i % 4
            )
        })
        .collect();
    format!("[{}]", persons.join(",\n"))
}

/// In-memory log output shared between the subscriber and the test.
#[derive(Clone, Default)]
pub struct SharedBuffer(std::sync::Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    /// What was written, as text.
    pub fn text(&self) -> String {
        let bytes = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        String::from_utf8(bytes).expect("UTF-8")
    }
}

impl std::io::Write for SharedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs `f` with a subscriber that captures the log at every level, `trace` included, and
/// returns its result and the log. The caller holds [`lock_log_tests`].
pub fn capture_log<R>(f: impl FnOnce() -> R) -> (R, String) {
    let buffer = SharedBuffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    (result, buffer.text())
}

/// A copy of the application of `examples/annuaire` in a temporary folder: the tests that
/// write a session state file never write in `examples/`.
pub fn directory_copy() -> TempDir {
    directory_with(
        &example("annuaire/app.json"),
        &example("annuaire/personnes.json"),
    )
}
