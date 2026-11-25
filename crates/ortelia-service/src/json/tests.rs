//! Tests of the JSON reader (specification `data-loading`).

use std::sync::Arc;

use ortelia_model::builder::{
    DataPath, DiagnosticKind, LoadError, LoadErrorKind, LoadFailure, Loaded, MAX_REPORTED,
    PathSegment,
};
use ortelia_model::fragment::{Fragment, Shape};
use ortelia_model::metamodel::{Model, ModelFile};
use ortelia_model::store::ObjectId;
use ortelia_model::value::Value;

use super::{read_json, read_json_slice};

/// A file of `examples/`.
fn example(path: &str) -> String {
    let path = format!("{}/../../examples/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn example_model(path: &str) -> Arc<Model> {
    let file = ModelFile::parse(&example(path)).expect("parse");
    Arc::new(Model::compile(&[file]).expect("compile"))
}

fn network_model() -> Arc<Model> {
    example_model("reseau-meteo/reseau.model.json")
}

fn load(json: &str) -> Result<Fragment, LoadFailure> {
    let model = network_model();
    let root = model.class_id("Reseau").expect("Reseau");
    read_json(
        json.as_bytes(),
        model,
        Shape::root(root),
        DataPath::default(),
    )
    .map(|loaded| loaded.fragment)
}

fn load_slice(json: &str) -> Result<Fragment, LoadFailure> {
    let model = network_model();
    let root = model.class_id("Reseau").expect("Reseau");
    read_json_slice(
        json.as_bytes(),
        model,
        Shape::root(root),
        DataPath::default(),
    )
    .map(|loaded| loaded.fragment)
}

/// The error of a load refused with exactly one error.
fn only_error<T>(result: Result<T, LoadFailure>) -> LoadError {
    let Err(failure) = result else {
        panic!("the load succeeded")
    };
    assert_eq!(failure.error_count, 1, "{:#?}", failure.errors);
    failure.errors.into_iter().next().expect("error")
}

/// One line per object, depth first, in file order: depth, class, and the values that are set.
fn dump(fragment: &Fragment) -> Vec<String> {
    let store = fragment.store();
    let model = store.model();
    let mut lines = Vec::new();
    let mut stack: Vec<(ObjectId, usize)> =
        fragment.heads().iter().rev().map(|&h| (h, 0)).collect();
    while let Some((obj, depth)) = stack.pop() {
        let class = model.class(store.class_of(obj));
        let mut line = format!("{depth} {}", class.name());
        for attribute in class.attributes() {
            let value = store.get(obj, attribute.id());
            if value != Value::Unset {
                line.push_str(&format!(" {}={value:?}", attribute.name()));
            }
        }
        lines.push(line);
        // Pushed in reverse, so that they are popped in order.
        for reference in class.references().iter().rev() {
            for &child in store.children(obj, reference.id()).iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }
    lines
}

fn count(lines: &[String], class: &str) -> usize {
    lines
        .iter()
        .filter(|l| l.split(' ').nth(1) == Some(class))
        .count()
}

/// Wraps a list of equipments in a minimal network document.
fn with_equipments(equipments: &str) -> String {
    format!(
        r#"{{ "stations": [ {{ "code": "S", "nom": "S", "equipements": [ {equipments} ] }} ] }}"#
    )
}

/// Scenario "Fichier d'exemple du réseau", and "Modèle et exemple chargés" of
/// `storage-benchmark`: objects at depth 4 under the root.
#[test]
fn example_file() {
    let store = load(&example("reseau-meteo/reseau-exemple.json")).expect("load");
    let lines = dump(&store);
    let expected = [
        ("Reseau", 1),
        ("Station", 2),
        ("Thermometre", 2),
        ("Anemometre", 1),
        ("Pluviometre", 1),
        ("Camera", 1),
        ("Releve", 6),
        ("Lacune", 1),
        ("Annotation", 1),
        ("Maintenance", 1),
        ("Panne", 1),
        ("Etalonnage", 1),
        ("ZoneAlerte", 2),
        ("Polygone", 1),
        ("Cercle", 1),
        ("Maille", 3),
        ("Point", 17),
    ];
    for (class, n) in expected {
        assert_eq!(count(&lines, class), n, "{class}\n{lines:#?}");
    }
    assert_eq!(lines.len(), expected.iter().map(|e| e.1).sum::<usize>());
    assert!(
        lines.iter().any(|l| l.starts_with("4 Annotation")),
        "{lines:#?}"
    );
    assert!(
        lines[1].contains(r#"nom=Str("Brest-Guipavas")"#),
        "{}",
        lines[1]
    );
    assert!(lines[1].contains("altitude=Int(94)") && lines[1].contains("categorie=Enum(0)"));
    // `"valeur": 12` (an integer) is read as a float; `"valeur": null` leaves it unset.
    assert!(lines.iter().any(|l| l.contains("valeur=Float(12.0)")));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("3 Releve") && !l.contains("valeur"))
    );
    // The annotation's instant had an offset of +01:00.
    let annotation = lines
        .iter()
        .find(|l| l.contains("Annotation"))
        .expect("annotation");
    assert!(annotation.contains("2026-03-12T11:05:00Z"), "{annotation}");
}

/// Scenario "`_type_` après d'autres clés".
#[test]
fn type_after_other_keys() {
    let late =
        with_equipments(r#"{ "numero_serie": "T-17", "_type_": "Thermometre", "plage_min": -40 }"#);
    let first =
        with_equipments(r#"{ "_type_": "Thermometre", "numero_serie": "T-17", "plage_min": -40 }"#);
    let late = dump(&load(&late).expect("late"));
    assert_eq!(late, dump(&load(&first).expect("first")));
    assert!(late[2].starts_with("2 Thermometre") && late[2].contains("plage_min=Float(-40.0)"));
}

/// A late `_type_` in a nested object: the replayed object keeps its children in order.
#[test]
fn late_type_with_children() {
    let json = with_equipments(
        r#"{ "numero_serie": "T-1", "mesures": [
              { "valeur": 1.5, "_type_": "Releve", "instant": "2026-01-01T00:00:00Z" },
              { "_type_": "Lacune", "instant": "2026-01-02T00:00:00Z" } ],
           "_type_": "Thermometre" }"#,
    );
    let lines = dump(&load(&json).expect("load"));
    let classes: Vec<&str> = lines
        .iter()
        .map(|l| l.split(' ').nth(1).expect("class"))
        .collect();
    assert_eq!(
        classes,
        ["Reseau", "Station", "Thermometre", "Releve", "Lacune"]
    );
}

/// A key repeated in an object gives the same result whether `_type_` comes first or late: the
/// two lists are appended, in file order.
#[test]
fn repeated_key_with_a_late_type() {
    let measures = r#""mesures": [ { "_type_": "Releve", "valeur": 1 }, { "_type_": "Releve", "valeur": 2 } ]"#;
    let more = r#""mesures": [ { "_type_": "Releve", "valeur": 3 } ]"#;
    let first = with_equipments(&format!(
        r#"{{ "_type_": "Thermometre", "numero_serie": "T", {measures}, {more} }}"#
    ));
    let late = with_equipments(&format!(
        r#"{{ "numero_serie": "T", {measures}, "_type_": "Thermometre", {more} }}"#
    ));
    let lines = dump(&load(&first).expect("first"));
    assert_eq!(lines, dump(&load(&late).expect("late")));
    let values: Vec<&str> = lines[3..]
        .iter()
        .map(|l| l.rsplit(' ').next().expect("value"))
        .collect();
    assert_eq!(
        values,
        [
            "valeur=Float(1.0)",
            "valeur=Float(2.0)",
            "valeur=Float(3.0)"
        ]
    );
}

/// A second `_type_` that names another class is refused, with `_type_` first or late; the
/// error compares it with the class given by the first one.
#[test]
fn conflicting_types() {
    for json in [
        r#"{ "_type_": "Thermometre", "numero_serie": "T", "_type_": "Camera" }"#,
        r#"{ "numero_serie": "T", "_type_": "Thermometre", "_type_": "Camera" }"#,
    ] {
        let error = only_error(load(&with_equipments(json)));
        assert_eq!(
            error.kind,
            LoadErrorKind::NonConformingClass {
                class: "Camera".into(),
                expected: "Thermometre".into()
            },
            "{json}"
        );
    }
}

/// An integer beyond `i64::MAX` fits a `Float` attribute only.
#[test]
fn integer_beyond_i64() {
    let big = "10000000000000000000";
    let float = with_equipments(&format!(
        r#"{{ "_type_": "Thermometre", "mesures": [ {{ "_type_": "Releve", "valeur": {big} }} ] }}"#
    ));
    let lines = dump(&load(&float).expect("float"));
    assert!(lines[3].ends_with("valeur=Float(1e19)"), "{}", lines[3]);
    let int = format!(r#"{{ "stations": [ {{ "code": "S", "nom": "S", "altitude": {big} }} ] }}"#);
    let error = only_error(load(&int));
    assert_eq!(error.path.to_string(), "/stations/0/altitude");
}

/// Scenario "`_type_` absent avec un type déclaré concret".
#[test]
fn missing_type_with_a_concrete_declared_type() {
    let json = r#"{ "zones": [ { "code": "Z", "mailles": [ { "numero": 1, "coins": [ { "lat": 1, "lon": 2 } ] } ] } ] }"#;
    let lines = dump(&load(json).expect("load"));
    assert_eq!(
        lines.last().expect("point"),
        "3 Point lat=Float(1.0) lon=Float(2.0)"
    );
}

/// Scenario "`_type_` absent avec un type déclaré abstrait".
#[test]
fn missing_type_with_an_abstract_declared_type() {
    let error = only_error(load(&with_equipments(r#"{ "numero_serie": "X" }"#)));
    assert_eq!(
        error.kind,
        LoadErrorKind::AbstractClass {
            class: "Equipement".into()
        }
    );
    assert_eq!(error.path.to_string(), "/stations/0/equipements/0");
}

/// Scenario "Classe non conforme", with `_type_` first or late.
#[test]
fn non_conforming_type() {
    for json in [
        r#"{ "_type_": "Releve" }"#,
        r#"{ "instant": "2026-01-01T00:00:00Z", "_type_": "Releve" }"#,
    ] {
        let error = only_error(load(&with_equipments(json)));
        assert_eq!(
            error.kind,
            LoadErrorKind::NonConformingClass {
                class: "Releve".into(),
                expected: "Equipement".into()
            }
        );
        let message = error.to_string();
        assert!(
            message.contains("Releve") && message.contains("Equipement"),
            "{message}"
        );
    }
}

/// `_type_` of a reference whose type has no subclass must name that type.
#[test]
fn type_of_a_class_without_subclasses() {
    let ok = r#"{ "zones": [ { "_type_": "ZoneAlerte", "code": "Z" } ] }"#;
    assert!(load(ok).is_ok());
    let wrong = r#"{ "zones": [ { "code": "Z", "_type_": "Station" } ] }"#;
    let error = only_error(load(wrong));
    assert!(
        matches!(error.kind, LoadErrorKind::NonConformingClass { .. }),
        "{error}"
    );
}

#[test]
fn unknown_type_name() {
    let error = only_error(load(&with_equipments(r#"{ "_type_": "Barometre" }"#)));
    assert_eq!(
        error.kind,
        LoadErrorKind::UnknownClass {
            name: "Barometre".into()
        }
    );
    assert_eq!(error.path.to_string(), "/stations/0/equipements/0");
}

/// Scenario "Clé inconnue".
#[test]
fn unknown_key() {
    let json = with_equipments(
        r#"{ "_type_": "Thermometre", "numero_serie": "T", "mesures": [
            { "_type_": "Releve", "instant": "2026-01-01T00:00:00Z" },
            { "_type_": "Releve", "instant": "2026-01-01T00:10:00Z" },
            { "_type_": "Releve", "instant": "2026-01-01T00:20:00Z", "valuer": 1.0 } ] }"#,
    );
    let error = only_error(load(&json));
    assert_eq!(
        error.kind,
        LoadErrorKind::UnknownKey {
            key: "valuer".into()
        }
    );
    assert_eq!(
        error.path.to_string(),
        "/stations/0/equipements/0/mesures/2"
    );
}

/// Scenario "Valeur de mauvais type", and other values of the wrong JSON type.
#[test]
fn wrong_value_type() {
    let error = only_error(load(
        r#"{ "stations": [ { "code": "A" }, { "code": "B", "altitude": "haute" } ] }"#,
    ));
    assert_eq!(
        error.kind,
        LoadErrorKind::WrongValueType {
            attribute: "altitude".into(),
            expected: "Int".into()
        }
    );
    assert_eq!(error.path.to_string(), "/stations/1/altitude");
    for value in ["98.5", "true", "{}", "[1]", "18446744073709551615"] {
        let json = format!(r#"{{ "stations": [ {{ "altitude": {value} }} ] }}"#);
        let error = only_error(load(&json));
        assert!(
            matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
            "{value}: {error}"
        );
    }
}

/// Scenario "JSON mal formé", and other structural errors.
#[test]
fn malformed_json() {
    let error = only_error(load("{\n  \"stations\": [\n    { \"code\": \"A\""));
    let LoadErrorKind::Syntax { line, column } = error.kind else {
        panic!("{error}")
    };
    assert_eq!(line, 3);
    assert!(column > 0);
    assert!(error.to_string().contains("line 3"), "{error}");
    let error = only_error(load(r#"{ "nom": "a" } x"#));
    assert!(
        matches!(error.kind, LoadErrorKind::Syntax { .. }),
        "{error}"
    );
}

/// A value of the wrong kind for a reference or for the branch: the error says what was
/// expected, at the path of the reference, and never quotes the value.
#[test]
fn wrong_structure() {
    for (json, path, expected) in [
        (r#"{ "stations": {} }"#, "/stations", "an array of objects"),
        (
            r#"{ "zones": [ { "emprise": [] } ] }"#,
            "/zones/0/emprise",
            "an object",
        ),
        ("[]", "/", "an object"),
        (r#""secret""#, "/", "an object"),
    ] {
        let error = only_error(load(json));
        assert_eq!(
            error.kind,
            LoadErrorKind::WrongStructure {
                expected: expected.into()
            },
            "{json}"
        );
        assert_eq!(error.path.to_string(), path, "{json}");
        assert!(!error.to_string().contains("secret"), "{error}");
    }
}

fn directory_model() -> Arc<Model> {
    example_model("annuaire/annuaire.model.json")
}

/// Loads a file for the branch `/annuaire/personnes` of the directory.
fn load_persons(json: &str) -> Result<Loaded, LoadFailure> {
    let model = directory_model();
    let directory = model.class(model.class_id("Annuaire").expect("Annuaire"));
    let persons = directory.reference("personnes").expect("personnes");
    let shape = Shape::of(&model, persons);
    let base = DataPath(vec![
        PathSegment::Name("annuaire".into()),
        PathSegment::Name("personnes".into()),
    ]);
    read_json(json.as_bytes(), model, shape, base)
}

/// Scenario "Fichier des personnes".
#[test]
fn persons_file() {
    let loaded = load_persons(&example("annuaire/personnes.json")).expect("load");
    assert_eq!(loaded.diagnostic_count, 0, "{:#?}", loaded.diagnostics);
    let fragment = loaded.fragment;
    assert_eq!(fragment.heads().len(), 10);
    let lines = dump(&fragment);
    assert_eq!(lines.len(), 10);
    assert!(
        lines.iter().all(|l| l.starts_with("0 Personne ")),
        "{lines:#?}"
    );
    assert!(lines[0].contains(r#"nom=Str("Dupont")"#), "{}", lines[0]);
    assert!(lines[9].contains(r#"nom=Str("Garcia")"#), "{}", lines[9]);
    // Every type of value of the directory model, and the enum as the index of its literal.
    for part in [
        "date_naissance=Date(1985-03-12)",
        "nombre_enfants=Int(2)",
        "taille=Float(1.68)",
        "contact_autorise=Bool(true)",
        "situation=Enum(1)",
        "inscription=DateTime(2024-01-15T09:30:00Z)",
        "notes=Str(",
    ] {
        assert!(lines[0].contains(part), "{part}: {}", lines[0]);
    }
    // Optional attributes left out of the file stay unset.
    assert!(!lines[7].contains("nombre_enfants"), "{}", lines[7]);
}

/// Scenario "Objet au lieu d'un tableau".
#[test]
fn object_instead_of_an_array() {
    let error = only_error(load_persons(r#"{ "nom": "Dupont" }"#));
    assert_eq!(
        error.kind,
        LoadErrorKind::WrongStructure {
            expected: "an array of objects".into()
        }
    );
    assert_eq!(error.path.to_string(), "/annuaire/personnes");
    assert!(error.to_string().contains("array"), "{error}");
}

/// Scenario "Chemin depuis la racine de l'application".
#[test]
fn path_from_the_root_of_the_application() {
    let json = r#"[ { "nom": "A" }, { "nom": "B" }, { "nom": "C" },
                    { "nom": "D", "nombre_enfants": "deux" } ]"#;
    let error = only_error(load_persons(json));
    assert_eq!(
        error.path.to_string(),
        "/annuaire/personnes/3/nombre_enfants"
    );
}

/// A single reference may be null (no child).
#[test]
fn null_single_reference() {
    let lines = dump(&load(r#"{ "zones": [ { "code": "Z", "emprise": null } ] }"#).expect("load"));
    assert_eq!(lines.len(), 2);
}

/// Scenario "Même résultat dans les deux modes".
#[test]
fn stream_and_buffer_give_the_same_fragment() {
    let json = example("reseau-meteo/reseau-exemple.json");
    let stream = load(&json).expect("stream");
    let buffer = load_slice(&json).expect("buffer");
    assert_eq!(stream.heads(), buffer.heads());
    assert_eq!(dump(&stream), dump(&buffer));
    let error = only_error(load_slice(r#"{ "stations": [ { "altitude": "haute" } ] }"#));
    assert_eq!(error.path.to_string(), "/stations/0/altitude");
}

// --- Errors gathered, diagnostics (requirements "Erreurs rassemblées", "Attribut obligatoire
// absent" and "Valeur fautive absente du journal") ---

/// A file of persons, one JSON object per entry of `persons` (the content between the braces).
fn persons(persons: &[String]) -> String {
    let objects: Vec<String> = persons.iter().map(|p| format!("{{ {p} }}")).collect();
    format!("[ {} ]", objects.join(",\n  "))
}

/// The paths of the errors of a refused load, in order.
fn paths(failure: &LoadFailure) -> Vec<String> {
    failure.errors.iter().map(|e| e.path.to_string()).collect()
}

/// Scenario "Trois erreurs dans trois objets".
#[test]
fn three_errors_in_three_objects() {
    let mut entries: Vec<String> = (0..10).map(|i| format!(r#""nom": "P{i}""#)).collect();
    entries[1].push_str(r#", "nombre_enfants": "deux""#);
    entries[4].push_str(r#", "age": 40"#);
    entries[7].push_str(r#", "_type_": "Robot""#);
    let failure = load_persons(&persons(&entries)).expect_err("three errors");
    assert_eq!(failure.error_count, 3);
    assert_eq!(
        paths(&failure),
        [
            "/annuaire/personnes/1/nombre_enfants",
            "/annuaire/personnes/4",
            "/annuaire/personnes/7"
        ]
    );
    assert!(matches!(
        failure.errors[0].kind,
        LoadErrorKind::WrongValueType { .. }
    ));
    assert_eq!(
        failure.errors[1].kind,
        LoadErrorKind::UnknownKey { key: "age".into() }
    );
    assert_eq!(
        failure.errors[2].kind,
        LoadErrorKind::UnknownClass {
            name: "Robot".into()
        }
    );
}

/// Scenario "Plafond des erreurs".
#[test]
fn cap_on_errors() {
    let entries: Vec<String> = (0..150)
        .map(|i| format!(r#""nom": "P{i}", "nombre_enfants": "beaucoup""#))
        .collect();
    let failure = load_persons(&persons(&entries)).expect_err("150 errors");
    assert_eq!(failure.error_count, 150);
    assert_eq!(failure.errors.len(), MAX_REPORTED);
    assert_eq!(
        failure.errors[99].path.to_string(),
        "/annuaire/personnes/99/nombre_enfants"
    );
}

/// Scenario "Erreur de syntaxe après une erreur de données".
#[test]
fn syntax_error_after_a_data_error() {
    let json = r#"[ { "nom": "A", "age": 3 },
  { "nom": "B" },
  { "nom": "C", "#;
    let failure = load_persons(json).expect_err("two errors");
    assert_eq!(failure.error_count, 2, "{:#?}", failure.errors);
    assert_eq!(
        failure.errors[0].kind,
        LoadErrorKind::UnknownKey { key: "age".into() }
    );
    let LoadErrorKind::Syntax { line, column } = failure.errors[1].kind else {
        panic!("{:?}", failure.errors[1])
    };
    assert_eq!(line, 3);
    assert!(column > 0);
}

/// Data errors in objects refused or skipped do not stop the reading: an object of a refused
/// class is skipped whole, the following ones are read, and the positions stay those of the file.
#[test]
fn reading_goes_on_after_refused_objects() {
    let json = with_equipments(
        r#"{ "_type_": "Barometre", "mesures": [ { "_type_": "Releve", "valeur": 1 } ] },
           { "numero_serie": "late", "mesures": [ { "valeur": 2 } ], "_type_": "Robot" },
           { "_type_": "Releve", "instant": "2026-01-01T00:00:00Z" },
           { "_type_": "Camera", "numero_serie": "C", "zoom": { "x": [1, 2] } },
           { "_type_": "Thermometre", "numero_serie": "T", "altitude": [3] }"#,
    );
    let failure = load(&json).expect_err("errors");
    assert_eq!(
        paths(&failure),
        [
            "/stations/0/equipements/0",
            "/stations/0/equipements/1",
            "/stations/0/equipements/2",
            "/stations/0/equipements/3",
            "/stations/0/equipements/4"
        ],
        "{:#?}",
        failure.errors
    );
    // The value of an object refused, or of an unknown key, is never kept.
    assert!(failure.errors.iter().all(|e| e.value.is_none()));
}

/// Scenario "Personne sans nom".
#[test]
fn person_without_a_name() {
    let entries = [
        r#""nom": "A""#.to_owned(),
        r#""prenom": "B""#.to_owned(),
        r#""nom": "C""#.to_owned(),
        r#""nom": null, "prenom": "D""#.to_owned(),
    ];
    let loaded = load_persons(&persons(&entries)).expect("load");
    assert_eq!(loaded.fragment.heads().len(), 4);
    assert_eq!(loaded.diagnostic_count, 2);
    let diagnostics: Vec<String> = loaded.diagnostics.iter().map(|d| d.to_string()).collect();
    assert_eq!(
        diagnostics,
        [
            "/annuaire/personnes/1: required attribute `nom` has no value",
            "/annuaire/personnes/3: required attribute `nom` has no value"
        ]
    );
    assert_eq!(
        loaded.diagnostics[0].kind,
        DiagnosticKind::MissingRequired {
            attribute: "nom".into()
        }
    );
}

/// Scenario "Diagnostic et erreur dans le même fichier".
#[test]
fn diagnostic_and_error_in_the_same_file() {
    let entries = [
        r#""prenom": "A""#.to_owned(),
        r#""nom": "B", "age": 3"#.to_owned(),
    ];
    let failure = load_persons(&persons(&entries)).expect_err("unknown key");
    assert_eq!(failure.error_count, 1);
    assert_eq!(failure.diagnostic_count, 1);
    assert_eq!(
        failure.diagnostics[0].path.to_string(),
        "/annuaire/personnes/0"
    );
}

/// Scenario "Les deux formes d'une erreur".
#[test]
fn the_two_forms_of_an_error() {
    let entries = [r#""nom": "A", "nombre_enfants": "deux""#.to_owned()];
    let error = only_error(load_persons(&persons(&entries)));
    let user = error.user_message();
    assert!(user.contains("deux"), "{user}");
    let log = error.to_string();
    assert!(!log.contains("deux"), "{log}");
    for part in [
        "nombre_enfants",
        "Int",
        "/annuaire/personnes/0/nombre_enfants",
    ] {
        assert!(log.contains(part), "{part}: {log}");
    }
    // The form for the log is the one `{}` gives, also through the failure.
    let failure = load_persons(&persons(&entries)).expect_err("error");
    assert!(!failure.to_string().contains("deux"), "{failure}");
}

/// Scenario "Valeur longue": 40 characters (not bytes) then `…`.
#[test]
fn long_value() {
    let long = "é".repeat(1000);
    let entries = [format!(r#""nom": "A", "nombre_enfants": "{long}""#)];
    let error = only_error(load_persons(&persons(&entries)));
    let expected = format!("{}…", "é".repeat(40));
    assert_eq!(error.value.as_deref(), Some(expected.as_str()));
    assert!(error.user_message().contains(&expected));
    assert!(!error.user_message().contains(&"é".repeat(41)));
}

/// A list element that is not an object, or a `_type_` that is not a text, is an error of the
/// data, not of syntax: the reading goes on, and the positions stay those of the file.
#[test]
fn values_that_are_not_objects_in_a_list() {
    let json = r#"[ { "nom": "A" }, "secret", { "nom": 3 }, [1, 2], { "_type_": 5, "nom": "B" },
                    { "nom": "C" } ]"#;
    let failure = load_persons(json).expect_err("errors");
    let errors: Vec<String> = failure.errors.iter().map(|e| e.to_string()).collect();
    assert_eq!(
        errors,
        [
            "/annuaire/personnes/1: expected an object",
            "/annuaire/personnes/2/nom: attribute `nom` expects a value of type `String`",
            "/annuaire/personnes/3: expected an object",
            "/annuaire/personnes/4: expected a class name",
        ]
    );
    assert!(
        failure
            .errors
            .iter()
            .all(|e| e.value.as_deref() != Some("secret"))
    );

    // In a network, `_type_` read late (after another key) or first.
    for equipment in [
        r#"{ "numero_serie": "T", "_type_": ["Thermometre"] }"#,
        r#"{ "_type_": { "nom": "Thermometre" }, "numero_serie": "T" }"#,
    ] {
        let error = only_error(load(&with_equipments(equipment)));
        assert_eq!(
            error.to_string(),
            "/stations/0/equipements/0: expected a class name"
        );
    }
}

/// An unknown key or class name comes from the file: the form for the log leaves it out, the
/// message for the user gives it.
#[test]
fn unknown_names_stay_out_of_the_log() {
    let entries = [r#""nom": "A", "Dupont Jean, 12 rue X": 1"#.to_owned()];
    let error = only_error(load_persons(&persons(&entries)));
    assert_eq!(error.to_string(), "/annuaire/personnes/0: unknown key");
    assert_eq!(
        error.user_message(),
        "/annuaire/personnes/0: unknown key, found `Dupont Jean, 12 rue X`"
    );
    let error = only_error(load(&with_equipments(r#"{ "_type_": "Martin Paul" }"#)));
    assert!(!error.to_string().contains("Martin"), "{error}");
    assert!(
        error.user_message().contains("`Martin Paul`"),
        "{}",
        error.user_message()
    );
}

/// A list key repeated in an object appends to the list: the positions of the second array
/// continue after the first.
#[test]
fn repeated_list_key_continues_the_positions() {
    let json = r#"{ "stations": [ { "code": "A", "nom": "A" }, { "code": "B", "nom": "B" } ],
                    "stations": [ { "code": "C", "nom": "C", "altitude": "x" } ] }"#;
    let error = only_error(load(json));
    assert_eq!(error.path.to_string(), "/stations/2/altitude");
}
