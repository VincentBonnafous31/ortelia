//! Tests of the builder (specification `data-loading`: building in file order, conversion of
//! text values, values already typed, errors gathered, diagnostics).

mod common;

use jiff::Timestamp;
use jiff::civil::Date;
use ortelia_model::builder::{
    Builder, DataPath, DiagnosticKind, LoadError, LoadErrorKind, LoadFailure, MAX_REPORTED,
    PathSegment,
};
use ortelia_model::fragment::{Fragment, Shape};

use ortelia_model::value::Value;

use common::{Names, directory_model, network_model};

/// A builder whose root `Reseau` is begun.
fn new_builder() -> (Builder, Names) {
    let model = network_model();
    let names = Names::new(&model);
    let root = names.class("Reseau");
    let mut builder = Builder::new(model, Shape::root(root), DataPath::default());
    assert!(builder.begin_head(root));
    (builder, names)
}

/// Begins a new station in `stations`, and returns with the station as the current object.
fn begin_station(builder: &mut Builder, n: &Names) {
    builder.begin_reference(n.reference("Reseau", "stations"));
    assert!(builder.begin_object(n.class("Station")));
}

/// Begins a relevé under a new thermometer of a new station.
fn begin_releve(builder: &mut Builder, n: &Names) {
    begin_station(builder, n);
    builder.begin_reference(n.reference("Station", "equipements"));
    assert!(builder.begin_object(n.class("Thermometre")));
    builder.begin_reference(n.reference("Capteur", "mesures"));
    assert!(builder.begin_object(n.class("Releve")));
}

/// The fragment of a builder that recorded no error.
fn fragment(builder: Builder) -> Fragment {
    match builder.finish() {
        Ok(loaded) => loaded.fragment,
        Err(failure) => panic!("{:#?}", failure.errors),
    }
}

/// The failure of a builder that recorded at least one error.
fn failure(builder: Builder) -> LoadFailure {
    builder.finish().expect_err("an error")
}

/// The only error of a builder.
fn only_error(builder: Builder) -> LoadError {
    let failure = failure(builder);
    assert_eq!(failure.error_count, 1, "{:#?}", failure.errors);
    failure.errors.into_iter().next().expect("error")
}

/// Reads an attribute of the current object of a finished builder: the only object of its
/// class (the tests below create one of each).
fn read(fragment: &Fragment, n: &Names, class: &str, attr: (&str, &str)) -> String {
    let store = fragment.store();
    let wanted = n.class(class);
    let obj = fragment
        .heads()
        .iter()
        .flat_map(|&head| store.descendants(head))
        .find(|&obj| store.class_of(obj) == wanted)
        .unwrap_or_else(|| panic!("no {class}"));
    format!("{:?}", store.get(obj, n.attr(attr.0, attr.1)))
}

/// Scenario "Arbre construit dans l'ordre des appels".
#[test]
fn tree_built_in_call_order() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    assert!(builder.begin_object(n.class("Thermometre")));
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    let fragment = fragment(builder);
    let store = fragment.store();

    // The root of the network is the only head; the fragment's store has no root of its own.
    assert_eq!(fragment.shape(), Shape::root(n.class("Reseau")));
    assert_eq!(fragment.heads().len(), 1);
    assert_eq!(store.root(), None);
    let root = fragment.heads()[0];
    assert_eq!(store.class_of(root), n.class("Reseau"));
    assert_eq!(store.container(root), None);
    let stations = store.children(root, n.reference("Reseau", "stations"));
    assert_eq!(stations.len(), 1);
    assert_eq!(store.class_of(stations[0]), n.class("Station"));
    let equipments = store.children(stations[0], n.reference("Station", "equipements"));
    assert_eq!(equipments.len(), 1);
    assert_eq!(store.class_of(equipments[0]), n.class("Thermometre"));
    assert_eq!(
        store.container(equipments[0]).map(|c| c.0),
        Some(stations[0])
    );
}

