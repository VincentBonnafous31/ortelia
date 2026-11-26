//! Tests of the session state of the application (specifications `session-state` and
//! `app-launch`): computed, restored at the opening and saved at the closing, on copies of the
//! annuaire in temporary folders.

use std::path::{Path, PathBuf};

use ortelia_config::SortOrder;
use tempfile::TempDir;

use super::*;
use crate::session::{FORMAT_VERSION, SortSession};
use crate::test_support::{
    capture_log, directory_copy, folder, lock_log_tests, open_example, table,
};

fn app_file(dir: &TempDir) -> PathBuf {
    dir.path().join("app.json")
}

fn state_file(dir: &TempDir) -> PathBuf {
    dir.path().join("app.session.json")
}

/// A copy of the annuaire with the session state file `saved`.
fn directory_with_session(saved: &str) -> TempDir {
    let dir = directory_copy();
    std::fs::write(state_file(&dir), saved).expect("write");
    dir
}

/// The size of the window of the session state.
fn saved_window(state: &AppState) -> (Option<f32>, Option<f32>) {
    let window = state.session().expect("open").window.expect("window");
    (window.width, window.height)
}

/// The state file of `dir` was written, in the format of this version.
fn assert_state_file_written(dir: &TempDir) {
    let text = std::fs::read_to_string(state_file(dir)).expect("state file");
    let json: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(json["formatVersion"], FORMAT_VERSION);
}

fn open_with_session(path: &Path) -> AppState {
    let mut state = AppState::open_with_session(path);
    state.prepare();
    state
}

/// The headers of the first columns.
fn headers(state: &AppState, count: usize) -> Vec<String> {
    table(state).columns()[..count]
        .iter()
        .map(|c| c.header().to_owned())
        .collect()
}

/// The column of the header `header`.
fn column(state: &AppState, header: &str) -> usize {
    table(state)
        .columns()
        .iter()
        .position(|c| c.header() == header)
        .expect("column")
}

/// The width of the column of the header `header`.
fn width(state: &AppState, header: &str) -> f32 {
    table(state).columns()[column(state, header)].width()
}

/// The table of the state has the layout of `app.json`.
fn assert_layout_of_app_json(state: &AppState) {
    let fresh = open_example();
    assert_eq!(state.window(), fresh.window());
    let saved = state.session().expect("open");
    assert_eq!(saved, fresh.session().expect("open"));
}

const SAVED_LAYOUT: &str = r#"{
  "formatVersion": "1",
  "window": { "width": 1200, "height": 800 },
  "views": [
    {
      "id": "personnes",
      "table": {
        "columns": [ { "path": "prenom", "width": 120 }, { "path": "nom", "width": 180 } ],
        "sort": [ { "path": "nom", "order": "desc" } ],
        "filter": "dup"
      }
    }
  ]
}"#;

/// Scenario "Disposition modifiée".
#[test]
fn modified_layout() {
    let mut state = open_example();
    state.dispatch(Intent::WindowResized {
        width: 1200.0,
        height: 800.0,
    });
    state.dispatch(Intent::Table(TableIntent::MoveColumn {
        from: column(&state, "Né le"),
        to: 0,
    }));
    state.dispatch(Intent::Table(TableIntent::ResizeColumn {
        column: column(&state, "Nom"),
        width: 200.0,
    }));
    state.dispatch(Intent::Table(TableIntent::SortBy {
        column: column(&state, "Situation"),
        add: false,
    }));
    for _ in 0..2 {
        state.dispatch(Intent::Table(TableIntent::SortBy {
            column: column(&state, "Nom"),
            add: true,
        }));
    }
    state.dispatch(Intent::Table(TableIntent::SetFilter("dup".into())));
    assert_eq!(saved_window(&state), (Some(1200.0), Some(800.0)));
    let saved = state.session().expect("open");
    assert_eq!(saved.views.len(), 1);
    assert_eq!(saved.views[0].id, "personnes");
    // The columns, the sort and the filter, as the table gives them (checked by the tests of
    // `TableView::session`).
    let app = state.app.as_ref().expect("open");
    let table = app.table.session(app.context.document().model());
    assert_eq!(table.columns[0].path, "date_naissance");
    assert_eq!(table.filter, "dup");
    assert_eq!(saved.views[0].table, Some(table));
}

/// Scenario "Disposition inchangée".
#[test]
fn unchanged_layout() {
    let state = open_example();
    assert_eq!(saved_window(&state), (Some(1000.0), Some(700.0)));
    let saved = state.session().expect("open");
    let table = saved.views[0].table.as_ref().expect("table");
    assert_eq!(table.columns.len(), 8);
    assert_eq!(
        (table.columns[0].path.as_str(), table.columns[0].width),
        ("nom", Some(140.0))
    );
    assert_eq!(
        table.sort,
        Some(vec![
            SortSession {
                path: "nom".into(),
                order: SortOrder::Asc
            },
            SortSession {
                path: "prenom".into(),
                order: SortOrder::Asc
            }
        ])
    );
    assert_eq!(table.filter, "");
}

