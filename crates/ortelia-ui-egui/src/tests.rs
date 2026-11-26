//! Tests of the interface without a screen (egui_kittest, through the AccessKit tree): what is
//! shown comes from the presentation, and the interactions give the expected intents.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use egui::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use ortelia_presentation::{AppState, Intent, TableIntent};

use super::*;

/// Serialises the tests that run `OrteliaApp`.
///
/// `tracing` caches, for each `info!` call site, whether a subscriber is interested in it.
/// While a single subscriber exists, that interest is taken from the subscriber of the
/// thread that reaches the call site first: if a test without subscriber reaches "main
/// window shown" first, the cache says "nobody is interested" and the event is lost for
/// the test that captures it. Running these tests one at a time avoids that.
static APP_TESTS: Mutex<()> = Mutex::new(());

/// Also taken by the snapshots of the whole application (`snapshot_tests`).
pub(crate) fn lock_app_tests() -> MutexGuard<'static, ()> {
    // A test that panicked while holding the lock "poisons" it; the lock is still usable.
    APP_TESTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn example(path: &str) -> String {
    let path = examples().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The state of `examples/annuaire`.
pub(crate) fn directory() -> AppState {
    AppState::open(&examples().join("annuaire/app.json"))
}

/// The state of the directory application with another `app.json` and `personnes.json`,
/// written in a temporary folder.
fn directory_with(app: &str, persons: &str) -> AppState {
    let dir = tempfile::tempdir().expect("temporary folder");
    for (name, content) in [
        ("app.json", app.to_owned()),
        (
            "annuaire.model.json",
            example("annuaire/annuaire.model.json"),
        ),
        ("personnes.json", persons.to_owned()),
    ] {
        std::fs::write(dir.path().join(name), content).expect("write");
    }
    // Everything is read at the opening: the folder can go.
    AppState::open(&dir.path().join("app.json"))
}

/// A state and the intents its drawing gave, not dispatched: the tests check what the
/// interface asks, not what the presentation does of it.
struct Recorder {
    state: AppState,
    intents: Vec<Intent>,
}

/// A harness that draws the main window of `state` and records its intents.
fn recorder(state: AppState) -> Harness<'static, Recorder> {
    Harness::new_ui_state(
        |ui, recorder: &mut Recorder| {
            recorder.state.prepare();
            let intents = draw_main_window(ui, &recorder.state);
            recorder.intents.extend(intents);
        },
        Recorder {
            state,
            intents: Vec::new(),
        },
    )
}

/// The intents of the table recorded, without the resizes (a first layout may adjust widths).
fn table_intents(harness: &Harness<'_, Recorder>) -> Vec<TableIntent> {
    harness
        .state()
        .intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Table(TableIntent::ResizeColumn { .. }) => None,
            Intent::Table(intent) => Some(intent.clone()),
            _ => None,
        })
        .collect()
}

/// Scenario "Titre de la fenêtre native".
#[test]
fn native_window_title_and_size() {
    let options = native_options(&directory());
    assert_eq!(options.viewport.title.as_deref(), Some("Annuaire"));
    assert_eq!(options.viewport.inner_size, Some(egui::vec2(1000.0, 700.0)));
}

/// Scenarios "Texte central affiché" (`app-launch`) and "Table de l'annuaire affichée"
/// (`table-view`): headers, sort indicators, a cell, the counter, a check box.
#[test]
fn table_of_the_directory_is_displayed() {
    let _lock = lock_app_tests();
    let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(directory()));
    harness.run();
    let nom = harness.get_by_role_and_label(Role::ColumnHeader, "Nom");
    // Sorted by `nom` then `prenom` (the indicators come from the presentation).
    assert_eq!(nom.value().as_deref(), Some("⏶ 1"));
    let prenom = harness.get_by_role_and_label(Role::ColumnHeader, "Prénom");
    assert_eq!(prenom.value().as_deref(), Some("⏶ 2"));
    let taille = harness.get_by_role_and_label(Role::ColumnHeader, "Taille (m)");
    assert_eq!(taille.value(), None);
    harness.get_by_label("Dupont");
    harness.get_by_label("10 / 10");
    // The check boxes, disabled.
    let boxes = harness.get_all_by_role(Role::CheckBox).collect::<Vec<_>>();
    assert!(!boxes.is_empty());
    // No status bar without warning (scenario "Aucun avertissement").
    assert!(harness.query_by_label_contains("avertissement").is_none());
}

/// Scenario "Clic et Maj+clic".
#[test]
fn click_and_shift_click() {
    let mut harness = recorder(directory());
    harness.run();
    harness
        .get_by_role_and_label(Role::ColumnHeader, "Nom")
        .click();
    harness.run();
    // Shift held during the click, as a user does.
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    harness
        .get_by_role_and_label(Role::ColumnHeader, "Prénom")
        .click();
    harness.run();
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    harness.run();
    assert_eq!(
        table_intents(&harness),
        [
            TableIntent::SortBy {
                column: 0,
                add: false
            },
            TableIntent::SortBy {
                column: 1,
                add: true
            }
        ]
    );
}

/// Scenario "Saisie du filtre".
#[test]
fn filter_typed() {
    let mut harness = recorder(directory());
    harness.run();
    harness.get_by_role(Role::TextInput).focus();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("dup");
    harness.run();
    assert_eq!(
        table_intents(&harness),
        [TableIntent::SetFilter("dup".into())]
    );
}

