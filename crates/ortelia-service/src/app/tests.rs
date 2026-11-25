//! Tests of the opening of an application (specification `app-context`), with applications
//! written in temporary folders.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ortelia_model::builder::{DiagnosticKind, LoadErrorKind};
use ortelia_model::document::{Change, Target};
use ortelia_model::metamodel::ModelLoadErrorKind;
use ortelia_model::store::{ObjectId, Store};
use ortelia_model::value::Value;
use tempfile::TempDir;

use super::{AppContext, OpenError, SourceError, SourceStatus};

/// `tracing` caches, per call site, whether a subscriber is interested: a test without
/// subscriber that reaches "source failed" first would hide the event from the test that
/// captures the log. The tests of the opening run one at a time.
static OPEN_TESTS: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    // A test that panicked while holding the lock "poisons" it; the lock is still usable.
    OPEN_TESTS
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

/// A folder with the given files, `(name, content)`.
fn folder(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temporary folder");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write");
    }
    dir
}

/// An `app.json` with a model file and the sources `(id, file, target)`.
fn app_json(model: &str, root: &str, sources: &[(&str, &str, &str)]) -> String {
    let sources: Vec<String> = sources
        .iter()
        .map(|(id, file, target)| {
            format!(
                r#"{{ "id": "{id}", "format": "json", "file": "{file}", "target": "{target}" }}"#
            )
        })
        .collect();
    format!(
        r#"{{ "formatVersion": "0.3", "id": "test", "name": "Test",
             "model": {{ "files": ["{model}"], "root": "{root}" }},
             "sources": [ {} ] }}"#,
        sources.join(", ")
    )
}

/// A network application in a temporary folder, with its data files `(name, content)` and its
/// sources `(id, file, target)`.
fn network_app(data: &[(&str, &str)], sources: &[(&str, &str, &str)]) -> TempDir {
    let app = app_json("reseau.model.json", "Reseau", sources);
    let model = example("reseau-meteo/reseau.model.json");
    let mut files = vec![
        ("app.json", app.as_str()),
        ("reseau.model.json", model.as_str()),
    ];
    files.extend_from_slice(data);
    folder(&files)
}

/// The directory application in a temporary folder, with another `personnes.json`, or none.
fn directory_app(persons: Option<&str>) -> TempDir {
    let app = example("annuaire/app.json");
    let model = example("annuaire/annuaire.model.json");
    let mut files = vec![
        ("app.json", app.as_str()),
        ("annuaire.model.json", model.as_str()),
    ];
    if let Some(persons) = persons {
        files.push(("personnes.json", persons));
    }
    folder(&files)
}

fn open(dir: &TempDir) -> AppContext {
    AppContext::open(&dir.path().join("app.json")).unwrap_or_else(|e| panic!("{e}"))
}

fn open_error(dir: &TempDir) -> OpenError {
    AppContext::open(&dir.path().join("app.json")).expect_err("refused")
}

/// The only child of a single reference, named in the class of `obj`.
fn child(store: &Store, obj: ObjectId, reference: &str) -> ObjectId {
    let children = children(store, obj, reference);
    assert_eq!(children.len(), 1, "{reference}");
    children[0]
}

fn children<'s>(store: &'s Store, obj: ObjectId, reference: &str) -> &'s [ObjectId] {
    let model = store.model();
    let id = model
        .class(store.class_of(obj))
        .reference(reference)
        .unwrap_or_else(|| panic!("{reference}"));
    store.children(obj, id)
}

fn text(store: &Store, obj: ObjectId, attr: &str) -> String {
    let model = store.model();
    let id = model
        .class(store.class_of(obj))
        .attribute(attr)
        .unwrap_or_else(|| panic!("{attr}"));
    match store.get(obj, id) {
        Value::Str(text) => text.to_owned(),
        other => panic!("{attr}: {other:?}"),
    }
}

fn class_name(store: &Store, obj: ObjectId) -> &str {
    store.model().class(store.class_of(obj)).name()
}

/// The persons of the directory of an open application.
fn persons(context: &AppContext) -> Vec<String> {
    let store = context.document().store();
    let directory = child(store, context.document().root(), "annuaire");
    children(store, directory, "personnes")
        .iter()
        .map(|&p| text(store, p, "nom"))
        .collect()
}