/// Scenario "Fenêtre redimensionnée", in the presentation.
#[test]
fn window_resized() {
    let mut state = open_example();
    assert_eq!(state.reported_window_size(), (1000.0, 700.0));
    state.dispatch(Intent::WindowResized {
        width: 1200.0,
        height: 800.0,
    });
    assert_eq!(state.reported_window_size(), (1200.0, 800.0));
    assert_eq!(saved_window(&state), (Some(1200.0), Some(800.0)));
    // The initial size does not change.
    assert_eq!(state.window().width, 1000.0);
}

/// Scenario "Taille hors bornes"; a size that is not finite is ignored.
#[test]
fn size_out_of_bounds() {
    let mut state = open_example();
    state.dispatch(Intent::WindowResized {
        width: 100.0,
        height: 20_000.0,
    });
    assert_eq!(saved_window(&state), (Some(200.0), Some(10_000.0)));
    // Kept as received: the interface, which compares it to the real size, does not send it
    // again.
    assert_eq!(state.reported_window_size(), (100.0, 20_000.0));
    state.dispatch(Intent::WindowResized {
        width: f32::NAN,
        height: 800.0,
    });
    assert_eq!(state.reported_window_size(), (100.0, 20_000.0));
}

/// Scenario "Fichier de l'annuaire".
#[test]
fn file_of_the_directory() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let state = open_with_session(&app_file(&dir));
    state.save_session();
    assert_state_file_written(&dir);
}

/// Scenario "Fichier d'application d'un autre nom".
#[test]
fn file_of_another_name() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let app = dir.path().join("annuaire.json");
    std::fs::rename(app_file(&dir), &app).expect("rename");
    let state = open_with_session(&app);
    assert_eq!(state.exit_code(), 0);
    state.save_session();
    assert!(dir.path().join("annuaire.session.json").is_file());
    assert!(!state_file(&dir).exists());
}

/// Scenarios "Disposition retrouvée" and "Taille de l'état de session" (`app-launch`).
#[test]
fn layout_restored() {
    let _lock = lock_log_tests();
    let dir = directory_with_session(SAVED_LAYOUT);
    let (state, log) = capture_log(|| open_with_session(&app_file(&dir)));
    assert_eq!(headers(&state, 2), ["Prénom", "Nom"]);
    assert_eq!(width(&state, "Nom"), 180.0);
    let table = table(&state);
    assert_eq!(table.counter(), "1 / 10");
    assert_eq!(table.cell(0, 1), Cell::Text("Dupont".into()));
    assert_eq!(table.filter(), "dup");
    assert_eq!(table.sort_indicator(1).as_deref(), Some("⏷"));
    assert_eq!(
        state.window(),
        &WindowViewModel {
            title: "Annuaire".into(),
            width: 1200.0,
            height: 800.0
        }
    );
    let restored = log
        .lines()
        .find(|l| l.contains("session state restored"))
        .unwrap_or_else(|| panic!("{log}"));
    assert!(
        restored.contains(" INFO ") && restored.contains("views=1"),
        "{log}"
    );
}

/// Scenario "Pas de fichier".
#[test]
fn no_file() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let (state, log) = capture_log(|| open_with_session(&app_file(&dir)));
    assert_layout_of_app_json(&state);
    assert_eq!(state.status_bar(), None);
    assert!(!log.contains(" WARN "), "{log}");
}

/// Scenario "Ouverture sans état de session".
#[test]
fn opening_without_session_state() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let saved = r#"{ "formatVersion": "1", "views": [ { "id": "personnes",
                     "table": { "sort": [ { "path": "prenom" } ] } } ] }"#;
    std::fs::write(state_file(&dir), saved).expect("write");
    let mut state = AppState::open(&app_file(&dir));
    state.prepare();
    assert_layout_of_app_json(&state);
    state.save_session();
    assert_eq!(
        std::fs::read_to_string(state_file(&dir)).expect("read"),
        saved
    );
}

/// Scenario "Valeurs ramenées dans leurs bornes", for the window; a size missing or `null` is
/// ignored.
#[test]
fn window_size_within_bounds() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let saved = r#"{ "formatVersion": "1", "window": { "width": 50, "height": 50000 } }"#;
    std::fs::write(state_file(&dir), saved).expect("write");
    let state = open_with_session(&app_file(&dir));
    assert_eq!(
        (state.window().width, state.window().height),
        (200.0, 10_000.0)
    );
    let saved = r#"{ "formatVersion": "1", "window": { "width": null } }"#;
    std::fs::write(state_file(&dir), saved).expect("write");
    let state = open_with_session(&app_file(&dir));
    assert_eq!(
        (state.window().width, state.window().height),
        (1000.0, 700.0)
    );
}

