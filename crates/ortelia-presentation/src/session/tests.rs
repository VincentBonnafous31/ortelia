//! Tests of the session state file (specification `session-state`): its path, its reading and
//! its writing, in temporary folders.

use std::path::Path;

use super::*;
use crate::test_support::{capture_log, directory_copy, folder, lock_log_tests};

/// A valid state: the window and the sort of the view `personnes`.
fn state(order: SortOrder) -> SessionFile {
    SessionFile {
        format_version: FORMAT_VERSION.to_owned(),
        window: Some(WindowSession {
            width: Some(1200.0),
            height: Some(800.0),
        }),
        views: vec![ViewSession {
            id: "personnes".to_owned(),
            table: Some(TableSession {
                columns: vec![ColumnSession {
                    path: "nom".to_owned(),
                    width: Some(180.0),
                }],
                sort: Some(vec![SortSession {
                    path: "nom".to_owned(),
                    order,
                }]),
                filter: "dup".to_owned(),
            }),
        }],
    }
}

/// The names of the files of a folder, sorted.
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read the folder")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Loads a file of the given content, and returns the result, the log and the folder.
fn load_text(text: &str) -> (Option<SessionFile>, String, tempfile::TempDir) {
    let dir = folder(&[("app.session.json", text)]);
    let (file, log) = capture_log(|| load(&dir.path().join("app.session.json")));
    (file, log, dir)
}

/// The first `WARN` line of `log`.
fn warning(log: &str) -> &str {
    log.lines()
        .find(|l| l.contains(" WARN "))
        .unwrap_or_else(|| panic!("{log}"))
}

#[test]
fn file_of_app_json() {
    assert_eq!(
        file_for(Path::new("dossier/app.json")),
        Path::new("dossier/app.session.json")
    );
}

/// Scenario "Fichier d'application d'un autre nom".
#[test]
fn file_of_another_name() {
    assert_eq!(
        file_for(Path::new("D/annuaire.json")),
        Path::new("D/annuaire.session.json")
    );
}

/// The extension in capitals is `.json` too.
#[test]
fn file_of_an_extension_in_capitals() {
    assert_eq!(
        file_for(Path::new("D/APP.JSON")),
        Path::new("D/APP.session.json")
    );
}

/// The application file is never overwritten, whatever its name.
#[test]
fn file_without_json_extension() {
    assert_eq!(
        file_for(Path::new("D/app")),
        Path::new("D/app.session.json")
    );
    assert_eq!(
        file_for(Path::new("D/app.txt")),
        Path::new("D/app.txt.session.json")
    );
}

