//! Tests of `app.json` (specification `app-config`): reading the file, then checking it against
//! its model.

use std::sync::Arc;

use ortelia_config::{
    AppFile, CheckedApp, ConfigError, ConfigWarning, Content, PatternError, SortOrder,
    SourceFormat, TableWidget, ViewDef, Widget,
};
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

/// An `app.json` of root `root`, with one source per `(id, target)`, and a window that shows
/// a list of the model of this root.
fn app(root: &str, sources: &[(&str, &str)]) -> AppFile {
    let data = match root {
        "Reseau" => "/stations",
        _ => "/annuaire/personnes",
    };
    app_with(root, data, sources)
}

/// An `app.json` of root `root`, whose window shows `data`, with one source per `(id, target)`.
fn app_with(root: &str, data: &str, sources: &[(&str, &str)]) -> AppFile {
    let sources: Vec<String> = sources
        .iter()
        .map(|(id, target)| {
            format!(
                r#"{{ "id": "{id}", "format": "json", "file": "{id}.json", "target": "{target}" }}"#
            )
        })
        .collect();
    let text = format!(
        r#"{{ "formatVersion": "0.4", "id": "test", "name": "Test", "languages": ["fr"],
             "model": {{ "files": ["m.model.json"], "root": "{root}" }},
             "sources": [ {} ],
             "window": {{ "content": {{ "_type_": "View", "id": "v",
                 "widget": {{ "_type_": "Table" }}, "data": "{data}" }} }} }}"#,
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
  "formatVersion": "0.4",
  "id": "a",
  "name": "A",
  "model": { "files": ["m.model.json"], "root": "Racine" },
  "sources": [
    { "id": "s", "format": "json", "file": "s.json", "target": "/" }
  ],
  "languages": ["fr"],
  "window": { "content": { "_type_": "View", "id": "v", "widget": { "_type_": "Table" },
                           "data": "/annuaire/personnes" } }
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

    // Languages, presentation of `Personne`, and a window that shows the persons in a table.
    assert_eq!(app.languages, ["fr", "en"]);
    assert_eq!(app.presentations.len(), 1);
    assert_eq!(app.presentations[0].class, "Personne");
    let Content::View(view) = &app.window.content;
    assert_eq!(view.widget, Widget::Table(TableWidget {}));
    assert_eq!(view.data, "/annuaire/personnes");
    let personne = model.class_id("Personne").expect("Personne");
    assert_eq!(checked.window.view.class, personne);
    assert_eq!(checked.window.view.data, target.refs);
    let table = checked.presentations[&personne]
        .table
        .as_ref()
        .expect("table");
    // All the columns but `notes`, `date_naissance` with its format, and the sort.
    let names: Vec<&str> = table
        .columns
        .iter()
        .map(|c| model.attribute(c.attr).name())
        .collect();
    assert_eq!(
        names,
        [
            "nom",
            "prenom",
            "date_naissance",
            "nombre_enfants",
            "taille",
            "contact_autorise",
            "situation",
            "inscription"
        ]
    );
    assert!(table.columns[2].format.is_some());
    let attr = |name| model.class(personne).attribute(name).expect("attribute");
    assert_eq!(
        table.sort,
        [
            (attr("nom"), SortOrder::Asc),
            (attr("prenom"), SortOrder::Asc)
        ]
    );
    assert!(checked.warnings.is_empty(), "{:?}", checked.warnings);
}

// --- Format 0.4: languages, window, content, presentations, labels ---

