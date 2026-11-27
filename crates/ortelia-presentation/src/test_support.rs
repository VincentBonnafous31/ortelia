//! Helpers of the tests of the presentation: the applications of `examples/`, applications
//! written in temporary folders, and the lock of the tests that reach a log call site.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ortelia_service::app::AppContext;
use tempfile::TempDir;

use crate::{
    AppState, Cell, FormRef, LayoutNode, MainContent, ObjectId, SplitNode, TableRef, ViewRef,
};

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

/// The layout of the window of an open application.
pub fn layout(state: &AppState) -> &LayoutNode {
    let MainContent::Layout(layout) = state.content() else {
        panic!("no application")
    };
    layout
}

/// The first table of the window of an open application.
pub fn table(state: &AppState) -> TableRef<'_> {
    layout(state)
        .views()
        .into_iter()
        .find_map(|view| match state.view(view.id()) {
            Some(ViewRef::Table(table)) => Some(table),
            _ => None,
        })
        .expect("a table view")
}

/// The split `id` of the window of an open application.
pub fn split<'a>(state: &'a AppState, id: &str) -> &'a SplitNode {
    layout(state)
        .split(id)
        .unwrap_or_else(|| panic!("no split {id}"))
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

/// The directory application in a temporary folder, whose window has another content, given
/// as JSON: other splits and views of the persons.
pub fn directory_with_content(content: &str) -> TempDir {
    let content: serde_json::Value = serde_json::from_str(content).expect("content");
    directory_edited(|app| app["window"]["content"] = content)
}

/// The directory application in a temporary folder, with its `app.json` changed by `edit`.
pub fn directory_edited(edit: impl FnOnce(&mut serde_json::Value)) -> TempDir {
    directory_edited_with(edit, &example("annuaire/personnes.json"))
}

/// The directory application in a temporary folder, with its `app.json` changed by `edit`, and
/// the persons `persons`.
pub fn directory_edited_with(edit: impl FnOnce(&mut serde_json::Value), persons: &str) -> TempDir {
    let mut app: serde_json::Value =
        serde_json::from_str(&example("annuaire/app.json")).expect("app.json");
    edit(&mut app);
    directory_with(&serde_json::to_string_pretty(&app).expect("JSON"), persons)
}

/// The form of the view `id` of an open application.
pub fn form<'a>(state: &'a AppState, id: &str) -> FormRef<'a> {
    match state.view(id) {
        Some(ViewRef::Form(form)) => form,
        _ => panic!("no form view {id}"),
    }
}

/// The person of the table of an open application whose `nom` (first column) is `name`.
pub fn person(state: &AppState, name: &str) -> ObjectId {
    let table = table(state);
    (0..table.row_count())
        .find(|&row| table.cell(row, 0) == Cell::Text(name.to_owned()))
        .and_then(|row| table.object(row))
        .unwrap_or_else(|| panic!("no person {name}"))
}

/// A view of the persons in a table, as JSON.
pub fn table_json(id: &str) -> String {
    format!(
        r#"{{ "_type_": "View", "id": "{id}", "widget": {{ "_type_": "Table" }},
             "data": "/annuaire/personnes" }}"#
    )
}

/// An empty view, as JSON.
pub fn empty_json(id: &str) -> String {
    format!(r#"{{ "_type_": "View", "id": "{id}", "widget": {{ "_type_": "Empty" }} }}"#)
}

/// A split, as JSON: `weights` is `""` or `"\"weights\": [...],"`.
pub fn split_json(id: &str, direction: &str, weights: &str, children: &[String]) -> String {
    format!(
        r#"{{ "_type_": "Split", "id": "{id}", "direction": "{direction}", {weights}
             "children": [ {} ] }}"#,
        children.join(", ")
    )
}

/// The shares `parts` are `expected`, to a millionth.
pub fn assert_parts(parts: &[f32], expected: &[f32]) {
    assert_eq!(parts.len(), expected.len(), "{parts:?}");
    for (part, expected) in parts.iter().zip(expected) {
        assert!((part - expected).abs() < 1e-6, "{parts:?} != {expected:?}");
    }
}

/// The state of an application of `dir`, without session state, prepared.
pub fn open_dir(dir: &TempDir) -> AppState {
    let mut state = AppState::open(&dir.path().join("app.json"));
    state.prepare();
    state
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