/// A builder for the branch `/annuaire/personnes` of the directory, and its names.
fn persons_builder() -> (Builder, Names) {
    let model = directory_model();
    let n = Names::new(&model);
    let shape = Shape::of(&model, n.reference("Annuaire", "personnes"));
    let base = DataPath(vec![
        PathSegment::Name("annuaire".into()),
        PathSegment::Name("personnes".into()),
    ]);
    (Builder::new(model, shape, base), n)
}

/// Scenario "Fragment d'une liste".
#[test]
fn fragment_of_a_list() {
    let (mut builder, n) = persons_builder();
    let nom = n.attr("Personne", "nom");
    for name in ["A", "B", "C"] {
        assert!(builder.begin_head(n.class("Personne")));
        builder.set_text(nom, name);
        builder.end_object();
    }
    let fragment = fragment(builder);
    let store = fragment.store();
    assert_eq!(fragment.heads().len(), 3);
    let names: Vec<String> = fragment
        .heads()
        .iter()
        .map(|&head| {
            assert_eq!(store.class_of(head), n.class("Personne"));
            assert_eq!(store.container(head), None);
            format!("{:?}", store.get(head, nom))
        })
        .collect();
    assert_eq!(names, [r#"Str("A")"#, r#"Str("B")"#, r#"Str("C")"#]);
}

/// The path of an error starts with the path of the branch, then the position of the head
/// (scenario "Chemin depuis la racine de l'application", at the level of the builder).
#[test]
fn error_path_starts_with_the_branch() {
    let (mut builder, n) = persons_builder();
    for _ in 0..3 {
        assert!(builder.begin_head(n.class("Personne")));
        builder.set_text(n.attr("Personne", "nom"), "X");
        builder.end_object();
    }
    assert!(builder.begin_head(n.class("Personne")));
    builder.set_text(n.attr("Personne", "nombre_enfants"), "deux");
    let error = only_error(builder);
    assert_eq!(
        error.path.to_string(),
        "/annuaire/personnes/3/nombre_enfants"
    );
}

/// Scenario "Deux objets pour une branche simple".
#[test]
fn two_heads_for_a_single_branch() {
    let model = directory_model();
    let n = Names::new(&model);
    let shape = Shape::of(&model, n.reference("Racine", "annuaire"));
    let base = DataPath(vec![PathSegment::Name("annuaire".into())]);
    let mut builder = Builder::new(model, shape, base);
    assert!(builder.begin_head(n.class("Annuaire")));
    builder.end_object();
    assert!(!builder.begin_head(n.class("Annuaire")), "second head");
    let error = only_error(builder);
    assert_eq!(
        error.kind,
        LoadErrorKind::ReferenceAlreadySet {
            reference: "annuaire".into()
        }
    );
    assert_eq!(error.path.to_string(), "/annuaire");
    assert!(
        error.to_string().contains("more than one object"),
        "{error}"
    );
}

/// A head must conform to the shape, and be concrete.
#[test]
fn head_of_the_wrong_class() {
    let (mut builder, n) = persons_builder();
    assert!(!builder.begin_head(n.class("Annuaire")));
    let error = only_error(builder);
    assert_eq!(
        error.kind,
        LoadErrorKind::NonConformingClass {
            class: "Annuaire".into(),
            expected: "Personne".into()
        }
    );
    assert_eq!(error.path.to_string(), "/annuaire/personnes/0");
}

/// Scenario "Conversions réussies".
#[test]
fn text_conversions() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.set_text(n.attr("Station", "altitude"), "42");
    builder.set_text(n.attr("Station", "latitude"), "12.5");
    builder.set_text(n.attr("Station", "active"), "true");
    builder.set_text(n.attr("Station", "mise_en_service"), "1985-03-12");
    builder.set_text(n.attr("Station", "nom"), "Brest");
    builder.end_object();
    builder.end_reference();
    builder.begin_reference(n.reference("Reseau", "zones"));
    assert!(builder.begin_object(n.class("ZoneAlerte")));
    builder.set_text(n.attr("ZoneAlerte", "niveau"), "ORANGE");
    builder.set_text(n.attr("ZoneAlerte", "debut"), "2026-03-12T10:15:00.123456Z");
    builder.end_object();
    builder.end_reference();
    let store = fragment(builder);

    let date: Date = "1985-03-12".parse().expect("date");
    let instant: Timestamp = "2026-03-12T10:15:00.123456Z".parse().expect("instant");
    let station = |attr| read(&store, &n, "Station", ("Station", attr));
    assert_eq!(station("altitude"), format!("{:?}", Value::Int(42)));
    assert_eq!(station("latitude"), format!("{:?}", Value::Float(12.5)));
    assert_eq!(station("active"), format!("{:?}", Value::Bool(true)));
    assert_eq!(
        station("mise_en_service"),
        format!("{:?}", Value::Date(date))
    );
    assert_eq!(station("nom"), format!("{:?}", Value::Str("Brest")));
    let zone = |attr| read(&store, &n, "ZoneAlerte", ("ZoneAlerte", attr));
    // ORANGE is the third literal of NiveauAlerte.
    assert_eq!(zone("niveau"), format!("{:?}", Value::Enum(2)));
    assert_eq!(zone("debut"), format!("{:?}", Value::DateTime(instant)));
}

/// Asserts that a text is refused for an attribute of a station, with a type error.
fn assert_text_refused(attr: &str, text: &str) {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.set_text(n.attr("Station", attr), text);
    let error = only_error(builder);
    assert!(
        matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
        "{error}"
    );
    assert_eq!(error.path.to_string(), format!("/stations/0/{attr}"));
    assert_eq!(error.value.as_deref(), Some(text));
}

/// Scenario "Texte non convertible", and other texts that do not convert.
#[test]
fn texts_that_do_not_convert() {
    assert_text_refused("mise_en_service", "1985-13-45");
    assert_text_refused("altitude", "haute");
    assert_text_refused("altitude", "98.5");
    assert_text_refused("active", "oui");
    assert_text_refused("categorie", "INCONNUE");
}

/// Scenarios "Date-heure trop précise" and "Date-heure sans décalage".
#[test]
fn datetimes_that_do_not_convert() {
    for text in ["2026-03-12T10:15:00.123456789Z", "2026-03-12T10:15:00"] {
        let (mut builder, n) = new_builder();
        begin_releve(&mut builder, &n);
        builder.set_text(n.attr("Mesure", "instant"), text);
        let error = only_error(builder);
        assert!(
            matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
            "{error}"
        );
        assert_eq!(error.value.as_deref(), Some(text));
    }
}

/// Scenario "Entier pour un décimal".
#[test]
fn integer_for_a_float() {
    let (mut builder, n) = new_builder();
    begin_releve(&mut builder, &n);
    builder.set_int(n.attr("Releve", "valeur"), 5);
    for _ in 0..3 {
        builder.end_object();
        builder.end_reference();
    }
    let store = fragment(builder);
    assert_eq!(
        read(&store, &n, "Releve", ("Releve", "valeur")),
        format!("{:?}", Value::Float(5.0))
    );
}

/// Scenarios "Décimal pour un entier" and "Nombre pour un texte", and other mismatches: each
/// value is refused, with an error, and the following ones are tried all the same.
#[test]
fn typed_values_of_the_wrong_type() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    let altitude = n.attr("Station", "altitude");
    let nom = n.attr("Station", "nom");
    builder.set_float(altitude, 98.5);
    builder.set_int(nom, 12);
    builder.set_float(nom, 12.0);
    builder.set_bool(nom, true);
    builder.set_bool(altitude, true);
    let failure = failure(builder);
    assert_eq!(failure.error_count, 5);
    assert_eq!(
        failure.errors[0].kind,
        LoadErrorKind::WrongValueType {
            attribute: "altitude".into(),
            expected: "Int".into()
        }
    );
    let values: Vec<Option<&str>> = failure.errors.iter().map(|e| e.value.as_deref()).collect();
    assert_eq!(
        values,
        [
            Some("98.5"),
            Some("12"),
            Some("12"),
            Some("true"),
            Some("true")
        ]
    );
}

