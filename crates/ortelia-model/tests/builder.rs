//! Tests of the builder (specification `data-loading`: building in file order, conversion of
//! text values, values already typed).

mod common;

use jiff::Timestamp;
use jiff::civil::Date;
use ortelia_model::builder::{Builder, LoadErrorKind};
use ortelia_model::store::Store;
use ortelia_model::value::Value;

use common::{Names, network_model};

/// A builder whose root `Reseau` is begun.
fn new_builder() -> (Builder, Names) {
    let model = network_model();
    let names = Names {
        model: model.clone(),
    };
    let mut builder = Builder::new(model);
    builder.begin_root(names.class("Reseau")).expect("root");
    (builder, names)
}

/// Begins a new station in `stations`, and returns with the station as the current object.
fn begin_station(builder: &mut Builder, n: &Names) {
    builder.begin_reference(n.reference("Reseau", "stations"));
    builder.begin_object(n.class("Station")).expect("station");
}

/// Begins a relevé under a new thermometer of a new station.
fn begin_releve(builder: &mut Builder, n: &Names) {
    begin_station(builder, n);
    builder.begin_reference(n.reference("Station", "equipements"));
    builder
        .begin_object(n.class("Thermometre"))
        .expect("thermometer");
    builder.begin_reference(n.reference("Capteur", "mesures"));
    builder.begin_object(n.class("Releve")).expect("releve");
}

/// Reads an attribute of the current object of a finished builder: the only object of its
/// class (the tests below create one of each).
fn read(store: &Store, n: &Names, class: &str, attr: (&str, &str)) -> String {
    // Walk the tree from the root, depth first, until an object of the class.
    let model = store.model();
    let wanted = n.class(class);
    let mut stack = vec![store.root().expect("root")];
    while let Some(obj) = stack.pop() {
        if store.class_of(obj) == wanted {
            return format!("{:?}", store.get(obj, n.attr(attr.0, attr.1)));
        }
        for reference in model.class(store.class_of(obj)).references() {
            stack.extend(store.children(obj, reference.id()));
        }
    }
    panic!("no {class}");
}

/// Scenario "Arbre construit dans l'ordre des appels".
#[test]
fn tree_built_in_call_order() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    builder
        .begin_object(n.class("Thermometre"))
        .expect("thermometer");
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    builder.end_reference();
    let store = builder.finish();

    let root = store.root().expect("root");
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

/// Scenario "Conversions réussies".
#[test]
fn text_conversions() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder
        .set_text(n.attr("Station", "altitude"), "42")
        .expect("Int");
    builder
        .set_text(n.attr("Station", "latitude"), "12.5")
        .expect("Float");
    builder
        .set_text(n.attr("Station", "active"), "true")
        .expect("Boolean");
    builder
        .set_text(n.attr("Station", "mise_en_service"), "1985-03-12")
        .expect("Date");
    builder
        .set_text(n.attr("Station", "nom"), "Brest")
        .expect("String");
    builder.end_object();
    builder.end_reference();
    builder.begin_reference(n.reference("Reseau", "zones"));
    builder.begin_object(n.class("ZoneAlerte")).expect("zone");
    builder
        .set_text(n.attr("ZoneAlerte", "niveau"), "ORANGE")
        .expect("enum");
    builder
        .set_text(n.attr("ZoneAlerte", "debut"), "2026-03-12T10:15:00.123456Z")
        .expect("DateTime");
    builder.end_object();
    builder.end_reference();
    let store = builder.finish();

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
    let error = builder
        .set_text(n.attr("Station", attr), text)
        .expect_err(text);
    assert!(
        matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
        "{error}"
    );
    assert_eq!(error.path.to_string(), format!("/stations/0/{attr}"));
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
        let error = builder
            .set_text(n.attr("Mesure", "instant"), text)
            .expect_err(text);
        assert!(
            matches!(error.kind, LoadErrorKind::WrongValueType { .. }),
            "{error}"
        );
    }
}