/// The clear button of the filter, shown when there is a filter.
#[test]
fn filter_cleared() {
    let mut state = directory();
    state.dispatch(Intent::Table(TableIntent::SetFilter("dup".into())));
    let mut harness = recorder(state);
    harness.run();
    harness.get_by_label("1 / 10");
    harness.get_by_label("✖").click();
    harness.run();
    assert_eq!(
        table_intents(&harness),
        [TableIntent::SetFilter(String::new())]
    );
}

/// Scenario "Glisser-déposer d'un en-tête": `Né le` (third) dropped on `Nom` (first).
#[test]
fn header_dragged_and_dropped() {
    let mut harness = recorder(directory());
    harness.run();
    let from = harness
        .get_by_role_and_label(Role::ColumnHeader, "Né le")
        .rect()
        .center();
    let to = harness
        .get_by_role_and_label(Role::ColumnHeader, "Nom")
        .rect()
        .center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    // Moved in steps, as a real pointer: the drag starts beyond a small distance.
    for step in 1..=4 {
        harness.hover_at(from + (to - from) * (step as f32 / 4.0));
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    assert_eq!(
        table_intents(&harness),
        [TableIntent::MoveColumn { from: 2, to: 0 }]
    );
}

/// A resize of a column by the user gives its new width.
#[test]
fn column_resized() {
    let mut harness = recorder(directory());
    harness.run();
    let nom = harness
        .get_by_role_and_label(Role::ColumnHeader, "Nom")
        .rect();
    // The resize handle is the right edge of the column, half the spacing further.
    let edge = egui::pos2(nom.right() + 4.0, nom.center().y);
    harness.hover_at(edge);
    harness.run();
    harness.drag_at(edge);
    harness.run();
    for step in 1..=4 {
        harness.hover_at(edge + egui::vec2(15.0 * step as f32, 0.0));
        harness.run();
    }
    harness.drop_at(edge + egui::vec2(60.0, 0.0));
    harness.run();
    let resized: Vec<(usize, f32)> = harness
        .state()
        .intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Table(TableIntent::ResizeColumn { column, width }) => Some((*column, *width)),
            _ => None,
        })
        .collect();
    let (column, width) = *resized.last().expect("a resize");
    assert_eq!(column, 0);
    assert!(width > 180.0, "{width}");
    // No sort: the handle is not the header.
    assert!(
        table_intents(&harness).is_empty(),
        "{:?}",
        harness.state().intents
    );
}

/// Scenario "Bouton Quitter": the intent, then the closing of the window.
#[test]
fn quit_button() {
    let _lock = lock_app_tests();
    let errors = vec!["app.json: first error".into(), "app.json: second".into()];
    let mut harness =
        Harness::new_eframe(|_cc| OrteliaApp::new(AppState::with_open_errors(errors)));
    harness.run();
    harness.get_by_label("Impossible d'ouvrir l'application");
    harness.get_by_label("app.json: second");
    harness.get_by_label("Quitter").click();
    harness.run();
    let app = harness.state();
    assert!(app.state.quit_requested());
    assert!(app.close_sent);
}

/// The button of the dialog of an opening that failed emits `Quit`.
#[test]
fn quit_button_intent() {
    let mut harness = recorder(AppState::with_open_errors(vec!["e".into()]));
    harness.run();
    harness.get_by_label("Quitter").click();
    harness.run();
    assert!(harness.state().intents.contains(&Intent::Quit));
}

/// Scenario "Bouton Continuer".
#[test]
fn continue_button() {
    let _lock = lock_app_tests();
    let state = directory_with(&example("annuaire/app.json"), "[ { \"nom\": ");
    let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(state));
    harness.run();
    let title = "Certaines données n'ont pas pu être chargées";
    harness.get_by_label(title);
    harness.get_by_label("0 / 0");
    harness.get_by_label("Continuer").click();
    harness.run();
    assert!(harness.query_by_label(title).is_none());
}

/// Scenario "Libellés manquants": the status bar, then the list of the warnings.
#[test]
fn missing_labels() {
    let _lock = lock_app_tests();
    let app = example("annuaire/app.json")
        .replace(
            r#""key": "Personne.prenom", "text": "Prénom""#,
            r#""key": "Annuaire", "text": "Annuaire""#,
        )
        .replace(
            r#""key": "SituationFamiliale.VEUF", "text": "Veuf, veuve""#,
            r#""key": "Racine", "text": "Racine""#,
        );
    let state = directory_with(&app, &example("annuaire/personnes.json"));
    let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(state));
    harness.run();
    harness.get_by_label("⚠ 2 avertissements").click();
    harness.run();
    harness.get_by_label("Avertissements");
    harness.get_by_label("missing label `Personne.prenom` (fr)");
    harness.get_by_label("missing label `SituationFamiliale.VEUF` (fr)");
    harness.get_by_label("Fermer").click();
    harness.run();
    assert!(harness.query_by_label("Avertissements").is_none());
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
        let mut harness = Harness::new_eframe(|_cc| OrteliaApp::new(directory()));
        harness.run();
        harness.run_steps(5);
    });
    let log = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert_eq!(log.matches("main window shown").count(), 1, "{log}");
}

/// Once drawn, a table that nobody touches emits nothing: no intent, so no repaint asked
/// at each frame.
#[test]
fn idle_table_emits_no_intent() {
    let mut harness = recorder(directory());
    harness.run();
    // The widths measured by the first frames go to the presentation, as the application does.
    for intent in std::mem::take(&mut harness.state_mut().intents) {
        harness.state_mut().state.dispatch(intent);
    }
    harness.run_steps(5);
    assert!(
        harness.state().intents.is_empty(),
        "{:?}",
        harness.state().intents
    );
}
