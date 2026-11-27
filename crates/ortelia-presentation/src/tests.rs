//! Tests of the state of the application (specifications `app-launch` and `labels`): opened or
//! in error, dialogs, status bar, intents.

use super::*;
use crate::test_support::{
    capture_log, directory_with, example, folder, lock_log_tests, open_dir, open_example, table,
};

/// The annuaire without the French labels of `Personne.prenom` and `SituationFamiliale.VEUF`.
fn two_missing_labels() -> String {
    // Their entries take other keys, valid and unused: the file stays valid.
    example("annuaire/app.json")
        .replace(
            r#""key": "Personne.prenom", "text": "Prénom""#,
            r#""key": "Annuaire", "text": "Annuaire""#,
        )
        .replace(
            r#""key": "SituationFamiliale.VEUF", "text": "Veuf, veuve""#,
            r#""key": "Racine", "text": "Racine""#,
        )
}

/// Scenario "Textes de la fenêtre principale".
#[test]
fn texts_of_the_main_window() {
    let state = open_example();
    assert_eq!(
        state.window(),
        &WindowViewModel {
            title: "Annuaire".into(),
            width: 1000.0,
            height: 700.0
        }
    );
    // The layout itself is checked by the tests of `layout`.
    let table = table(&state);
    assert_eq!(table.id(), "personnes");
    assert_eq!(table.row_count(), 10);
    assert_eq!(table.columns()[0].header(), "Nom");
    assert_eq!(state.dialog(), None);
    assert_eq!(state.exit_code(), 0);
}

/// Scenarios "Application qui ne s'ouvre pas" and "Dossier sans `app.json`".
#[test]
fn application_that_does_not_open() {
    let dir = folder(&[]);
    let state = open_dir(&dir);
    assert_eq!(state.window().title, "Ortelia");
    assert!(matches!(state.content(), MainContent::Nothing));
    let dialog = state.dialog().expect("dialog");
    assert_eq!(dialog.title, "Impossible d'ouvrir l'application");
    assert_eq!(dialog.button, "Quitter");
    assert_eq!(dialog.button_intent, Intent::Quit);
    assert_eq!(dialog.lines.len(), 1);
    assert!(dialog.lines[0].contains("app.json"), "{:?}", dialog.lines);
    assert_eq!(state.open_errors(), dialog.lines);
    assert_eq!(state.exit_code(), 1);
    assert_eq!(state.status_bar(), None);
}

/// Each error of `app.json` is a line, which names the file.
#[test]
fn errors_of_app_json() {
    let app = example("annuaire/app.json")
        .replace(r#""root": "Racine""#, r#""root": "Racinne""#)
        .replace(r#""class": "Personne""#, r#""class": "Persone""#);
    let dir = directory_with(&app, "[]");
    let state = open_dir(&dir);
    let lines = state.dialog().expect("dialog").lines;
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines.iter().all(|l| l.contains("app.json")), "{lines:?}");
}

/// Scenario "Erreurs nombreuses".
#[test]
fn many_errors() {
    let errors = (1..=25).map(|i| format!("error {i}")).collect();
    let state = AppState::with_open_errors(errors);
    let dialog = state.dialog().expect("dialog");
    assert_eq!(dialog.lines.len(), 20);
    assert_eq!(dialog.lines[19], "error 20");
    assert_eq!(dialog.more.as_deref(), Some("… et 5 autres"));
    assert_eq!(state.open_errors().len(), 25);
}

/// Scenario "Bouton Quitter", in the presentation; the dialog of the opening does not close.
#[test]
fn quit() {
    let mut state = AppState::with_open_errors(vec!["e".into()]);
    state.dispatch(Intent::CloseDialog);
    assert!(state.dialog().is_some());
    assert!(!state.quit_requested());
    state.dispatch(Intent::Quit);
    assert!(state.quit_requested());
    assert_eq!(state.exit_code(), 1);
}

/// Scenario "Fichier de données fautif", then "Bouton Continuer" in the presentation.
#[test]
fn source_that_fails() {
    let dir = directory_with(&example("annuaire/app.json"), "[ { \"nom\": ");
    let mut state = open_dir(&dir);
    assert_eq!(table(&state).total_count(), 0);
    let dialog = state.dialog().expect("dialog");
    assert_eq!(dialog.title, "Certaines données n'ont pas pu être chargées");
    assert_eq!(dialog.button, "Continuer");
    assert!(
        dialog.lines.iter().any(|l| l.contains("`personnes`")),
        "{:?}",
        dialog.lines
    );
    assert_eq!(state.exit_code(), 0);
    state.dispatch(dialog.button_intent);
    assert_eq!(state.dialog(), None);
}

/// A source with diagnostics: they are warnings.
#[test]
fn diagnostics_are_warnings() {
    let dir = directory_with(
        &example("annuaire/app.json"),
        r#"[ { "nom": "A" }, { "prenom": "B" } ]"#,
    );
    let state = open_dir(&dir);
    assert_eq!(state.dialog(), None);
    assert_eq!(state.status_bar(), Some("⚠ 1 avertissement"));
}

/// Scenarios "Libellés manquants" (`app-launch`) and "Deux libellés manquants" (`labels`): two
/// warnings, each once, logged at the `WARN` level.
#[test]
fn missing_labels() {
    let _lock = lock_log_tests();
    let persons = example("annuaire/personnes.json");
    let dir = directory_with(&two_missing_labels(), &persons);
    let (mut state, log) = capture_log(|| open_dir(&dir));
    for _ in 0..3 {
        state.prepare();
    }
    assert_eq!(state.status_bar(), Some("⚠ 2 avertissements"));
    state.dispatch(Intent::ShowWarnings);
    let dialog = state.dialog().expect("dialog");
    assert_eq!(dialog.title, "Avertissements");
    assert_eq!(dialog.button, "Fermer");
    assert_eq!(
        dialog.lines,
        [
            "missing label `Personne.prenom` (fr)",
            "missing label `SituationFamiliale.VEUF` (fr)"
        ]
    );
    // The header and the literal show their key.
    let table = table(&state);
    assert_eq!(table.columns()[1].header(), "#Personne.prenom#");
    let warnings: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("missing label"))
        .collect();
    assert_eq!(warnings.len(), 2, "{log}");
    assert!(
        warnings
            .iter()
            .all(|l| l.contains(" WARN ") && l.contains("lang=fr"))
    );
    assert!(warnings[0].contains("Personne.prenom"), "{log}");
    assert!(warnings[1].contains("SituationFamiliale.VEUF"), "{log}");
    state.dispatch(Intent::CloseDialog);
    assert_eq!(state.dialog(), None);
}

/// Scenario "Aucun avertissement".
#[test]
fn no_warning() {
    let mut state = open_example();
    assert_eq!(state.status_bar(), None);
    state.dispatch(Intent::ShowWarnings);
    assert_eq!(state.dialog(), None);
}

/// The intents of the table reach it.
#[test]
fn table_intents() {
    let mut state = open_example();
    state.dispatch(Intent::table(
        "personnes",
        TableIntent::SetFilter("dup".into()),
    ));
    state.prepare();
    assert_eq!(table(&state).counter(), "1 / 10");
    assert_eq!(table(&state).filter(), "dup");
}