/// Scenario "Valeur nulle".
#[test]
fn null_value_leaves_the_attribute_unset() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    assert!(builder.begin_object(n.class("Camera")));
    builder.set_text(n.attr("Equipement", "modele"), "X");
    builder.set_null(n.attr("Equipement", "modele"));
    for _ in 0..2 {
        builder.end_object();
        builder.end_reference();
    }
    let store = fragment(builder);
    assert_eq!(
        read(&store, &n, "Camera", ("Equipement", "modele")),
        format!("{:?}", Value::Unset)
    );
}

/// The path of an error: reference names and positions in lists. An object that cannot be
/// created counts in the path of the following ones.
#[test]
fn error_path() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    assert!(builder.begin_object(n.class("Camera")));
    builder.end_object();
    assert!(builder.begin_object(n.class("Thermometre")));
    builder.begin_reference(n.reference("Capteur", "mesures"));
    for _ in 0..3 {
        assert!(builder.begin_object(n.class("Releve")));
        builder.end_object();
    }
    // The fourth relevé is the current position when its value is wrong.
    assert!(builder.begin_object(n.class("Releve")));
    builder.set_text(n.attr("Releve", "valeur"), "x");
    builder.end_object();
    assert!(!builder.begin_object(n.class("Camera")));
    assert!(builder.begin_object(n.class("Releve")));
    builder.set_text(n.attr("Releve", "valeur"), "y");
    let failure = failure(builder);
    let errors: Vec<String> = failure.errors.iter().map(|e| e.to_string()).collect();
    assert_eq!(
        errors,
        [
            "/stations/0/equipements/1/mesures/3/valeur: attribute `valeur` expects a value of type `Float`",
            "/stations/0/equipements/1/mesures/4: class `Camera` is not a `Mesure`",
            "/stations/0/equipements/1/mesures/5/valeur: attribute `valeur` expects a value of type `Float`",
        ]
    );
    assert_eq!(
        failure.errors[1].kind,
        LoadErrorKind::NonConformingClass {
            class: "Camera".into(),
            expected: "Mesure".into()
        }
    );
}

