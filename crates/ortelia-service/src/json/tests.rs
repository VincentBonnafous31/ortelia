//! Tests of the JSON reader (specification `data-loading`).

use std::sync::Arc;

use ortelia_model::builder::{LoadError, LoadErrorKind};
use ortelia_model::metamodel::{Model, ModelFile};
use ortelia_model::store::{ObjectId, Store};
use ortelia_model::value::Value;

use super::{read_json, read_json_slice};

fn example(name: &str) -> String {
    let path = format!(
        "{}/../../examples/reseau-meteo/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn network_model() -> Arc<Model> {
    let file = ModelFile::parse(&example("reseau.model.json")).expect("parse");
    Arc::new(Model::compile(&[file]).expect("compile"))
}

fn load(json: &str) -> Result<Store, LoadError> {
    let model = network_model();
    let root = model.class_id("Reseau").expect("Reseau");
    read_json(json.as_bytes(), model, root)
}

fn load_slice(json: &str) -> Result<Store, LoadError> {
    let model = network_model();
    let root = model.class_id("Reseau").expect("Reseau");
    read_json_slice(json.as_bytes(), model, root)
}

/// One line per object, depth first, in file order: depth, class, and the values that are set.
fn dump(store: &Store) -> Vec<String> {
    let model = store.model();
    let mut lines = Vec::new();
    let mut stack: Vec<(ObjectId, usize)> = vec![(store.root().expect("root"), 0)];
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
    let store = load(&example("reseau-exemple.json")).expect("load");
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
        let error = load(&with_equipments(json)).expect_err("conflicting");
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
    let error = load(&int).expect_err("int");
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
    let error = load(&with_equipments(r#"{ "numero_serie": "X" }"#)).expect_err("abstract");
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
        let error = load(&with_equipments(json)).expect_err("non conforming");
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
    let error = load(wrong).expect_err("wrong type");
    assert!(
        matches!(error.kind, LoadErrorKind::NonConformingClass { .. }),
        "{error}"
    );
}

#[test]
fn unknown_type_name() {
    let error = load(&with_equipments(r#"{ "_type_": "Barometre" }"#)).expect_err("unknown");
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
    let error = load(&json).expect_err("unknown key");
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
    let error =
        load(r#"{ "stations": [ { "code": "A" }, { "code": "B", "altitude": "haute" } ] }"#)
            .expect_err("wrong type");
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
        let error = load(&json).expect_err(value);
        assert!(
            matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
            "{value}: {error}"
        );
    }
}

/// Scenario "JSON mal formé", and other structural errors.
#[test]
fn malformed_json() {
    let error = load("{\n  \"stations\": [\n    { \"code\": \"A\"").expect_err("truncated");
    let LoadErrorKind::Syntax { line, column } = error.kind else {
        panic!("{error}")
    };
    assert_eq!(line, 3);
    assert!(column > 0);
    assert!(error.to_string().contains("line 3"), "{error}");
    for json in [
        r#"{ "nom": "a" } x"#,
        r#"{ "stations": {} }"#,
        r#"{ "zones": [ { "emprise": [] } ] }"#,
        "[]",
    ] {
        let error = load(json).expect_err(json);
        assert!(
            matches!(error.kind, LoadErrorKind::Syntax { .. }),
            "{json}: {error}"
        );
    }
}

/// A single reference may be null (no child).
#[test]
fn null_single_reference() {
    let lines = dump(&load(r#"{ "zones": [ { "code": "Z", "emprise": null } ] }"#).expect("load"));
    assert_eq!(lines.len(), 2);
}

/// Scenario "Même résultat dans les deux modes".
#[test]
fn stream_and_buffer_give_the_same_store() {
    let json = example("reseau-exemple.json");
    let stream = dump(&load(&json).expect("stream"));
    let buffer = dump(&load_slice(&json).expect("buffer"));
    assert_eq!(stream, buffer);
    let error =
        load_slice(r#"{ "stations": [ { "altitude": "haute" } ] }"#).expect_err("wrong type");
    assert_eq!(error.path.to_string(), "/stations/0/altitude");
}