/// `VALID` with `languages` replaced.
fn with_languages(languages: &str) -> String {
    VALID.replace(
        r#""languages": ["fr"]"#,
        &format!(r#""languages": {languages}"#),
    )
}

/// `VALID` with the given `window`.
fn with_window(window: &str) -> String {
    let start = VALID.find(r#""window""#).expect("window");
    format!("{}\"window\": {window}\n}}", &VALID[..start])
}

/// Scenario "Liste vide".
#[test]
fn no_language() {
    let errors = check_errors(
        &AppFile::parse(&with_languages("[]")).expect("parse"),
        &directory_model(),
    );
    assert_eq!(errors, [ConfigError::NoLanguage]);
    assert!(errors[0].to_string().contains("languages"), "{}", errors[0]);
}

/// Scenario "Langue en double".
#[test]
fn duplicate_language() {
    let errors = check_errors(
        &AppFile::parse(&with_languages(r#"["fr", "fr"]"#)).expect("parse"),
        &directory_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::DuplicateLanguage { lang: "fr".into() }]
    );
    assert!(errors[0].to_string().contains("`fr`"), "{}", errors[0]);
}

/// The languages and the window are required.
#[test]
fn languages_and_window_are_required() {
    let without_languages = VALID.replace(r#""languages": ["fr"],"#, "");
    assert!(parse_error(&without_languages).contains("`languages`"));
    let end = VALID.find(",\n  \"window\"").expect("window");
    let without_window = format!("{}\n}}", &VALID[..end]);
    let message = parse_error(&without_window);
    assert!(message.contains("`window`"), "{message}");
}

const TABLE_CONTENT: &str = r#"{ "_type_": "View", "id": "personnes",
    "widget": { "_type_": "Table" }, "data": "/annuaire/personnes" }"#;

/// Scenario "Fenêtre de l'annuaire".
#[test]
fn window_of_the_directory() {
    let checked = AppFile::parse(&example("annuaire/app.json"))
        .expect("parse")
        .check(&directory_model())
        .expect("check");
    assert_eq!(checked.window.title, "Annuaire");
    assert_eq!(
        (checked.window.width, checked.window.height),
        (Some(1000), Some(700))
    );
}

/// Scenario "Titre par défaut".
#[test]
fn default_title() {
    let text = with_window(&format!(r#"{{ "content": {TABLE_CONTENT} }}"#));
    let checked = AppFile::parse(&text)
        .expect("parse")
        .check(&directory_model())
        .expect("check");
    assert_eq!(checked.window.title, "A");
    assert_eq!((checked.window.width, checked.window.height), (None, None));
}

/// Scenario "Taille hors limites", for the width and the height; the limits are accepted.
#[test]
fn window_size_out_of_range() {
    let check = |width: u32, height: u32| {
        let text = with_window(&format!(
            r#"{{ "width": {width}, "height": {height}, "content": {TABLE_CONTENT} }}"#
        ));
        AppFile::parse(&text)
            .expect("parse")
            .check(&directory_model())
    };
    let errors = check(50, 700).expect_err("50");
    assert_eq!(
        errors,
        [ConfigError::WindowSize {
            key: "width",
            value: 50
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("width") && message.contains("50"),
        "{message}"
    );
    assert_eq!(
        check(1000, 10_001).expect_err("height"),
        [ConfigError::WindowSize {
            key: "height",
            value: 10_001
        }]
    );
    check(200, 10_000).expect("limits");
}

/// Scenario "Vue table".
#[test]
fn table_view() {
    let app = AppFile::parse(&with_window(&format!(
        r#"{{ "content": {TABLE_CONTENT} }}"#
    )))
    .expect("parse");
    assert_eq!(
        app.window.content,
        Content::View(ViewDef {
            id: "personnes".into(),
            widget: Widget::Table(TableWidget {}),
            data: "/annuaire/personnes".into()
        })
    );
}

/// Scenario "Conteneur inconnu", and an unknown widget.
#[test]
fn unknown_container() {
    let message = parse_error(&with_window(r#"{ "content": { "_type_": "Split" } }"#));
    assert!(message.contains("Split"), "{message}");
    let message = parse_error(&with_window(&format!(
        r#"{{ "content": {} }}"#,
        TABLE_CONTENT.replace(r#""Table""#, r#""Tree""#)
    )));
    assert!(message.contains("Tree"), "{message}");
}

/// Unknown keys are refused in a view and in a table, despite the internal tag (design.md,
/// Risks); the message names the key.
#[test]
fn unknown_keys_in_polymorphic_objects() {
    let in_view = TABLE_CONTENT.replace(r#""id""#, r#""titre": "x", "id""#);
    let message = parse_error(&with_window(&format!(r#"{{ "content": {in_view} }}"#)));
    assert!(message.contains("titre"), "{message}");
    let in_table = TABLE_CONTENT.replace(r#""_type_": "Table""#, r#""_type_": "Table", "x": 1"#);
    let message = parse_error(&with_window(&format!(r#"{{ "content": {in_table} }}"#)));
    assert!(message.contains("`x`"), "{message}");
    // A missing `_type_` is refused too.
    let untyped = TABLE_CONTENT.replace(r#""_type_": "View", "#, "");
    let message = parse_error(&with_window(&format!(r#"{{ "content": {untyped} }}"#)));
    assert!(message.contains("_type_"), "{message}");
}

/// Scenario "Données de l'annuaire" is in `application_of_the_directory`; scenario "Données
/// qui ne sont pas une liste", and the other errors of the data path.
#[test]
fn data_of_a_view() {
    let model = directory_model();
    let errors = check_errors(&app_with("Racine", "/annuaire", &[]), &model);
    assert_eq!(
        errors,
        [ConfigError::DataNotList {
            view: "v".into(),
            segment: "annuaire".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("`v`") && message.contains("`annuaire` is not a list"),
        "{message}"
    );
    assert_eq!(
        check_errors(&app_with("Racine", "/annuaire/persones", &[]), &model),
        [ConfigError::UnknownDataSegment {
            view: "v".into(),
            segment: "persones".into(),
            class: "Annuaire".into()
        }]
    );
    for data in ["/", "annuaire", "/annuaire/"] {
        assert_eq!(
            check_errors(&app_with("Racine", data, &[]), &model),
            [ConfigError::InvalidData {
                view: "v".into(),
                data: data.into()
            }],
            "{data}"
        );
    }
    let network = network_model();
    assert!(matches!(
        &check_errors(&app_with("Reseau", "/stations/equipements", &[]), &network)[..],
        [ConfigError::DataListTraversed { segment, .. }] if segment == "stations"
    ));
    // An abstract single reference can be traversed by a view.
    let checked = app_with("Racine", "/forme/sommets", &[])
        .check(&compile(SMALL_MODEL))
        .expect("abstract traversed");
    assert_eq!(checked.window.view.data.len(), 2);
}

/// `VALID` with the given presentations.
fn with_presentations(presentations: &str) -> AppFile {
    let text = VALID.replace(
        r#""languages""#,
        &format!(r#""presentations": {presentations}, "languages""#),
    );
    AppFile::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// The errors of `VALID` with the given presentations, against the directory model.
fn presentation_errors(presentations: &str) -> Vec<ConfigError> {
    check_errors(&with_presentations(presentations), &directory_model())
}

/// A presentation of `Personne` with the given columns and sort.
fn person_table(columns: &str, sort: &str) -> String {
    format!(r#"[ {{ "class": "Personne", "table": {{ "columns": {columns}, "sort": {sort} }} }} ]"#)
}

/// Scenario "Classe inconnue".
#[test]
fn presentation_of_an_unknown_class() {
    let errors = presentation_errors(r#"[ { "class": "Persone" } ]"#);
    assert_eq!(
        errors,
        [ConfigError::UnknownPresentationClass {
            class: "Persone".into()
        }]
    );
    assert!(errors[0].to_string().contains("Persone"));
}

/// Scenario "Deux présentations pour une classe".
#[test]
fn two_presentations_of_a_class() {
    let errors = presentation_errors(r#"[ { "class": "Personne" }, { "class": "Personne" } ]"#);
    assert_eq!(
        errors,
        [ConfigError::DuplicatePresentation {
            class: "Personne".into()
        }]
    );
    assert!(errors[0].to_string().contains("Personne"));
}

/// A presentation without table is valid: its columns will be deduced from the model.
#[test]
fn presentation_without_table() {
    let model = directory_model();
    let checked = with_presentations(r#"[ { "class": "Personne" } ]"#)
        .check(&model)
        .expect("check");
    let personne = model.class_id("Personne").expect("Personne");
    assert_eq!(checked.presentations[&personne].table, None);
}

/// Scenario "Attribut inconnu".
#[test]
fn column_of_an_unknown_attribute() {
    let errors = presentation_errors(&person_table(r#"[ { "path": "prenon" } ]"#, "[]"));
    assert_eq!(
        errors,
        [ConfigError::UnknownColumn {
            class: "Personne".into(),
            path: "prenon".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("Personne") && message.contains("prenon"),
        "{message}"
    );
}

/// Scenario "Colonne en double", an empty table and a width out of range.
#[test]
fn wrong_columns() {
    let errors = presentation_errors(&person_table(
        r#"[ { "path": "nom" }, { "path": "nom" } ]"#,
        "[]",
    ));
    assert_eq!(
        errors,
        [ConfigError::DuplicateColumn {
            class: "Personne".into(),
            path: "nom".into()
        }]
    );
    assert!(errors[0].to_string().contains("nom"));
    assert_eq!(
        presentation_errors(&person_table("[]", "[]")),
        [ConfigError::NoColumn {
            class: "Personne".into()
        }]
    );
    assert_eq!(
        presentation_errors(&person_table(r#"[ { "path": "nom", "width": 10 } ]"#, "[]")),
        [ConfigError::ColumnWidth {
            class: "Personne".into(),
            path: "nom".into(),
            width: 10
        }]
    );
}

/// An inherited attribute can be a column of a subclass.
#[test]
fn column_of_an_inherited_attribute() {
    let model = compile(INHERITED_MODEL);
    let checked = with_presentations(
        r#"[ { "class": "Employe", "table": {
               "columns": [ { "path": "nom" }, { "path": "matricule" } ] } } ]"#,
    )
    .check(&model)
    .expect("check");
    let employe = model.class_id("Employe").expect("Employe");
    let personne = model.class_id("Personne").expect("Personne");
    let columns = &checked.presentations[&employe]
        .table
        .as_ref()
        .expect("table")
        .columns;
    assert_eq!(columns[0].attr.owner(), personne);
}

/// Scenario "Alignement inconnu".
#[test]
fn unknown_alignment() {
    let text = VALID.replace(
        r#""languages""#,
        &format!(
            r#""presentations": {}, "languages""#,
            person_table(r#"[ { "path": "nom", "align": "middle" } ]"#, "[]")
        ),
    );
    let message = parse_error(&text);
    assert!(message.contains("middle"), "{message}");
}

/// Scenario "Tri sur une colonne absente", and a column twice in the sort.
#[test]
fn sort_on_a_missing_column() {
    let columns = r#"[ { "path": "nom" }, { "path": "prenom" } ]"#;
    let errors = presentation_errors(&person_table(
        columns,
        r#"[ { "path": "date_naissance" } ]"#,
    ));
    assert_eq!(
        errors,
        [ConfigError::UnknownSortColumn {
            class: "Personne".into(),
            path: "date_naissance".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("Personne") && message.contains("date_naissance"),
        "{message}"
    );
    assert_eq!(
        presentation_errors(&person_table(
            columns,
            r#"[ { "path": "nom" }, { "path": "nom", "order": "desc" } ]"#
        )),
        [ConfigError::DuplicateSort {
            class: "Personne".into(),
            path: "nom".into()
        }]
    );
}

/// Scenario "Sens par défaut", and the descending order.
#[test]
fn default_sort_order() {
    let model = directory_model();
    let checked = with_presentations(&person_table(
        r#"[ { "path": "nom" }, { "path": "prenom" } ]"#,
        r#"[ { "path": "nom" }, { "path": "prenom", "order": "desc" } ]"#,
    ))
    .check(&model)
    .expect("check");
    let personne = model.class_id("Personne").expect("Personne");
    let orders: Vec<SortOrder> = checked.presentations[&personne]
        .table
        .as_ref()
        .expect("table")
        .sort
        .iter()
        .map(|&(_, order)| order)
        .collect();
    assert_eq!(orders, [SortOrder::Asc, SortOrder::Desc]);
}

/// The error of a column of `Personne` with a format.
fn format_errors(path: &str, format: &str) -> Vec<ConfigError> {
    presentation_errors(&person_table(
        &format!(r#"[ {{ "path": "{path}", "format": "{format}" }} ]"#),
        "[]",
    ))
}

/// Scenario "Format sur un entier" (`value-formats`).
#[test]
fn format_on_an_integer() {
    let errors = format_errors("nombre_enfants", "dd");
    assert_eq!(
        errors,
        [ConfigError::FormatNotDate {
            class: "Personne".into(),
            path: "nombre_enfants".into()
        }]
    );
    assert!(errors[0].to_string().contains("nombre_enfants"));
}

/// Scenarios "Lettre inconnue" and "Heure pour une date" (`value-formats`), when checking the
/// application; a time in the format of a date-time is valid.
#[test]
fn invalid_patterns() {
    let errors = format_errors("date_naissance", "dd/MM/yyyy Q");
    assert!(
        matches!(&errors[..], [ConfigError::InvalidPattern { path, pattern, error: PatternError::UnknownField(run), .. }]
            if path == "date_naissance" && pattern == "dd/MM/yyyy Q" && run == "Q"),
        "{errors:?}"
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("date_naissance") && message.contains("dd/MM/yyyy Q"),
        "{message}"
    );
    let errors = format_errors("date_naissance", "dd/MM/yyyy HH:mm");
    assert!(
        matches!(
            &errors[..],
            [ConfigError::InvalidPattern {
                error: PatternError::TimeInDate(_),
                ..
            }]
        ),
        "{errors:?}"
    );
    assert!(errors[0].to_string().contains("date_naissance"));
    assert!(matches!(
        &format_errors("inscription", "'à HH")[..],
        [ConfigError::InvalidPattern {
            error: PatternError::UnclosedQuote,
            ..
        }]
    ));
    with_presentations(&person_table(
        r#"[ { "path": "inscription", "format": "dd/MM/yyyy HH:mm" } ]"#,
        "[]",
    ))
    .check(&directory_model())
    .expect("a time in the format of a date-time");
}

/// A model where `Employe` inherits from `Personne`.
const INHERITED_MODEL: &str = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p", "classes": [
    { "name": "Racine", "references": [ { "name": "annuaire", "type": "Annuaire" } ] },
    { "name": "Annuaire", "references": [
        { "name": "personnes", "type": "Personne", "upperBound": -1 } ] },
    { "name": "Personne", "attributes": [ { "name": "nom", "type": "String" } ] },
    { "name": "Employe", "superClass": "Personne",
      "attributes": [ { "name": "matricule", "type": "Int" } ] } ] } ] }"#;

/// `VALID` with the given label groups.
fn with_labels(labels: &str) -> AppFile {
    let text = VALID.replace(
        r#""languages": ["fr"]"#,
        &format!(r#""languages": ["fr", "en"], "labels": {labels}"#),
    );
    AppFile::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// A group of labels of the language `lang`, with the entries `(key, text)`.
fn group(lang: &str, entries: &[(&str, &str)]) -> String {
    let entries: Vec<String> = entries
        .iter()
        .map(|(key, text)| format!(r#"{{ "key": "{key}", "text": "{text}" }}"#))
        .collect();
    format!(
        r#"{{ "lang": "{lang}", "entries": [ {} ] }}"#,
        entries.join(", ")
    )
}

/// Scenario "Libellés de l'annuaire" (`labels`).
#[test]
fn labels_of_the_directory() {
    let checked = AppFile::parse(&example("annuaire/app.json"))
        .expect("parse")
        .check(&directory_model())
        .expect("check");
    assert_eq!(checked.languages, ["fr", "en"]);
    assert_eq!(checked.labels.text("fr", "Personne.nom"), Some("Nom"));
    assert_eq!(checked.labels.text("en", "Personne.nom"), Some("Last name"));
    assert_eq!(checked.labels.text("de", "Personne.nom"), None);
}

/// Scenario "Libellé d'un littéral" (`labels`): the three forms of keys.
#[test]
fn forms_of_the_keys() {
    let checked = with_labels(&format!(
        "[ {} ]",
        group(
            "fr",
            &[
                ("Personne", "Personne"),
                ("Personne.nom", "Nom"),
                ("SituationFamiliale.MARIE", "Marié(e)")
            ]
        )
    ))
    .check(&directory_model())
    .expect("check");
    assert!(checked.warnings.is_empty(), "{:?}", checked.warnings);
    assert_eq!(
        checked.labels.text("fr", "SituationFamiliale.MARIE"),
        Some("Marié(e)")
    );
    // An inherited attribute, under the key of the subclass.
    let model = compile(INHERITED_MODEL);
    let checked = with_labels(&format!("[ {} ]", group("fr", &[("Employe.nom", "Nom")])))
        .check(&model)
        .expect("check");
    assert!(checked.warnings.is_empty(), "{:?}", checked.warnings);
}

/// Scenario "Faute de frappe" (`labels`): a warning, not an error; and other keys that name
/// nothing.
#[test]
fn key_that_names_nothing() {
    let checked = with_labels(&format!(
        "[ {} ]",
        group(
            "fr",
            &[
                ("Personne.nmo", "Nom"),
                ("Personn", "x"),
                ("SituationFamiliale.MARIEE", "x"),
                ("SituationFamiliale", "x"),
                ("Annuaire.personnes", "x")
            ]
        )
    ))
    .check(&directory_model())
    .expect("opens");
    let keys: Vec<&str> = checked
        .warnings
        .iter()
        .map(|w| match w {
            ConfigWarning::UnknownLabelKey { lang, key } => {
                assert_eq!(lang, "fr");
                key.as_str()
            }
        })
        .collect();
    assert_eq!(
        keys,
        [
            "Personne.nmo",
            "Personn",
            "SituationFamiliale.MARIEE",
            "SituationFamiliale",
            "Annuaire.personnes"
        ]
    );
    let message = checked.warnings[0].to_string();
    assert!(
        message.contains("Personne.nmo") && message.contains("fr"),
        "{message}"
    );
}

/// Scenario "Doublon" (`labels`).
#[test]
fn duplicate_key() {
    let errors = check_errors(
        &with_labels(&format!(
            "[ {} ]",
            group("fr", &[("Personne.nom", "Nom"), ("Personne.nom", "Nom 2")])
        )),
        &directory_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::DuplicateLabel {
            lang: "fr".into(),
            key: "Personne.nom".into()
        }]
    );
    let message = errors[0].to_string();
    assert!(
        message.contains("Personne.nom") && message.contains("fr"),
        "{message}"
    );
    // The same key in two languages is no duplicate.
    with_labels(&format!(
        "[ {}, {} ]",
        group("fr", &[("Personne.nom", "Nom")]),
        group("en", &[("Personne.nom", "Name")])
    ))
    .check(&directory_model())
    .expect("two languages");
}

/// Scenarios "Langue absente de la liste" and "Deux groupes pour une langue" (`labels`).
#[test]
fn undeclared_language() {
    let errors = check_errors(
        &with_labels(&format!("[ {} ]", group("de", &[("Personne.nom", "Name")]))),
        &directory_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::UndeclaredLabelLanguage { lang: "de".into() }]
    );
    assert!(errors[0].to_string().contains("`de`"));
    let errors = check_errors(
        &with_labels(&format!(
            "[ {}, {} ]",
            group("fr", &[("Personne.nom", "Nom")]),
            // The same key: the group alone is reported, not each of its keys again.
            group(
                "fr",
                &[("Personne.nom", "Nom"), ("Personne.prenom", "Prénom")]
            )
        )),
        &directory_model(),
    );
    assert_eq!(
        errors,
        [ConfigError::DuplicateLabelGroup { lang: "fr".into() }]
    );
    assert!(errors[0].to_string().contains("`fr`"));
}

/// Errors of several sections are all reported.
#[test]
fn errors_of_several_sections() {
    let text = with_languages("[]").replace(
        r#""languages""#,
        r#""presentations": [ { "class": "Persone" } ], "languages""#,
    );
    let errors = check_errors(&AppFile::parse(&text).expect("parse"), &directory_model());
    assert_eq!(
        errors,
        [
            ConfigError::NoLanguage,
            ConfigError::UnknownPresentationClass {
                class: "Persone".into()
            }
        ]
    );
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
    let error = AppFile::parse(r#"{ "formatVersion": "0.4", "id": "#).expect_err("truncated");
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
    let error = AppFile::parse(&VALID.replace("0.4", "0.3")).expect_err("0.3");
    assert_eq!(
        error,
        ConfigError::Version {
            found: "0.3".into()
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("`0.3`") && message.contains("`0.4`"),
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
    { "name": "Forme", "abstract": true, "references": [
      { "name": "sommets", "type": "Point", "upperBound": -1 } ] },
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
        &app_with("Racine", "/a/items", &[("s", "/forme/points")]),
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
    let checked = app_with("Racine", "/a/items", &[("s", "/forme")])
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
    app_with(
        "Racine",
        "/a/items",
        &[("a", "/a/items"), ("b", "/b/items")],
    )
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
