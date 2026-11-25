//! Tests of the object store (specification `object-store`).

mod common;

use jiff::Timestamp;
use jiff::civil::Date;
use std::sync::Arc;

use ortelia_model::metamodel::{Model, ModelFile};
use ortelia_model::store::{ObjectId, Position, Store, StoreError};
use ortelia_model::value::Value;

use common::{Names, network_model};

/// A store with a root `Reseau`.
fn new_store() -> (Store, Names, ObjectId) {
    let model = network_model();
    let names = Names {
        model: model.clone(),
    };
    let mut store = Store::new(model);
    let root = store.create_root(names.class("Reseau")).expect("root");
    (store, names, root)
}

fn add(
    store: &mut Store,
    n: &Names,
    parent: ObjectId,
    reference: (&str, &str),
    class: &str,
) -> ObjectId {
    store
        .create_child(
            parent,
            n.reference(reference.0, reference.1),
            Position::End,
            n.class(class),
        )
        .expect("create child")
}

fn station(store: &mut Store, n: &Names, root: ObjectId, name: &str) -> ObjectId {
    let station = add(store, n, root, ("Reseau", "stations"), "Station");
    store
        .set(station, n.attr("Station", "nom"), Value::Str(name))
        .expect("nom");
    station
}

fn names_of(store: &Store, n: &Names, ids: &[ObjectId]) -> Vec<String> {
    ids.iter()
        .map(|&id| match store.get(id, n.attr("Station", "nom")) {
            Value::Str(text) => text.to_owned(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// Scenario "Identifiant d'un objet supprimé".
#[test]
fn deleted_id_is_never_reused() {
    let (mut store, n, root) = new_store();
    let first = station(&mut store, &n, root, "Brest");
    store.delete(first);
    let second = station(&mut store, &n, root, "Quimper");
    assert_ne!(first, second);
    assert!(!store.is_alive(first));
    assert!(store.is_alive(second));
}

/// Scenario "Enfant ajouté en fin de liste".
#[test]
fn children_are_appended_in_order() {
    let (mut store, n, root) = new_store();
    let ids: Vec<ObjectId> = ["A", "B", "C"]
        .iter()
        .map(|name| station(&mut store, &n, root, name))
        .collect();
    assert_eq!(store.children(root, n.reference("Reseau", "stations")), ids);
}

/// Scenario "Enfant inséré à une position".
#[test]
fn child_inserted_at_a_position() {
    let (mut store, n, root) = new_store();
    let stations = n.reference("Reseau", "stations");
    station(&mut store, &n, root, "A");
    station(&mut store, &n, root, "C");
    let b = store
        .create_child(root, stations, Position::At(1), n.class("Station"))
        .expect("insert");
    store
        .set(b, n.attr("Station", "nom"), Value::Str("B"))
        .expect("nom");
    assert_eq!(
        names_of(&store, &n, store.children(root, stations)),
        ["A", "B", "C"]
    );
}

#[test]
fn position_past_the_end_is_refused() {
    let (mut store, n, root) = new_store();
    let stations = n.reference("Reseau", "stations");
    station(&mut store, &n, root, "A");
    let error = store
        .create_child(root, stations, Position::At(2), n.class("Station"))
        .expect_err("past the end");
    assert_eq!(
        error,
        StoreError::PositionOutOfRange {
            position: 2,
            len: 1
        }
    );
    assert_eq!(store.children(root, stations).len(), 1);
}

/// Scenario "Classe abstraite refusée".
#[test]
fn abstract_class_is_refused() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let equipements = n.reference("Station", "equipements");
    let error = store
        .create_child(s, equipements, Position::End, n.class("Equipement"))
        .expect_err("abstract");
    assert_eq!(
        error,
        StoreError::AbstractClass {
            class: "Equipement".into()
        }
    );
    assert!(store.children(s, equipements).is_empty());
}

/// Scenario "Classe non conforme refusée".
#[test]
fn non_conforming_class_is_refused() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    add(&mut store, &n, s, ("Station", "equipements"), "Camera");
    let equipements = n.reference("Station", "equipements");
    let error = store
        .create_child(s, equipements, Position::End, n.class("Releve"))
        .expect_err("non conforming");
    assert_eq!(
        error,
        StoreError::NonConformingClass {
            class: "Releve".into(),
            expected: "Equipement".into()
        }
    );
    assert_eq!(store.children(s, equipements).len(), 1);
}

/// Scenario "Référence simple déjà occupée".
#[test]
fn occupied_single_reference_is_refused() {
    let (mut store, n, root) = new_store();
    let zone = add(&mut store, &n, root, ("Reseau", "zones"), "ZoneAlerte");
    let emprise = n.reference("ZoneAlerte", "emprise");
    let polygon = store
        .create_child(zone, emprise, Position::End, n.class("Polygone"))
        .expect("polygon");
    let error = store
        .create_child(zone, emprise, Position::End, n.class("Cercle"))
        .expect_err("occupied");
    assert_eq!(
        error,
        StoreError::ReferenceAlreadySet {
            reference: "emprise".into()
        }
    );
    assert_eq!(store.children(zone, emprise), [polygon]);
}

/// A single reference emptied by a deletion accepts a new child.
#[test]
fn single_reference_can_be_refilled() {
    let (mut store, n, root) = new_store();
    let zone = add(&mut store, &n, root, ("Reseau", "zones"), "ZoneAlerte");
    let emprise = n.reference("ZoneAlerte", "emprise");
    let polygon = store
        .create_child(zone, emprise, Position::End, n.class("Polygone"))
        .expect("polygon");
    store.delete(polygon);
    assert!(store.children(zone, emprise).is_empty());
    let circle = store
        .create_child(zone, emprise, Position::End, n.class("Cercle"))
        .expect("circle");
    assert_eq!(store.children(zone, emprise), [circle]);
}

/// Scenario "Attribut jamais renseigné".
#[test]
fn attribute_never_set_is_unset() {
    let (mut store, n, root) = new_store();
    let s = add(&mut store, &n, root, ("Reseau", "stations"), "Station");
    assert_eq!(store.get(s, n.attr("Station", "nom")), Value::Unset);
}

/// An integer that was never set is not an integer that is 0 (task 2.5).
#[test]
fn unset_integer_is_not_zero() {
    let (mut store, n, root) = new_store();
    let altitude = n.attr("Station", "altitude");
    let a = station(&mut store, &n, root, "A");
    let b = station(&mut store, &n, root, "B");
    store.set(b, altitude, Value::Int(0)).expect("altitude");
    assert_eq!(store.get(a, altitude), Value::Unset);
    assert_eq!(store.get(b, altitude), Value::Int(0));
}

/// Scenario "Attribut hérité dans une liste polymorphe".
#[test]
fn inherited_attribute_in_a_polymorphic_list() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let serial = n.attr("Equipement", "numero_serie");
    let thermometer = add(&mut store, &n, s, ("Station", "equipements"), "Thermometre");
    let camera = add(&mut store, &n, s, ("Station", "equipements"), "Camera");
    store
        .set(thermometer, serial, Value::Str("T-17"))
        .expect("serial");
    store
        .set(camera, serial, Value::Str("C-02"))
        .expect("serial");
    let read: Vec<Value<'_>> = store
        .children(s, n.reference("Station", "equipements"))
        .iter()
        .map(|&e| store.get(e, serial))
        .collect();
    assert_eq!(read, [Value::Str("T-17"), Value::Str("C-02")]);
}

/// Every type of value goes in and out unchanged.
#[test]
fn every_type_round_trips() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "Brest");
    let date: Date = "1985-03-12".parse().expect("date");
    let instant: Timestamp = "2026-03-12T10:15:00.123456Z".parse().expect("instant");
    let values = [
        ("latitude", Value::Float(48.4)),
        ("altitude", Value::Int(-12)),
        ("mise_en_service", Value::Date(date)),
        ("active", Value::Bool(true)),
        ("categorie", Value::Enum(2)),
        (
            "description",
            Value::Str("Station côtière, exposée à l’ouest"),
        ),
    ];
    for (name, value) in values {
        store.set(s, n.attr("Station", name), value).expect(name);
    }
    for (name, value) in values {
        assert_eq!(store.get(s, n.attr("Station", name)), value, "{name}");
    }
    let releve = new_releve(&mut store, &n, s);
    store
        .set(
            releve,
            n.attr("Mesure", "instant"),
            Value::DateTime(instant),
        )
        .expect("instant");
    assert_eq!(
        store.get(releve, n.attr("Mesure", "instant")),
        Value::DateTime(instant)
    );
}

