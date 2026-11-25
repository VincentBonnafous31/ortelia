//! Tests of `app.json` (specification `app-config`): reading the file, then checking it against
//! its model.

use std::sync::Arc;

use ortelia_config::{AppFile, CheckedApp, ConfigError, SourceFormat};
use ortelia_model::builder::DataPath;
use ortelia_model::fragment::Shape;
use ortelia_model::metamodel::{Model, ModelFile};

fn example(path: &str) -> String {
    let path = format!("{}/../../examples/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn compile(text: &str) -> Arc<Model> {
    Arc::new(Model::compile(&[ModelFile::parse(text).expect("parse")]).expect("compile"))
}

fn directory_model() -> Arc<Model> {
    compile(&example("annuaire/annuaire.model.json"))
}

fn network_model() -> Arc<Model> {
    compile(&example("reseau-meteo/reseau.model.json"))
}

/// An `app.json` of root `root`, with one source per `(id, target)`.
fn app(root: &str, sources: &[(&str, &str)]) -> AppFile {
    let sources: Vec<String> = sources
        .iter()
        .map(|(id, target)| {
            format!(
                r#"{{ "id": "{id}", "format": "json", "file": "{id}.json", "target": "{target}" }}"#
            )
        })
        .collect();
    let text = format!(
        r#"{{ "formatVersion": "0.3", "id": "test", "name": "Test",
             "model": {{ "files": ["m.model.json"], "root": "{root}" }},
             "sources": [ {} ] }}"#,
        sources.join(", ")
    );
    AppFile::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// The errors of the check of an application.
fn check_errors(app: &AppFile, model: &Model) -> Vec<ConfigError> {
    app.check(model).expect_err("errors")
}

/// The error of reading a text of `app.json`, as displayed.
fn parse_error(text: &str) -> String {
    AppFile::parse(text).expect_err("refused").to_string()
}

/// A valid `app.json`, as a text to alter.
const VALID: &str = r#"{
  "formatVersion": "0.3",
  "id": "a",
  "name": "A",
  "model": { "files": ["m.model.json"], "root": "Racine" },
  "sources": [
    { "id": "s", "format": "json", "file": "s.json", "target": "/" }
  ]
}"#;

// --- Reading ---

/// Scenario "Application de l'annuaire", then "Cible de l'annuaire".
#[test]
fn application_of_the_directory() {
    let app = AppFile::parse(&example("annuaire/app.json")).expect("parse");
    assert_eq!(app.id, "annuaire");
    assert_eq!(app.model.files, ["annuaire.model.json"]);
    assert_eq!(app.model.root, "Racine");
    assert_eq!(app.sources.len(), 1);
    let source = &app.sources[0];
    assert_eq!(
        (
            source.id.as_str(),
            source.format,
            source.file.as_str(),
            source.target.as_str()
        ),
        (
            "personnes",
            SourceFormat::Json,
            "personnes.json",
            "/annuaire/personnes"
        )
    );

    let model = directory_model();
    let checked = app.check(&model).expect("check");
    let racine = model.class_id("Racine").expect("Racine");
    let annuaire = model.class_id("Annuaire").expect("Annuaire");
    assert_eq!(checked.root, racine);
    let target = &checked.sources[0].target;
    let personnes = model.class(annuaire).reference("personnes").expect("ref");
    assert_eq!(
        target.refs,
        [
            model.class(racine).reference("annuaire").expect("ref"),
            personnes
        ]
    );
    assert_eq!(target.shape, Shape::of(&model, personnes));
    assert!(target.shape.many);
    assert_eq!(target.data_path(&model).to_string(), "/annuaire/personnes");
}