// --- Opening ---

/// Scenario "Annuaire ouvert".
#[test]
fn directory_opened() {
    let _lock = lock();
    let context = AppContext::open(&examples().join("annuaire/app.json")).expect("open");
    let store = context.document().store();
    assert_eq!(class_name(store, context.document().root()), "Racine");
    let names = persons(&context);
    assert_eq!(names.len(), 10);
    assert_eq!(names.first().map(String::as_str), Some("Dupont"));
    assert_eq!(names.last().map(String::as_str), Some("Garcia"));
    let source = &context.sources()[0];
    assert_eq!(source.id, "personnes");
    let SourceStatus::Loaded {
        diagnostic_count, ..
    } = source.status
    else {
        panic!("{:?}", source.status)
    };
    assert_eq!(diagnostic_count, 0);
    assert_eq!(context.app().id, "annuaire");
}

/// Scenario "Fichier de modèle introuvable".
#[test]
fn model_file_not_found() {
    let _lock = lock();
    let dir = folder(&[("app.json", &app_json("absent.model.json", "Racine", &[]))]);
    let error = open_error(&dir);
    assert!(matches!(error, OpenError::Io { .. }), "{error}");
    assert!(error.to_string().contains("absent.model.json"), "{error}");
}

/// Scenario "Modèle qui ne compile pas".
#[test]
fn model_that_does_not_compile() {
    let model = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p", "classes": [
        { "name": "Racine", "attributes": [ { "name": "age", "type": "Entier" } ] } ] } ] }"#;
    let _lock = lock();
    let dir = folder(&[
        ("app.json", &app_json("faux.model.json", "Racine", &[])),
        ("faux.model.json", model),
    ]);
    let error = open_error(&dir);
    let OpenError::Model(load) = &error else {
        panic!("{error}")
    };
    assert!(
        matches!(load.kind, ModelLoadErrorKind::Compile(_)),
        "{error}"
    );
    let message = error.to_string();
    for part in ["faux.model.json", "Racine", "age"] {
        assert!(message.contains(part), "{part}: {message}");
    }
}

/// An error in `app.json` names it: when reading it, and when checking it.
#[test]
fn error_in_app_json() {
    let _lock = lock();
    let model = example("annuaire/annuaire.model.json");
    for app in [
        "{ \"formatVersion\": \"0.3\" ".to_owned(),
        app_json("annuaire.model.json", "Racinne", &[]),
    ] {
        let dir = folder(&[("app.json", &app), ("annuaire.model.json", &model)]);
        let error = open_error(&dir);
        assert!(matches!(error, OpenError::App { .. }), "{error}");
        assert!(error.to_string().contains("app.json"), "{error}");
    }
}

// --- Root and skeleton ---

/// Scenario "Squelette de l'annuaire".
#[test]
fn skeleton_of_the_directory() {
    let _lock = lock();
    let dir = directory_app(Some(&example("annuaire/personnes.json")));
    let context = open(&dir);
    let document = context.document();
    let journal = document.journal();
    assert_eq!(journal.len(), 2);
    let store = document.store();
    let root = document.root();
    let directory = child(store, root, "annuaire");
    let racine = store.model().class(store.class_of(root));
    assert_eq!(
        journal[0].changes,
        [Change::Created {
            obj: directory,
            class: store.class_of(directory),
            parent: root,
            reference: racine.reference("annuaire").expect("annuaire"),
            index: 0
        }]
    );
    assert_eq!(class_name(store, directory), "Annuaire");
    assert!(matches!(journal[1].changes[..], [Change::Replaced { .. }]));
}

/// A small model: a single reference `a`, whose class has two lists `b` and `c`.
const SHARED_MODEL: &str = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p", "classes": [
    { "name": "Racine", "references": [ { "name": "a", "type": "A", "upperBound": 1 } ] },
    { "name": "A", "references": [
        { "name": "b", "type": "X", "upperBound": -1 },
        { "name": "c", "type": "X", "upperBound": -1 } ] },
    { "name": "X", "attributes": [ { "name": "nom", "type": "String" } ] } ] } ] }"#;