fn new_releve(store: &mut Store, n: &Names, station: ObjectId) -> ObjectId {
    let thermometer = add(store, n, station, ("Station", "equipements"), "Thermometre");
    add(store, n, thermometer, ("Capteur", "mesures"), "Releve")
}

/// Scenario "Valeur modifiée".
#[test]
fn value_is_modified() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let releve = new_releve(&mut store, &n, s);
    let valeur = n.attr("Releve", "valeur");
    store.set(releve, valeur, Value::Float(11.0)).expect("set");
    store.set(releve, valeur, Value::Float(12.5)).expect("set");
    assert_eq!(store.get(releve, valeur), Value::Float(12.5));
}

/// Scenario "Texte remplacé".
#[test]
fn text_is_replaced() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "Brest");
    store
        .set(s, n.attr("Station", "nom"), Value::Str("Brest-Guipavas"))
        .expect("set");
    assert_eq!(
        store.get(s, n.attr("Station", "nom")),
        Value::Str("Brest-Guipavas")
    );
}

/// Scenario "Valeur de mauvais type refusée", and other values that do not fit.
#[test]
fn wrong_value_type_is_refused() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let altitude = n.attr("Station", "altitude");
    store.set(s, altitude, Value::Int(98)).expect("set");
    let error = store
        .set(s, altitude, Value::Str("haute"))
        .expect_err("wrong type");
    assert_eq!(
        error,
        StoreError::WrongValueType {
            attribute: "altitude".into(),
            expected: "Int".into()
        }
    );
    assert_eq!(store.get(s, altitude), Value::Int(98));

    // A literal outside the enum, an instant finer than the microsecond.
    let categorie = n.attr("Station", "categorie");
    assert!(store.set(s, categorie, Value::Enum(3)).is_err());
    assert_eq!(store.get(s, categorie), Value::Unset);
    let releve = new_releve(&mut store, &n, s);
    let nanos: Timestamp = "2026-03-12T10:15:00.123456789Z".parse().expect("instant");
    assert!(
        store
            .set(releve, n.attr("Mesure", "instant"), Value::DateTime(nanos))
            .is_err()
    );
}