/// Single references have no position in the path; the root is `/`.
#[test]
fn single_reference_and_root_paths() {
    let (mut builder, n) = new_builder();
    builder.report(LoadErrorKind::UnknownKey { key: "k".into() });
    builder.begin_reference(n.reference("Reseau", "zones"));
    assert!(builder.begin_object(n.class("ZoneAlerte")));
    builder.begin_reference(n.reference("ZoneAlerte", "emprise"));
    assert!(builder.begin_object(n.class("Cercle")));
    builder.set_text(n.attr("Cercle", "rayon_km"), "loin");
    builder.end_object();
    assert!(!builder.begin_object(n.class("Polygone")), "occupied");
    let failure = failure(builder);
    let paths: Vec<String> = failure.errors.iter().map(|e| e.path.to_string()).collect();
    assert_eq!(
        paths,
        ["/", "/zones/0/emprise/rayon_km", "/zones/0/emprise"]
    );
    assert_eq!(
        failure.errors[2].kind,
        LoadErrorKind::ReferenceAlreadySet {
            reference: "emprise".into()
        }
    );
}

/// The first 100 errors are kept, in order, and all are counted.
#[test]
fn cap_and_total_of_errors() {
    let (mut builder, n) = persons_builder();
    for _ in 0..250 {
        assert!(builder.begin_head(n.class("Personne")));
        builder.set_text(n.attr("Personne", "nom"), "X");
        builder.set_text(n.attr("Personne", "taille"), "grand");
        builder.end_object();
    }
    let failure = failure(builder);
    assert_eq!(failure.error_count, 250);
    assert_eq!(failure.errors.len(), MAX_REPORTED);
    for (i, error) in failure.errors.iter().enumerate() {
        assert_eq!(
            error.path.to_string(),
            format!("/annuaire/personnes/{i}/taille")
        );
    }
    assert_eq!(failure.diagnostic_count, 0);
    assert_eq!(
        failure.to_string(),
        "/annuaire/personnes/0/taille: attribute `taille` expects a value of type `Float` \
         (and 249 more errors)"
    );
}

