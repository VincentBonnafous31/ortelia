//! Helpers of the tests of the presentation: the applications of `examples/`, applications
//! written in temporary folders, and the lock of the tests that reach a log call site.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ortelia_service::app::AppContext;
use tempfile::TempDir;

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