/// What is saved is read back.
#[test]
fn save_then_load() {
    let _lock = lock_log_tests();
    let dir = folder(&[]);
    let path = dir.path().join("app.session.json");
    save(&path, &state(SortOrder::Desc));
    assert_eq!(load(&path), Some(state(SortOrder::Desc)));
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains(r#""formatVersion": "1""#), "{text}");
    assert!(text.contains(r#""order": "desc""#), "{text}");
}

/// Scenario "Remplacement d'un état existant".
#[test]
fn replace_an_existing_state() {
    let _lock = lock_log_tests();
    let dir = directory_copy();
    let path = dir.path().join("app.session.json");
    save(&path, &state(SortOrder::Asc));
    save(&path, &state(SortOrder::Desc));
    assert_eq!(load(&path), Some(state(SortOrder::Desc)));
    assert_eq!(
        file_names(dir.path()),
        [
            "annuaire.model.json",
            "app.json",
            "app.session.json",
            "personnes.json"
        ]
    );
}

/// Scenario "Écriture impossible".
#[test]
fn write_impossible() {
    let _lock = lock_log_tests();
    let dir = folder(&[("ordinaire.txt", "")]);
    let path = dir.path().join("ordinaire.txt").join("app.session.json");
    let ((), log) = capture_log(|| save(&path, &state(SortOrder::Asc)));
    let warning = warning(&log);
    assert!(warning.contains("cannot save the session state"), "{log}");
    assert!(warning.contains(&path.display().to_string()), "{log}");
    assert_eq!(file_names(dir.path()), ["ordinaire.txt"]);
}

/// No file: `None`, without warning.
#[test]
fn no_file() {
    let _lock = lock_log_tests();
    let dir = folder(&[]);
    let (file, log) = capture_log(|| load(&dir.path().join("app.session.json")));
    assert_eq!(file, None);
    assert!(!log.contains(" WARN "), "{log}");
}

/// Scenario "JSON invalide", for the file.
#[test]
fn invalid_json() {
    let _lock = lock_log_tests();
    let (file, log, dir) = load_text(r#"{ "formatVersion": "1", "#);
    let path = dir.path().join("app.session.json");
    assert_eq!(file, None);
    let warning = warning(&log);
    assert!(warning.contains(&path.display().to_string()), "{log}");
    assert!(
        warning.contains("line=1") && warning.contains("column="),
        "{log}"
    );
}

/// Scenario "Autre version de format", for the file.
#[test]
fn another_format_version() {
    let _lock = lock_log_tests();
    let (file, log, dir) = load_text(r#"{ "formatVersion": "2", "views": 3 }"#);
    let path = dir.path().join("app.session.json");
    assert_eq!(file, None);
    let warning = warning(&log);
    assert!(warning.contains(&path.display().to_string()), "{log}");
    assert!(warning.contains("unsupported format version"), "{log}");
    assert!(warning.contains("version=2"), "{log}");
}

/// The version cited is cut, and a version missing is refused.
#[test]
fn long_or_missing_version() {
    let _lock = lock_log_tests();
    let (file, log, _dir) = load_text(&format!(r#"{{ "formatVersion": "{}" }}"#, "9".repeat(40)));
    assert_eq!(file, None);
    assert!(
        log.contains(&format!("version={}", "9".repeat(16))),
        "{log}"
    );
    assert!(!log.contains(&"9".repeat(17)), "{log}");
    let (file, log, _dir) = load_text(r#"{ "views": [] }"#);
    assert_eq!(file, None);
    assert!(log.contains("version=none"), "{log}");
}

/// A value of a wrong type: the whole file is ignored, with its position.
#[test]
fn value_of_a_wrong_type() {
    let _lock = lock_log_tests();
    let text = r#"{ "formatVersion": "1",
                    "window": { "width": "large" } }"#;
    let (file, log, _dir) = load_text(text);
    assert_eq!(file, None);
    assert!(log.contains("value of a wrong type"), "{log}");
    assert!(log.contains("line=2"), "{log}");
    assert!(!log.contains("large"), "{log}");
}

/// Scenario "Filtre d'un type faux".
#[test]
fn filter_of_a_wrong_type() {
    let _lock = lock_log_tests();
    let text = r#"{ "formatVersion": "1",
                    "views": [ { "id": "personnes", "table": { "filter": ["dup"] } } ] }"#;
    let (file, log, _dir) = load_text(text);
    assert_eq!(file, None);
    assert!(log.contains(" WARN "), "{log}");
    assert!(!log.contains("dup"), "{log}");
}

/// Scenario "Clé inconnue", for the file.
#[test]
fn unknown_keys() {
    let _lock = lock_log_tests();
    let text = r#"{ "formatVersion": "1", "language": "en",
                    "window": { "width": 1200, "height": 800, "maximized": true },
                    "views": [ { "id": "personnes", "split": 0.5,
                                 "table": { "columns": [ { "path": "nom", "width": 180,
                                                           "visible": true } ],
                                            "sort": [ { "path": "nom", "order": "desc" } ],
                                            "filter": "dup" } } ] }"#;
    let (file, log, _dir) = load_text(text);
    assert_eq!(file, Some(state(SortOrder::Desc)));
    assert!(!log.contains(" WARN "), "{log}");
}

/// Every key but `formatVersion` is optional, and a `null` width is ignored.
#[test]
fn optional_keys() {
    let _lock = lock_log_tests();
    let text = r#"{ "formatVersion": "1",
                    "views": [ { "id": "personnes",
                                 "table": { "columns": [ { "path": "nom", "width": null } ] } } ] }"#;
    let (file, _log, _dir) = load_text(text);
    let file = file.expect("valid");
    assert_eq!(file.window, None);
    let table = file.views[0].table.as_ref().expect("table");
    assert_eq!(table.columns[0].width, None);
    assert_eq!(table.sort, None);
    assert_eq!(table.filter, "");
}