/// Scenario "Attribut remis à l'état non renseigné".
#[test]
fn attribute_is_unset() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let thermometer = add(&mut store, &n, s, ("Station", "equipements"), "Thermometre");
    let modele = n.attr("Equipement", "modele");
    store
        .set(thermometer, modele, Value::Str("PT100"))
        .expect("set");
    store.set(thermometer, modele, Value::Unset).expect("unset");
    assert_eq!(store.get(thermometer, modele), Value::Unset);
}

/// Scenario "Remontée jusqu'à la racine".
#[test]
fn walk_up_to_the_root() {
    let (mut store, n, root) = new_store();
    let zone = add(&mut store, &n, root, ("Reseau", "zones"), "ZoneAlerte");
    let cell = add(&mut store, &n, zone, ("ZoneAlerte", "mailles"), "Maille");
    let corner = add(&mut store, &n, cell, ("Maille", "coins"), "Point");
    let mut path = Vec::new();
    let mut current = corner;
    while let Some((parent, reference)) = store.container(current) {
        path.push((parent, store.model().reference(reference).name().to_owned()));
        current = parent;
    }
    assert_eq!(
        path,
        [
            (cell, "coins".to_owned()),
            (zone, "mailles".to_owned()),
            (root, "zones".to_owned())
        ]
    );
    assert_eq!(store.root(), Some(root));
    assert_eq!(store.container(root), None);
}

/// Every reference, declared or inherited, is reported by `container`: the root declares three
/// references and `Node` two, so the local index of a reference differs from its rank in the
/// whole model.
#[test]
fn container_reports_every_reference() {
    let text = r#"{
      "formatVersion": "0.1",
      "packages": [{ "name": "p", "classes": [
        { "name": "Root", "references": [
          { "name": "a", "type": "Node", "upperBound": -1 },
          { "name": "b", "type": "Node", "upperBound": -1 },
          { "name": "c", "type": "Node", "upperBound": -1 } ] },
        { "name": "Node", "references": [
          { "name": "x", "type": "Node", "upperBound": -1 },
          { "name": "y", "type": "Node", "upperBound": -1 } ] },
        { "name": "Leaf", "superClass": "Node" } ] }]
    }"#;
    let model = Model::compile(&[ModelFile::parse(text).expect("parse")]).expect("compile");
    let n = Names {
        model: Arc::new(model),
    };
    let mut store = Store::new(n.model.clone());
    let root = store.create_root(n.class("Root")).expect("root");
    let node = add(&mut store, &n, root, ("Root", "c"), "Node");
    let leaf = add(&mut store, &n, node, ("Node", "y"), "Leaf");
    let below_leaf = add(&mut store, &n, leaf, ("Node", "y"), "Node");
    let first = add(&mut store, &n, leaf, ("Node", "x"), "Node");
    for (child, parent, reference) in [
        (node, root, ("Root", "c")),
        (leaf, node, ("Node", "y")),
        (below_leaf, leaf, ("Node", "y")),
        (first, leaf, ("Node", "x")),
    ] {
        let expected = n.reference(reference.0, reference.1);
        assert_eq!(
            store.container(child),
            Some((parent, expected)),
            "{reference:?}"
        );
    }
}