/// Scenario "Entier pour un décimal".
#[test]
fn integer_for_a_float() {
    let (mut builder, n) = new_builder();
    begin_releve(&mut builder, &n);
    builder
        .set_int(n.attr("Releve", "valeur"), 5)
        .expect("int for float");
    for _ in 0..3 {
        builder.end_object();
        builder.end_reference();
    }
    let store = builder.finish();
    assert_eq!(
        read(&store, &n, "Releve", ("Releve", "valeur")),
        format!("{:?}", Value::Float(5.0))
    );
}

/// Scenarios "Décimal pour un entier" and "Nombre pour un texte", and other mismatches.
#[test]
fn typed_values_of_the_wrong_type() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    let altitude = n.attr("Station", "altitude");
    let error = builder
        .set_float(altitude, 98.5)
        .expect_err("float for int");
    assert_eq!(
        error.kind,
        LoadErrorKind::WrongValueType {
            attribute: "altitude".into(),
            expected: "Int".into()
        }
    );
    let nom = n.attr("Station", "nom");
    assert!(builder.set_int(nom, 12).is_err());
    assert!(builder.set_float(nom, 12.0).is_err());
    assert!(builder.set_bool(nom, true).is_err());
    assert!(builder.set_bool(altitude, true).is_err());
}

/// Scenario "Valeur nulle".
#[test]
fn null_value_leaves_the_attribute_unset() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    builder.begin_object(n.class("Camera")).expect("camera");
    builder
        .set_text(n.attr("Equipement", "modele"), "X")
        .expect("modele");
    builder
        .set_null(n.attr("Equipement", "modele"))
        .expect("null");
    for _ in 0..2 {
        builder.end_object();
        builder.end_reference();
    }
    let store = builder.finish();
    assert_eq!(
        read(&store, &n, "Camera", ("Equipement", "modele")),
        format!("{:?}", Value::Unset)
    );
}

/// The path of an error: reference names and positions in lists.
#[test]
fn error_path() {
    let (mut builder, n) = new_builder();
    begin_station(&mut builder, &n);
    builder.begin_reference(n.reference("Station", "equipements"));
    builder.begin_object(n.class("Camera")).expect("camera");
    builder.end_object();
    builder
        .begin_object(n.class("Thermometre"))
        .expect("thermometer");
    builder.begin_reference(n.reference("Capteur", "mesures"));
    for _ in 0..3 {
        builder.begin_object(n.class("Releve")).expect("releve");
        builder.end_object();
    }
    // The third relevé is the current position when its value is wrong.
    builder.begin_object(n.class("Releve")).expect("releve");
    let error = builder
        .set_text(n.attr("Releve", "valeur"), "x")
        .expect_err("wrong type");
    assert_eq!(
        error.path.to_string(),
        "/stations/0/equipements/1/mesures/3/valeur"
    );
    assert_eq!(
        error.to_string(),
        "/stations/0/equipements/1/mesures/3/valeur: attribute `valeur` expects a value of type `Float`"
    );
    // An object that cannot be created counts in the path.
    builder.end_object();
    let error = builder
        .begin_object(n.class("Camera"))
        .expect_err("non conforming");
    assert_eq!(
        error.path.to_string(),
        "/stations/0/equipements/1/mesures/4"
    );
    assert_eq!(
        error.kind,
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
    assert_eq!(
        builder
            .error(LoadErrorKind::UnknownKey { key: "k".into() })
            .path
            .to_string(),
        "/"
    );
    builder.begin_reference(n.reference("Reseau", "zones"));
    builder.begin_object(n.class("ZoneAlerte")).expect("zone");
    builder.begin_reference(n.reference("ZoneAlerte", "emprise"));
    builder.begin_object(n.class("Cercle")).expect("circle");
    let error = builder
        .set_text(n.attr("Cercle", "rayon_km"), "loin")
        .expect_err("wrong type");
    assert_eq!(error.path.to_string(), "/zones/0/emprise/rayon_km");
    builder.end_object();
    let error = builder
        .begin_object(n.class("Polygone"))
        .expect_err("occupied");
    assert_eq!(
        error.kind,
        LoadErrorKind::ReferenceAlreadySet {
            reference: "emprise".into()
        }
    );
}