/// A required attribute without a value gives a diagnostic at the end of its object, also when
/// it is inherited, when it is null, or when its value was refused; the load still succeeds
/// without other errors, and the diagnostics are capped like the errors.
#[test]
fn diagnostics_of_required_attributes() {
    let (mut builder, n) = new_builder();
    begin_releve(&mut builder, &n);
    // `instant` is declared by `Mesure`, superclass of `Releve`.
    builder.end_object();
    assert!(builder.begin_object(n.class("Releve")));
    builder.set_text(n.attr("Mesure", "instant"), "2026-01-01T00:00:00Z");
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    builder.end_reference();
    let loaded = builder.finish().expect("diagnostics only");
    let diagnostics: Vec<String> = loaded.diagnostics.iter().map(|d| d.to_string()).collect();
    assert_eq!(
        diagnostics,
        [
            "/stations/0/equipements/0/mesures/0: required attribute `instant` has no value",
            "/stations/0/equipements/0: required attribute `numero_serie` has no value",
            "/stations/0: required attribute `code` has no value",
            "/stations/0: required attribute `nom` has no value",
        ]
    );
    assert_eq!(loaded.diagnostic_count, 4);

    let (mut builder, n) = persons_builder();
    let nom = n.attr("Personne", "nom");
    for i in 0..150 {
        assert!(builder.begin_head(n.class("Personne")));
        match i % 3 {
            0 => {}
            1 => builder.set_null(nom),
            _ => builder.set_int(nom, 3),
        }
        builder.end_object();
    }
    let failure = failure(builder);
    // The 50 values refused are errors; the 150 missing names are diagnostics.
    assert_eq!(failure.error_count, 50);
    assert_eq!(failure.diagnostic_count, 150);
    assert_eq!(failure.diagnostics.len(), MAX_REPORTED);
    assert_eq!(
        failure.diagnostics[1].kind,
        DiagnosticKind::MissingRequired {
            attribute: "nom".into()
        }
    );
    assert_eq!(
        failure.diagnostics[1].path.to_string(),
        "/annuaire/personnes/1"
    );
}

/// Scenarios "Les deux formes d'une erreur" and "Valeur longue", at the level of the builder.
#[test]
fn the_two_forms_of_an_error() {
    let (mut builder, n) = persons_builder();
    assert!(builder.begin_head(n.class("Personne")));
    builder.set_text(n.attr("Personne", "nombre_enfants"), "deux");
    builder.set_text(n.attr("Personne", "situation"), &"X".repeat(39));
    builder.set_text(n.attr("Personne", "date_naissance"), &"€".repeat(1000));
    let failure = failure(builder);
    let error = &failure.errors[0];
    assert_eq!(
        error.to_string(),
        "/annuaire/personnes/0/nombre_enfants: attribute `nombre_enfants` expects a value of type `Int`"
    );
    assert_eq!(
        error.user_message(),
        "/annuaire/personnes/0/nombre_enfants: attribute `nombre_enfants` expects a value of type `Int`, found `deux`"
    );
    // 39 characters are kept whole; 1000 are cut at 40 characters (3 bytes each), then `…`.
    assert_eq!(failure.errors[1].value, Some("X".repeat(39)));
    assert_eq!(
        failure.errors[2].value,
        Some(format!("{}…", "€".repeat(40)))
    );
    assert!(failure.errors.iter().all(|e| !e.to_string().contains('€')));
    // Errors found by the reader have no value. An unknown key comes from the file: only the
    // message for the user gives it, shortened; an error without text of the file has one form.
    let (mut builder, _) = persons_builder();
    builder.report(LoadErrorKind::UnknownKey {
        key: "k".repeat(50),
    });
    builder.report(LoadErrorKind::WrongStructure {
        expected: "an object".into(),
    });
    let reader_errors = builder.finish().expect_err("two errors");
    let (key, structure) = (&reader_errors.errors[0], &reader_errors.errors[1]);
    assert_eq!(key.value, None);
    assert_eq!(key.to_string(), "/annuaire/personnes: unknown key");
    assert_eq!(
        key.user_message(),
        format!(
            "/annuaire/personnes: unknown key, found `{}…`",
            "k".repeat(40)
        )
    );
    assert_eq!(structure.user_message(), structure.to_string());
}