/// Scenario "Liste polymorphe".
#[test]
fn polymorphic_list() {
    let (mut store, n, root) = new_store();
    let s = station(&mut store, &n, root, "A");
    let classes = ["Thermometre", "Camera", "Anemometre"];
    let ids: Vec<ObjectId> = classes
        .iter()
        .map(|c| add(&mut store, &n, s, ("Station", "equipements"), c))
        .collect();
    let children = store.children(s, n.reference("Station", "equipements"));
    assert_eq!(children, ids);
    let read: Vec<&str> = children
        .iter()
        .map(|&e| store.model().class(store.class_of(e)).name())
        .collect();
    assert_eq!(read, classes);
}

/// Scenario "Suppression au milieu d'une liste".
#[test]
fn delete_in_the_middle_of_a_list() {
    let (mut store, n, root) = new_store();
    let ids: Vec<ObjectId> = ["A", "B", "C"]
        .iter()
        .map(|name| station(&mut store, &n, root, name))
        .collect();
    store.delete(ids[1]);
    assert_eq!(
        store.children(root, n.reference("Reseau", "stations")),
        [ids[0], ids[2]]
    );
}

/// Scenario "Suppression d'un sous-arbre".
#[test]
fn delete_a_subtree() {
    let (mut store, n, root) = new_store();
    let doomed = station(&mut store, &n, root, "A");
    let kept = station(&mut store, &n, root, "B");
    let mut subtree = vec![doomed];
    for s in [doomed, kept] {
        let thermometer = add(&mut store, &n, s, ("Station", "equipements"), "Thermometre");
        let lacune = add(
            &mut store,
            &n,
            thermometer,
            ("Capteur", "mesures"),
            "Lacune",
        );
        let annotation = add(
            &mut store,
            &n,
            lacune,
            ("Mesure", "annotations"),
            "Annotation",
        );
        let panne = add(&mut store, &n, s, ("Station", "interventions"), "Panne");
        if s == doomed {
            subtree.extend([thermometer, lacune, annotation, panne]);
        }
    }
    store.delete(doomed);
    assert!(subtree.iter().all(|&id| !store.is_alive(id)));
    assert_eq!(
        store.children(root, n.reference("Reseau", "stations")),
        [kept]
    );
    assert_eq!(names_of(&store, &n, &[kept]), ["B"]);
    let thermometer = store.children(kept, n.reference("Station", "equipements"))[0];
    let lacune = store.children(thermometer, n.reference("Capteur", "mesures"))[0];
    assert_eq!(
        store
            .children(lacune, n.reference("Mesure", "annotations"))
            .len(),
        1
    );
}

#[test]
fn second_root_is_refused() {
    let (mut store, n, _root) = new_store();
    assert_eq!(
        store.create_root(n.class("Reseau")),
        Err(StoreError::RootAlreadyExists)
    );
}

/// Scenario "Store construit dans un autre thread".
#[test]
fn store_built_in_another_thread() {
    let handle = std::thread::spawn(|| {
        let (mut store, n, root) = new_store();
        station(&mut store, &n, root, "Brest");
        store
    });
    let store = handle.join().expect("thread");
    let model = network_model();
    let n = Names { model };
    let root = store.root().expect("root");
    let stations = store.children(root, n.reference("Reseau", "stations"));
    assert_eq!(names_of(&store, &n, stations), ["Brest"]);
}

/// Compiles only if `T` can be sent to another thread.
fn assert_send<T: Send>() {}

#[test]
fn store_is_send() {
    assert_send::<Store>();
}