/// Scenario "Vue inconnue".
#[test]
fn unknown_view() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let saved = r#"{ "formatVersion": "1", "views": [
        { "id": "contacts", "table": { "sort": [] } },
        { "id": "personnes", "table": { "sort": [ { "path": "nom", "order": "desc" } ] } } ] }"#;
    std::fs::write(state_file(&dir), saved).expect("write");
    let (state, log) = capture_log(|| open_with_session(&app_file(&dir)));
    assert_eq!(table(&state).cell(0, 0), Cell::Text("Robert".into()));
    assert_eq!(state.status_bar(), None);
    assert!(!log.contains(" WARN "), "{log}");
    // Only the views of the application are saved again.
    let views = state.session().expect("open").views;
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].id, "personnes");
}

/// A view of the same identifier without table, written by a newer version, before the table.
#[test]
fn view_without_table_first() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let saved = r#"{ "formatVersion": "1", "views": [
        { "id": "personnes", "form": {} },
        { "id": "personnes", "table": { "sort": [ { "path": "nom", "order": "desc" } ] } } ] }"#;
    std::fs::write(state_file(&dir), saved).expect("write");
    let state = open_with_session(&app_file(&dir));
    assert_eq!(table(&state).cell(0, 0), Cell::Text("Robert".into()));
}

/// Scenario "Clé inconnue".
#[test]
fn unknown_keys() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let saved = SAVED_LAYOUT
        .replace(
            r#""formatVersion": "1","#,
            r#""formatVersion": "1", "language": "en","#,
        )
        .replace(
            r#""id": "personnes","#,
            r#""id": "personnes", "split": 0.3,"#,
        );
    std::fs::write(state_file(&dir), saved).expect("write");
    let state = open_with_session(&app_file(&dir));
    assert_eq!(headers(&state, 2), ["Prénom", "Nom"]);
    assert_eq!(table(&state).filter(), "dup");
    assert_eq!(state.window().width, 1200.0);
}

/// Scenarios "JSON invalide" and "Autre version de format", for the application: the layout of
/// `app.json`.
#[test]
fn file_ignored() {
    let _lock = lock_log_tests();
    for saved in [r#"{ "formatVersion": "1", "#, r#"{ "formatVersion": "2" }"#] {
        let dir = directory_with_session(saved);
        let (state, log) = capture_log(|| open_with_session(&app_file(&dir)));
        assert_layout_of_app_json(&state);
        assert!(log.contains("session state ignored"), "{log}");
    }
}

/// Scenario "Fichier ignoré puis remplacé".
#[test]
fn file_ignored_then_replaced() {
    let _lock = lock_log_tests();
    let dir = directory_with_session("{ ");
    let state = open_with_session(&app_file(&dir));
    state.save_session();
    assert_state_file_written(&dir);
}

/// Scenario "Application qui ne s'ouvre pas" of "Enregistrement à la fermeture".
#[test]
fn application_that_does_not_open() {
    let _lock = lock_log_tests();
    let dir = folder(&[]);
    let state = AppState::open_with_session(&app_file(&dir));
    assert_eq!(state.exit_code(), 1);
    assert_eq!(state.session(), None);
    state.save_session();
    assert_eq!(std::fs::read_dir(dir.path()).expect("read").count(), 0);
}

/// Scenario "`app.json` intact".
#[test]
fn app_json_unchanged() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let before = std::fs::read(app_file(&dir)).expect("read");
    let mut state = open_with_session(&app_file(&dir));
    state.dispatch(Intent::Table(TableIntent::ResizeColumn {
        column: 0,
        width: 300.0,
    }));
    state.save_session();
    assert_eq!(std::fs::read(app_file(&dir)).expect("read"), before);
}

/// Scenario "Filtre enregistré puis restauré".
#[test]
fn filter_saved_then_restored() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let ((), log) = capture_log(|| {
        let mut state = open_with_session(&app_file(&dir));
        state.dispatch(Intent::Table(TableIntent::SetFilter("dup".into())));
        state.prepare();
        state.save_session();
        let mut state = open_with_session(&app_file(&dir));
        state.prepare();
        assert_eq!(table(&state).filter(), "dup");
    });
    let text = std::fs::read_to_string(state_file(&dir)).expect("state file");
    assert!(text.contains(r#""filter": "dup""#), "{text}");
    assert!(log.contains("session state saved"), "{log}");
    assert!(log.contains("session state restored"), "{log}");
    // The temporary folder has a random name, which could hold the text by chance: the paths
    // are removed before the check.
    let log = log.replace(&dir.path().display().to_string(), "<dir>");
    assert!(!log.contains("dup"), "{log}");
}