/// Scenario "Objet intermédiaire commun à deux cibles".
#[test]
fn intermediate_object_shared_by_two_targets() {
    let _lock = lock();
    let app = app_json(
        "m.model.json",
        "Racine",
        &[("b", "b.json", "/a/b"), ("c", "c.json", "/a/c")],
    );
    let dir = folder(&[
        ("app.json", &app),
        ("m.model.json", SHARED_MODEL),
        ("b.json", r#"[ { "nom": "b1" }, { "nom": "b2" } ]"#),
        ("c.json", r#"[ { "nom": "c1" } ]"#),
    ]);
    let context = open(&dir);
    let store = context.document().store();
    let a = child(store, context.document().root(), "a");
    assert_eq!(children(store, a, "b").len(), 2);
    assert_eq!(text(store, children(store, a, "c")[0], "nom"), "c1");
    // One object created by the skeleton.
    assert_eq!(context.document().journal()[0].changes.len(), 1);
}

// --- Sources ---

const STATIONS: &str =
    r#"[ { "code": "S1", "nom": "Brest" }, { "code": "S2", "nom": "Quimper" } ]"#;
const ZONES: &str = r#"[ { "code": "Z1" } ]"#;

/// Scenario "Deux sources dans deux branches".
#[test]
fn two_sources_in_two_branches() {
    let _lock = lock();
    let dir = network_app(
        &[("stations.json", STATIONS), ("zones.json", ZONES)],
        &[
            ("stations", "stations.json", "/stations"),
            ("zones", "zones.json", "/zones"),
        ],
    );
    let context = open(&dir);
    let document = context.document();
    let store = document.store();
    let root = document.root();
    let names: Vec<String> = children(store, root, "stations")
        .iter()
        .map(|&s| text(store, s, "nom"))
        .collect();
    assert_eq!(names, ["Brest", "Quimper"]);
    assert_eq!(children(store, root, "zones").len(), 1);
    // No intermediate object: one transaction per source, in the order of `app.json`.
    let targets: Vec<Target> = document
        .journal()
        .iter()
        .map(|t| match t.changes[..] {
            [Change::Replaced { target, .. }] => target,
            ref other => panic!("{other:?}"),
        })
        .collect();
    let sources: Vec<Target> = context.sources().iter().map(|s| s.target).collect();
    assert_eq!(targets, sources);
    assert_eq!(targets.len(), 2);
}

/// Scenario "Source à la racine".
#[test]
fn source_at_the_root() {
    let _lock = lock();
    let network = r#"{ "nom": "Bretagne", "stations": [ { "code": "S1", "nom": "Brest" } ] }"#;
    let dir = network_app(
        &[("reseau.json", network)],
        &[("reseau", "reseau.json", "/")],
    );
    let context = open(&dir);
    let document = context.document();
    let root = document.root();
    assert_eq!(text(document.store(), root, "nom"), "Bretagne");
    assert_eq!(context.sources()[0].target, Target::Root);
    assert_eq!(
        document.journal().last().expect("transaction").changes,
        [Change::Replaced {
            target: Target::Root,
            objects: vec![root]
        }]
    );
}

/// Scenario "Fichier de données absent".
#[test]
fn data_file_missing() {
    let _lock = lock();
    let dir = directory_app(None);
    let context = open(&dir);
    assert!(persons(&context).is_empty());
    let source = &context.sources()[0];
    let SourceStatus::Failed(error @ SourceError::Io { .. }) = &source.status else {
        panic!("{:?}", source.status)
    };
    assert!(error.to_string().contains("personnes.json"), "{error}");
}

/// Scenario "Données refusées dans une source sur deux".
#[test]
fn data_refused_in_one_source_of_two() {
    let _lock = lock();
    let stations = r#"[ { "code": "S1", "nom": "Brest" }, { "code": "S2", "nmo": "Quimper" } ]"#;
    let dir = network_app(
        &[("stations.json", stations), ("zones.json", ZONES)],
        &[
            ("stations", "stations.json", "/stations"),
            ("zones", "zones.json", "/zones"),
        ],
    );
    let context = open(&dir);
    let store = context.document().store();
    let root = context.document().root();
    assert!(children(store, root, "stations").is_empty());
    assert_eq!(children(store, root, "zones").len(), 1);
    let SourceStatus::Failed(SourceError::Data(failure)) = &context.sources()[0].status else {
        panic!("{:?}", context.sources()[0].status)
    };
    assert_eq!(failure.error_count, 1);
    assert_eq!(
        failure.errors[0].kind,
        LoadErrorKind::UnknownKey { key: "nmo".into() }
    );
    assert_eq!(failure.errors[0].path.to_string(), "/stations/1");
    assert!(matches!(
        context.sources()[1].status,
        SourceStatus::Loaded { .. }
    ));
}

// --- State of each source ---

/// Scenario "Personne sans nom".
#[test]
fn person_without_a_name() {
    let _lock = lock();
    let dir = directory_app(Some(
        r#"[ { "nom": "A" }, { "nom": "B" }, { "prenom": "C" }, { "nom": "D" } ]"#,
    ));
    let context = open(&dir);
    let SourceStatus::Loaded {
        diagnostics,
        diagnostic_count,
        ..
    } = &context.sources()[0].status
    else {
        panic!("{:?}", context.sources()[0].status)
    };
    assert_eq!(*diagnostic_count, 1);
    assert_eq!(diagnostics[0].path.to_string(), "/annuaire/personnes/2");
    assert_eq!(
        diagnostics[0].kind,
        DiagnosticKind::MissingRequired {
            attribute: "nom".into()
        }
    );
    let store = context.document().store();
    let directory = child(store, context.document().root(), "annuaire");
    assert_eq!(children(store, directory, "personnes").len(), 4);
}

/// Scenario "Lien entre source et branche".
#[test]
fn link_between_source_and_branch() {
    let _lock = lock();
    let context = AppContext::open(&examples().join("annuaire/app.json")).expect("open");
    let document = context.document();
    let store = document.store();
    let directory = child(store, document.root(), "annuaire");
    let source = &context.sources()[0];
    assert_eq!(
        source.target,
        Target::Reference {
            parent: directory,
            reference: store
                .model()
                .class(store.class_of(directory))
                .reference("personnes")
                .expect("personnes")
        }
    );
    assert!(source.file.ends_with("personnes.json"));
    let SourceStatus::Loaded { integrated_at, .. } = source.status else {
        panic!("{:?}", source.status)
    };
    let last = document.journal().last().expect("transaction");
    assert_eq!(integrated_at, last.revision);
    assert!(
        matches!(last.changes[..], [Change::Replaced { target, .. }] if target == source.target)
    );
}

// --- Log ---

/// In-memory log output shared between the subscriber and the test.
#[derive(Clone, Default)]
struct SharedBuffer(std::sync::Arc<Mutex<Vec<u8>>>);

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

/// Scenario "Valeur fautive non journalisée": the log gives the application, the files, the
/// counts and the path of the error, never a value of the data.
#[test]
fn offending_value_not_logged() {
    let _lock = lock();
    let stations = r#"[ { "code": "S1", "nom": "Brest", "altitude": "haute" } ]"#;
    let dir = network_app(
        &[("stations.json", stations), ("zones.json", ZONES)],
        &[
            ("stations", "stations.json", "/stations"),
            ("zones", "zones.json", "/zones"),
        ],
    );
    let buffer = SharedBuffer::default();
    let writer = buffer.clone();
    // A closure returning a writer is a `MakeWriter` for tracing-subscriber.
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    // Installed for this thread only: no global subscriber shared with other tests.
    let context = tracing::subscriber::with_default(subscriber, || open(&dir));
    assert!(matches!(
        context.sources()[0].status,
        SourceStatus::Failed(SourceError::Data(_))
    ));
    let log = String::from_utf8(buffer.0.lock().expect("log").clone()).expect("UTF-8");
    for part in [
        "source failed",
        "stations.json",
        "/stations/0/altitude",
        "errors=1",
        "source loaded",
        "objects=1",
        "application opened",
        "app=test",
    ] {
        assert!(log.contains(part), "{part}:\n{log}");
    }
    // Long enough not to appear by chance in the name of the temporary folder.
    for value in ["haute", "Brest"] {
        assert!(!log.contains(value), "{value}:\n{log}");
    }
}