/// Scenario "Clé inconnue".
#[test]
fn unknown_key() {
    let text = VALID.replace(r#""sources""#, r#""sourcse""#);
    let message = parse_error(&text);
    assert!(message.contains("sourcse"), "{message}");
    assert!(message.starts_with("line 6, column "), "{message}");
}

/// Scenario "Clé obligatoire absente".
#[test]
fn missing_key() {
    let text = VALID.replace(r#", "target": "/""#, "");
    let message = parse_error(&text);
    assert!(message.contains("`target`"), "{message}");
    assert!(message.starts_with("line "), "{message}");
}

/// Scenario "Format de source inconnu".
#[test]
fn unknown_source_format() {
    let text = VALID.replace(r#""format": "json""#, r#""format": "xml""#);
    let message = parse_error(&text);
    assert!(message.contains("xml"), "{message}");
}

/// Malformed JSON, and a value of the wrong type.
#[test]
fn malformed_file() {
    let error = AppFile::parse(r#"{ "formatVersion": "0.3", "id": "#).expect_err("truncated");
    assert!(
        matches!(error, ConfigError::Syntax { line: 1, .. }),
        "{error}"
    );
    let message = parse_error(&VALID.replace(r#""id": "a""#, r#""id": 3"#));
    assert!(message.starts_with("line 3, column "), "{message}");
}

/// Scenario "Ancienne version".
#[test]
fn old_version() {
    let error = AppFile::parse(&VALID.replace("0.3", "0.2")).expect_err("0.2");
    assert_eq!(
        error,
        ConfigError::Version {
            found: "0.2".into()
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("`0.2`") && message.contains("`0.3`"),
        "{message}"
    );
}

/// Scenario "Fichier avec `$schema`".
#[test]
fn schema_key() {
    let with_schema = VALID.replacen('{', r#"{ "$schema": "../../schemas/app.schema.json","#, 1);
    let mut app = AppFile::parse(&with_schema).expect("with $schema");
    assert_eq!(app.schema.as_deref(), Some("../../schemas/app.schema.json"));
    app.schema = None;
    assert_eq!(app, AppFile::parse(VALID).expect("without $schema"));
}

// --- Checking against the model ---

/// Scenario "Racine introuvable".
#[test]
fn unknown_root() {
    let errors = check_errors(&app("Racinne", &[]), &directory_model());
    assert_eq!(
        errors,
        [ConfigError::UnknownRoot {
            class: "Racinne".into()
        }]
    );
    assert!(errors[0].to_string().contains("Racinne"));
}

/// Scenario "Racine abstraite".
#[test]
fn abstract_root() {
    let errors = check_errors(&app("Equipement", &[]), &network_model());
    assert_eq!(
        errors,
        [ConfigError::AbstractRoot {
            class: "Equipement".into()
        }]
    );
    assert!(errors[0].to_string().contains("Equipement"));
}

/// Scenario "Segment inconnu".
#[test]
fn unknown_segment() {
    let errors = check_errors(
        &app("Racine", &[("personnes", "/annuaire/persones")]),
        &directory_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::UnknownSegment {
            source: "personnes".into(),
            segment: "persones".into(),
            class: "Annuaire".into()
        }]
    );
    let message = errors[0].to_string();
    for part in ["personnes", "persones", "Annuaire"] {
        assert!(message.contains(part), "{part}: {message}");
    }
}

/// Scenario "Segment intermédiaire multiple".
#[test]
fn list_traversed() {
    let errors = check_errors(
        &app("Reseau", &[("s", "/stations/equipements")]),
        &network_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::ListTraversed {
            source: "s".into(),
            segment: "stations".into(),
            class: "Reseau".into()
        }]
    );
    assert!(errors[0].to_string().contains("is a list"), "{}", errors[0]);
}

/// A small model: a root with a single reference to an abstract class, and two classes that
/// each have a reference named `items`.
const SMALL_MODEL: &str = r#"{
  "formatVersion": "0.1",
  "packages": [ { "name": "p", "classes": [
    { "name": "Racine", "references": [
      { "name": "forme", "type": "Forme", "upperBound": 1 },
      { "name": "a", "type": "A", "upperBound": 1 },
      { "name": "b", "type": "B", "upperBound": 1 } ] },
    { "name": "Forme", "abstract": true },
    { "name": "Cercle", "superClass": "Forme", "references": [
      { "name": "points", "type": "Point", "upperBound": -1 } ] },
    { "name": "A", "references": [ { "name": "items", "type": "Point", "upperBound": -1 } ] },
    { "name": "B", "references": [ { "name": "items", "type": "Point", "upperBound": -1 } ] },
    { "name": "Point" }
  ] } ]
}"#;

/// Scenario "Segment intermédiaire de type abstrait".
#[test]
fn abstract_class_traversed() {
    let errors = check_errors(
        &app("Racine", &[("s", "/forme/points")]),
        &compile(SMALL_MODEL),
    );
    assert_eq!(
        errors,
        [ConfigError::AbstractTraversed {
            source: "s".into(),
            segment: "forme".into(),
            class: "Forme".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("forme") && message.contains("Forme"),
        "{message}"
    );
    // The abstract single reference may be the branch itself: its object gives its class.
    let checked = app("Racine", &[("s", "/forme")])
        .check(&compile(SMALL_MODEL))
        .expect("abstract branch");
    assert!(!checked.sources[0].target.shape.many);
}

/// Scenario "Cible à la racine".
#[test]
fn target_at_the_root() {
    let model = network_model();
    let checked: CheckedApp = app("Reseau", &[("tout", "/")])
        .check(&model)
        .expect("check");
    let target = &checked.sources[0].target;
    assert!(target.refs.is_empty());
    assert_eq!(
        target.shape,
        Shape::root(model.class_id("Reseau").expect("Reseau"))
    );
    assert_eq!(target.data_path(&model), DataPath::default());
}

/// Targets that are not paths of names.
#[test]
fn invalid_targets() {
    for target in ["", "stations", "/stations/", "//stations"] {
        let errors = check_errors(&app("Reseau", &[("s", target)]), &network_model());
        assert_eq!(
            errors,
            [ConfigError::InvalidTarget {
                source: "s".into(),
                target: target.into()
            }],
            "{target}"
        );
    }
}

/// Scenario "Même branche".
#[test]
fn same_branch() {
    let errors = check_errors(
        &app("Reseau", &[("s1", "/stations"), ("s2", "/stations")]),
        &network_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::SourceConflict {
            first: "s1".into(),
            second: "s2".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("s1") && message.contains("s2"),
        "{message}"
    );
}

/// Scenario "Branches imbriquées", in both orders, and the root that excludes any other
/// source.
#[test]
fn nested_branches() {
    let model = directory_model();
    for sources in [
        [("a", "/annuaire"), ("p", "/annuaire/personnes")],
        [("p", "/annuaire/personnes"), ("a", "/annuaire")],
        [("p", "/annuaire/personnes"), ("a", "/")],
    ] {
        let errors = check_errors(&app("Racine", &sources), &model);
        assert_eq!(
            errors,
            [ConfigError::SourceConflict {
                first: sources[0].0.into(),
                second: sources[1].0.into()
            }],
            "{sources:?}"
        );
    }
}

/// Scenario "Deux branches distinctes", and two references of the same name in two classes.
#[test]
fn distinct_branches() {
    let checked = app("Reseau", &[("s", "/stations"), ("z", "/zones")])
        .check(&network_model())
        .expect("distinct");
    assert_eq!(checked.sources.len(), 2);
    app("Racine", &[("a", "/a/items"), ("b", "/b/items")])
        .check(&compile(SMALL_MODEL))
        .expect("same names, other classes");
}

/// Scenario "Identifiant en double": reported once, even for three sources.
#[test]
fn duplicate_identifier() {
    let errors = check_errors(
        &app(
            "Reseau",
            &[
                ("personnes", "/stations"),
                ("personnes", "/zones"),
                ("personnes", "/nom"),
            ],
        ),
        &network_model(),
    );
    assert_eq!(
        errors[0],
        ConfigError::DuplicateSource {
            id: "personnes".into()
        }
    );
    assert!(errors[0].to_string().contains("personnes"));
    assert_eq!(
        errors
            .iter()
            .filter(|e| matches!(e, ConfigError::DuplicateSource { .. }))
            .count(),
        1,
        "{errors:?}"
    );
}

/// Scenario "Deux cibles fautives".
#[test]
fn two_wrong_targets() {
    let errors = check_errors(
        &app("Reseau", &[("s", "/statoins"), ("z", "/zonse")]),
        &network_model(),
    );
    let sources: Vec<&str> = errors
        .iter()
        .map(|e| match e {
            ConfigError::UnknownSegment { source, .. } => source.as_str(),
            other => panic!("{other}"),
        })
        .collect();
    assert_eq!(sources, ["s", "z"]);
}
