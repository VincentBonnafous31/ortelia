//! Tests of the document (specification `change-tracking`).

use super::*;
use crate::test_support::{Names, directory_model};

/// A directory document: root `Racine`, its `Annuaire`, and `count` persons named `P0`, `P1`...
/// created in one transaction (revision 1).
fn directory(count: usize) -> (Document, Names, ObjectId, Vec<ObjectId>) {
    let model = directory_model();
    let n = Names::new(&model);
    let mut document = Document::new(model, n.class("Racine")).expect("document");
    let root = document.root();
    let mut edit = document.edit();
    let annuaire = edit
        .create_child(
            root,
            n.reference("Racine", "annuaire"),
            Position::End,
            n.class("Annuaire"),
        )
        .expect("annuaire");
    let mut persons = Vec::new();
    for i in 0..count {
        let person = edit
            .create_child(
                annuaire,
                n.reference("Annuaire", "personnes"),
                Position::End,
                n.class("Personne"),
            )
            .expect("person");
        let name = format!("P{i}");
        edit.set(person, n.attr("Personne", "nom"), Value::Str(&name))
            .expect("nom");
        persons.push(person);
    }
    drop(edit);
    (document, n, annuaire, persons)
}

/// Scenario "Document neuf".
#[test]
fn new_document() {
    let model = directory_model();
    let n = Names::new(&model);
    let document = Document::new(model, n.class("Racine")).expect("document");
    let store = document.store();
    assert_eq!(store.class_of(document.root()), n.class("Racine"));
    assert_eq!(store.root(), Some(document.root()));
    assert!(
        store
            .children(document.root(), n.reference("Racine", "annuaire"))
            .is_empty()
    );
    assert_eq!(document.revision(), 0);
    assert!(document.journal().is_empty());
}

/// Scenario "Racine abstraite refusée".
#[test]
fn abstract_root_is_refused() {
    let model = crate::test_support::network_model();
    let n = Names::new(&model);
    let error = Document::new(model, n.class("Equipement")).expect_err("abstract");
    assert_eq!(
        error,
        StoreError::AbstractClass {
            class: "Equipement".into()
        }
    );
    assert!(error.to_string().contains("Equipement"));
}

/// Scenario "Suppression de la racine refusée".
#[test]
fn root_cannot_be_deleted() {
    let (mut document, _n, annuaire, _) = directory(1);
    let root = document.root();
    let mut edit = document.edit();
    assert_eq!(edit.delete(root), Err(StoreError::RootNotDeletable));
    // Nothing of a batch with the root is deleted.
    assert_eq!(
        edit.delete_many(&[annuaire, root]),
        Err(StoreError::RootNotDeletable)
    );
    drop(edit);
    assert!(document.store().is_alive(root));
    assert!(document.store().is_alive(annuaire));
    assert_eq!(document.revision(), 1);
    assert_eq!(document.journal().len(), 1);
}

/// Giving a float: compared by its bits, so that -0.0 is a change and NaN again is none.
#[test]
fn floats_are_compared_by_their_bits() {
    let model = network_model();
    let n = Names::new(&model);
    let mut document = Document::new(model, n.class("Reseau")).expect("document");
    let root = document.root();
    let mut edit = document.edit();
    let station = edit
        .create_child(
            root,
            n.reference("Reseau", "stations"),
            Position::End,
            n.class("Station"),
        )
        .expect("station");
    let latitude = n.attr("Station", "latitude");
    edit.set(station, latitude, Value::Float(0.0)).expect("0.0");
    drop(edit);
    // -0.0 == 0.0 for `==`, but it is another value: it is stored and recorded.
    let revision = document.revision();
    document
        .edit()
        .set(station, latitude, Value::Float(-0.0))
        .expect("-0.0");
    assert_eq!(document.revision(), revision + 1);
    let Value::Float(stored) = document.store().get(station, latitude) else {
        panic!("float")
    };
    assert!(stored.is_sign_negative());
    // NaN != NaN for `==`, but giving it again changes nothing: nothing is recorded.
    document
        .edit()
        .set(station, latitude, Value::Float(f64::NAN))
        .expect("NaN");
    let revision = document.revision();
    document
        .edit()
        .set(station, latitude, Value::Float(f64::NAN))
        .expect("NaN again");
    assert_eq!(document.revision(), revision);
}

/// Scenario "Trois modifications dans une transaction".
#[test]
fn three_changes_in_one_transaction() {
    let (mut document, n, _, persons) = directory(1);
    let person = persons[0];
    let nom = n.attr("Personne", "nom");
    for i in 0..3 {
        // Revisions 2 to 4.
        let name = format!("Q{i}");
        document
            .edit()
            .set(person, nom, Value::Str(&name))
            .expect("nom");
    }
    assert_eq!(document.revision(), 4);

    let date: Date = "1985-03-12".parse().expect("date");
    let mut edit = document.edit();
    edit.set(person, nom, Value::Str("Dupont")).expect("nom");
    edit.set(person, n.attr("Personne", "prenom"), Value::Str("Marie"))
        .expect("prenom");
    edit.set(
        person,
        n.attr("Personne", "date_naissance"),
        Value::Date(date),
    )
    .expect("date");
    drop(edit);

    assert_eq!(document.revision(), 5);
    assert_eq!(document.journal().len(), 5);
    let last = document.journal().last().expect("transaction");
    assert_eq!(last.revision, 5);
    let attrs: Vec<AttrId> = last
        .changes
        .iter()
        .map(|change| match change {
            Change::Set { attr, .. } => *attr,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        attrs,
        [
            nom,
            n.attr("Personne", "prenom"),
            n.attr("Personne", "date_naissance")
        ]
    );
}

/// Scenario "Transaction vide".
#[test]
fn empty_transaction() {
    let (mut document, _, _, _) = directory(2);
    let journal = document.journal().to_vec();
    drop(document.edit());
    assert_eq!(document.revision(), 1);
    assert_eq!(document.journal(), journal);
}

/// Scenario "Ancienne et nouvelle valeur".
#[test]
fn old_and_new_value() {
    let (mut document, n, _, persons) = directory(1);
    let nom = n.attr("Personne", "nom");
    document
        .edit()
        .set(persons[0], nom, Value::Str("Dupont"))
        .expect("nom");
    document
        .edit()
        .set(persons[0], nom, Value::Str("Dupond"))
        .expect("nom");
    let last = document.journal().last().expect("transaction");
    assert_eq!(
        last.changes,
        [Change::Set {
            obj: persons[0],
            attr: nom,
            old: OwnedValue::Str("Dupont".into()),
            new: OwnedValue::Str("Dupond".into())
        }]
    );
    // A value absent before is recorded as such.
    let prenom = n.attr("Personne", "prenom");
    document
        .edit()
        .set(persons[0], prenom, Value::Str("Marie"))
        .expect("prenom");
    let Change::Set { old, new, .. } = &document.journal().last().expect("tx").changes[0] else {
        panic!("not a set");
    };
    assert_eq!(
        (old, new.as_value()),
        (&OwnedValue::Unset, Value::Str("Marie"))
    );
}

/// Scenario "Valeur inchangée".
#[test]
fn unchanged_value() {
    let (mut document, n, _, persons) = directory(1);
    let nom = n.attr("Personne", "nom");
    document
        .edit()
        .set(persons[0], nom, Value::Str("Dupont"))
        .expect("nom");
    let revision = document.revision();
    document
        .edit()
        .set(persons[0], nom, Value::Str("Dupont"))
        .expect("nom");
    assert_eq!(document.revision(), revision);
    assert_eq!(document.journal().len(), 2);
}

/// Scenario "Valeur refusée".
#[test]
fn refused_value() {
    let (mut document, n, _, persons) = directory(1);
    let children = n.attr("Personne", "nombre_enfants");
    document
        .edit()
        .set(persons[0], children, Value::Int(2))
        .expect("children");
    let revision = document.revision();
    let mut edit = document.edit();
    let error = edit
        .set(persons[0], children, Value::Str("deux"))
        .expect_err("wrong type");
    assert!(
        matches!(error, StoreError::WrongValueType { .. }),
        "{error}"
    );
    drop(edit);
    assert_eq!(document.store().get(persons[0], children), Value::Int(2));
    assert_eq!(document.revision(), revision);
}

/// Scenario "Création enregistrée", at the end and at a position.
#[test]
fn creation_is_recorded() {
    let (mut document, n, annuaire, _) = directory(10);
    let personnes = n.reference("Annuaire", "personnes");
    let class = n.class("Personne");
    let mut edit = document.edit();
    let last = edit
        .create_child(annuaire, personnes, Position::End, class)
        .expect("person");
    let first = edit
        .create_child(annuaire, personnes, Position::At(0), class)
        .expect("person");
    drop(edit);
    let changes = &document.journal().last().expect("transaction").changes;
    assert_eq!(
        changes,
        &[
            Change::Created {
                obj: last,
                class,
                parent: annuaire,
                reference: personnes,
                index: 10
            },
            Change::Created {
                obj: first,
                class,
                parent: annuaire,
                reference: personnes,
                index: 0
            }
        ]
    );
    // A refused creation records nothing.
    let revision = document.revision();
    let error = document
        .edit()
        .create_child(annuaire, personnes, Position::At(99), class)
        .expect_err("past the end");
    assert!(matches!(error, StoreError::PositionOutOfRange { .. }));
    assert_eq!(document.revision(), revision);
}

/// Scenario "Suppression enregistrée".
#[test]
fn deletion_is_recorded() {
    let (mut document, n, annuaire, persons) = directory(5);
    document.edit().delete(persons[2]).expect("delete");
    let changes = &document.journal().last().expect("transaction").changes;
    assert_eq!(
        changes,
        &[Change::Deleted {
            obj: persons[2],
            parent: annuaire,
            reference: n.reference("Annuaire", "personnes"),
            index: 2
        }]
    );
    assert!(!document.store().is_alive(persons[2]));
}

/// Scenario "Suppression en lot" of `change-tracking`.
#[test]
fn batch_deletion_is_one_transaction() {
    let (mut document, n, annuaire, persons) = directory(5);
    let revision = document.revision();
    document
        .edit()
        .delete_many(&[persons[0], persons[2], persons[4]])
        .expect("delete");
    let personnes = n.reference("Annuaire", "personnes");
    assert_eq!(
        document.store().children(annuaire, personnes),
        [persons[1], persons[3]]
    );
    assert_eq!(document.revision(), revision + 1);
    let changes = &document.journal().last().expect("transaction").changes;
    let deleted: Vec<(ObjectId, usize)> = changes
        .iter()
        .map(|change| match change {
            Change::Deleted { obj, index, .. } => (*obj, *index),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(deleted, [(persons[0], 0), (persons[2], 2), (persons[4], 4)]);
}

/// Scenario "Changements depuis une révision".
#[test]
fn changes_since_a_revision() {
    let (mut document, n, _, persons) = directory(1);
    let nom = n.attr("Personne", "nom");
    for name in ["A", "B"] {
        document
            .edit()
            .set(persons[0], nom, Value::Str(name))
            .expect("nom");
    }
    let noted = document.revision();
    assert_eq!(noted, 3);
    for name in ["C", "D"] {
        document
            .edit()
            .set(persons[0], nom, Value::Str(name))
            .expect("nom");
    }
    let since = document.changes_since(noted);
    let revisions: Vec<u64> = since.iter().map(|t| t.revision).collect();
    assert_eq!(revisions, [4, 5]);
    assert_eq!(since, &document.journal()[3..]);
    assert!(document.changes_since(document.revision()).is_empty());
    assert_eq!(document.changes_since(0).len(), 5);
}

/// Scenario "Lectures sans effet".
#[test]
fn reads_leave_the_revision_unchanged() {
    let (document, n, annuaire, persons) = directory(3);
    let revision = document.revision();
    let nom = n.attr("Personne", "nom");
    let personnes = n.reference("Annuaire", "personnes");
    for i in 0..1000 {
        let store = document.store();
        assert_eq!(store.children(annuaire, personnes).len(), 3);
        let name = format!("P{}", i % 3);
        assert_eq!(store.get(persons[i % 3], nom), Value::Str(&name));
    }
    assert_eq!(document.revision(), revision);
}

// --- Integration of a fragment (`Edit::replace`) ---

use crate::builder::{Builder, DataPath, PathSegment};
use crate::test_support::network_model;

/// A network document: two stations with a thermometer each, and one zone (revision 1).
struct Network {
    document: Document,
    n: Names,
    stations: Vec<ObjectId>,
    thermometers: Vec<ObjectId>,
    zone: ObjectId,
}

fn network() -> Network {
    let model = network_model();
    let n = Names::new(&model);
    let mut document = Document::new(model, n.class("Reseau")).expect("document");
    let root = document.root();
    let mut edit = document.edit();
    let mut stations = Vec::new();
    let mut thermometers = Vec::new();
    for name in ["Brest", "Quimper"] {
        let station = edit
            .create_child(
                root,
                n.reference("Reseau", "stations"),
                Position::End,
                n.class("Station"),
            )
            .expect("station");
        edit.set(station, n.attr("Station", "nom"), Value::Str(name))
            .expect("nom");
        let thermometer = edit
            .create_child(
                station,
                n.reference("Station", "equipements"),
                Position::End,
                n.class("Thermometre"),
            )
            .expect("thermometer");
        stations.push(station);
        thermometers.push(thermometer);
    }
    let zone = edit
        .create_child(
            root,
            n.reference("Reseau", "zones"),
            Position::End,
            n.class("ZoneAlerte"),
        )
        .expect("zone");
    drop(edit);
    Network {
        document,
        n,
        stations,
        thermometers,
        zone,
    }
}

/// The fragment of a builder that recorded no error.
fn fragment_of(builder: Builder) -> Fragment {
    builder.finish().expect("no load error").fragment
}

fn branch_path(names: &[&str]) -> DataPath {
    DataPath(
        names
            .iter()
            .map(|name| PathSegment::Name((*name).to_owned()))
            .collect(),
    )
}

/// A fragment for the branch `stations`: one station per name, each with a camera.
fn stations_fragment(model: &Arc<Model>, n: &Names, names: &[&str]) -> Fragment {
    let shape = Shape::of(model, n.reference("Reseau", "stations"));
    let mut builder = Builder::new(model.clone(), shape, branch_path(&["stations"]));
    for name in names {
        assert!(builder.begin_head(n.class("Station")));
        builder.set_text(n.attr("Station", "nom"), name);
        builder.begin_reference(n.reference("Station", "equipements"));
        assert!(builder.begin_object(n.class("Camera")));
        builder.end_object();
        builder.end_reference();
        builder.end_object();
    }
    fragment_of(builder)
}

fn name_of(store: &Store, n: &Names, station: ObjectId) -> String {
    match store.get(station, n.attr("Station", "nom")) {
        Value::Str(text) => text.to_owned(),
        other => panic!("{other:?}"),
    }
}

/// Scenario "Remplacement d'une liste", then a second integration in the same branch.
#[test]
fn replace_a_list() {
    let Network {
        mut document,
        n,
        stations: old_stations,
        thermometers,
        zone,
    } = network();
    // A fragment must be built with the document's model: the same `Arc`.
    let model = document.store().model_arc().clone();
    let stations = n.reference("Reseau", "stations");
    let root = document.root();
    let target = Target::Reference {
        parent: root,
        reference: stations,
    };
    let fragment = stations_fragment(&model, &n, &["A", "B", "C"]);
    let revision = document.revision();
    document.edit().replace(target, fragment).expect("replace");

    let store = document.store();
    store.assert_consistent(&[root]);
    let new_stations = store.children(root, stations).to_vec();
    let names: Vec<String> = new_stations
        .iter()
        .map(|&s| name_of(store, &n, s))
        .collect();
    assert_eq!(names, ["A", "B", "C"]);
    for old in old_stations.iter().chain(&thermometers) {
        assert!(!store.is_alive(*old), "{old:?}");
    }
    assert!(store.is_alive(zone));
    assert_eq!(store.children(root, n.reference("Reseau", "zones")), [zone]);
    assert_eq!(document.revision(), revision + 1);
    assert_eq!(
        document.journal().last().expect("transaction").changes,
        [Change::Replaced {
            target,
            objects: new_stations.clone()
        }]
    );

    // A second integration in the same branch replaces the first one.
    let fragment = stations_fragment(&model, &n, &["D"]);
    document.edit().replace(target, fragment).expect("replace");
    let store = document.store();
    store.assert_consistent(&[root]);
    let last = store.children(root, stations);
    assert_eq!(last.len(), 1);
    assert_eq!(name_of(store, &n, last[0]), "D");
    assert!(new_stations.iter().all(|&s| !store.is_alive(s)));
    assert!(last[0] > *new_stations.last().expect("station"));
}

/// Scenario "Valeurs intactes après intégration": every base type, a text longer than a block
/// of the string arena, children, and identifiers never given before.
#[test]
fn values_are_intact_after_integration() {
    let Network {
        mut document, n, ..
    } = network();
    let model = document.store().model_arc().clone();
    let root = document.root();
    // Identifiers are indexes in creation order: all those given so far are below this one.
    let next_id = document
        .store()
        .descendants(root)
        .iter()
        .max()
        .expect("root")
        .index()
        + 1;
    let long = "é".repeat(600_000); // 1.2 MB: a block of its own
    let shape = Shape::of(&model, n.reference("Reseau", "stations"));
    let mut builder = Builder::new(model.clone(), shape, branch_path(&["stations"]));
    assert!(builder.begin_head(n.class("Station")));
    let texts = [
        ("nom", "Brest-Guipavas"),
        ("latitude", "48.44"),
        ("altitude", "94"),
        ("mise_en_service", "1985-03-12"),
        ("active", "true"),
        ("categorie", "SYNOPTIQUE"),
        ("description", long.as_str()),
    ];
    for (attr, text) in texts {
        builder.set_text(n.attr("Station", attr), text);
    }
    builder.begin_reference(n.reference("Station", "equipements"));
    assert!(builder.begin_object(n.class("Thermometre")));
    builder.begin_reference(n.reference("Capteur", "mesures"));
    assert!(builder.begin_object(n.class("Releve")));
    builder.set_text(n.attr("Mesure", "instant"), "2026-03-12T10:15:00.123456Z");
    for _ in 0..2 {
        builder.end_object();
        builder.end_reference();
    }
    builder.end_object();
    let fragment = fragment_of(builder);
    // What the fragment holds, read before the integration.
    let expected = dump(fragment.store(), fragment.heads()[0]);

    let target = Target::Reference {
        parent: root,
        reference: n.reference("Reseau", "stations"),
    };
    document.edit().replace(target, fragment).expect("replace");
    let store = document.store();
    store.assert_consistent(&[root]);
    let station = store.children(root, n.reference("Reseau", "stations"))[0];
    assert_eq!(dump(store, station), expected);
    assert!(
        store
            .descendants(station)
            .iter()
            .all(|id| id.index() >= next_id),
        "identifiers reused"
    );
    assert_eq!(
        store.get(station, n.attr("Station", "description")),
        Value::Str(&long)
    );
}

/// The classes and values of a subtree, depth first.
fn dump(store: &Store, obj: ObjectId) -> Vec<String> {
    let model = store.model();
    store
        .descendants(obj)
        .into_iter()
        .map(|id| {
            let class = model.class(store.class_of(id));
            let values: Vec<String> = class
                .attributes()
                .iter()
                .map(|a| format!("{}={:?}", a.name(), store.get(id, a.id())))
                .collect();
            format!("{} {}", class.name(), values.join(" "))
        })
        .collect()
}

/// Scenario "Remplacement de la racine".
#[test]
fn replace_the_root() {
    let Network {
        mut document,
        n,
        stations,
        zone,
        ..
    } = network();
    let model = document.store().model_arc().clone();
    let old_root = document.root();
    let shape = Shape::root(n.class("Reseau"));
    let mut builder = Builder::new(model.clone(), shape, DataPath::default());
    assert!(builder.begin_head(n.class("Reseau")));
    builder.set_text(n.attr("Reseau", "nom"), "Bretagne");
    builder.begin_reference(n.reference("Reseau", "stations"));
    assert!(builder.begin_object(n.class("Station")));
    builder.end_object();
    builder.end_reference();
    builder.end_object();
    document
        .edit()
        .replace(Target::Root, fragment_of(builder))
        .expect("replace");

    let root = document.root();
    let store = document.store();
    store.assert_consistent(&[root]);
    assert_ne!(root, old_root);
    assert_eq!(store.root(), Some(root));
    assert_eq!(
        store.get(root, n.attr("Reseau", "nom")),
        Value::Str("Bretagne")
    );
    assert_eq!(
        store
            .children(root, n.reference("Reseau", "stations"))
            .len(),
        1
    );
    for old in stations.iter().chain([&old_root, &zone]) {
        assert!(!store.is_alive(*old), "{old:?}");
    }
    assert_eq!(
        document.journal().last().expect("transaction").changes,
        [Change::Replaced {
            target: Target::Root,
            objects: vec![root]
        }]
    );
}

/// A single reference is replaced too, here twice in a row.
#[test]
fn replace_a_single_reference() {
    let (mut document, n, annuaire, persons) = directory(2);
    let model = document.store().model_arc().clone();
    let root = document.root();
    let reference = n.reference("Racine", "annuaire");
    let target = Target::Reference {
        parent: root,
        reference,
    };
    for _ in 0..2 {
        let shape = Shape::of(&model, reference);
        let mut builder = Builder::new(model.clone(), shape, branch_path(&["annuaire"]));
        assert!(builder.begin_head(n.class("Annuaire")));
        builder.begin_reference(n.reference("Annuaire", "personnes"));
        assert!(builder.begin_object(n.class("Personne")));
        builder.end_object();
        builder.end_reference();
        builder.end_object();
        document
            .edit()
            .replace(target, fragment_of(builder))
            .expect("replace");
        document.store().assert_consistent(&[root]);
    }
    let store = document.store();
    assert!(!store.is_alive(annuaire));
    assert!(persons.iter().all(|&p| !store.is_alive(p)));
    let new = store.children(root, reference);
    assert_eq!(new.len(), 1);
    assert_eq!(
        store
            .children(new[0], n.reference("Annuaire", "personnes"))
            .len(),
        1
    );
}

/// Scenario "Fragment d'un autre type".
#[test]
fn fragment_of_another_type_is_refused() {
    let Network {
        mut document,
        n,
        stations,
        ..
    } = network();
    let model = document.store().model_arc().clone();
    let shape = Shape::of(&model, n.reference("Reseau", "zones"));
    let fragment = fragment_of(Builder::new(model, shape, branch_path(&["zones"])));
    let root = document.root();
    let revision = document.revision();
    let target = Target::Reference {
        parent: root,
        reference: n.reference("Reseau", "stations"),
    };
    let error = document
        .edit()
        .replace(target, fragment)
        .expect_err("other type");
    assert_eq!(
        error,
        IntegrateError::WrongShape {
            expected: "a list of `Station`".into(),
            found: "a list of `ZoneAlerte`".into()
        }
    );
    assert_eq!(
        document
            .store()
            .children(root, n.reference("Reseau", "stations")),
        stations
    );
    assert_eq!(document.revision(), revision);
    assert_eq!(document.journal().len(), 1);
}

/// Scenario "Liste pour une référence simple".
#[test]
fn list_for_a_single_reference_is_refused() {
    let (mut document, n, annuaire, _) = directory(1);
    let model = document.store().model_arc().clone();
    let shape = Shape {
        class: n.class("Annuaire"),
        many: true,
    };
    let fragment = fragment_of(Builder::new(model, shape, DataPath::default()));
    let root = document.root();
    let reference = n.reference("Racine", "annuaire");
    let target = Target::Reference {
        parent: root,
        reference,
    };
    let error = document.edit().replace(target, fragment).expect_err("list");
    assert!(
        matches!(error, IntegrateError::WrongShape { .. }),
        "{error}"
    );
    assert_eq!(document.store().children(root, reference), [annuaire]);
}

/// Scenario "Racine vide".
#[test]
fn empty_root_fragment_is_refused() {
    let (mut document, n, annuaire, _) = directory(1);
    let model = document.store().model_arc().clone();
    let shape = Shape::root(n.class("Racine"));
    let fragment = fragment_of(Builder::new(model, shape, DataPath::default()));
    let root = document.root();
    let error = document
        .edit()
        .replace(Target::Root, fragment)
        .expect_err("empty");
    assert_eq!(error, IntegrateError::EmptyRoot);
    assert_eq!(document.root(), root);
    assert!(document.store().is_alive(annuaire));
}

/// A fragment built with another model (even compiled from the same file) is refused.
#[test]
fn fragment_of_another_model_is_refused() {
    let (mut document, n, _, _) = directory(1);
    let other = directory_model();
    let shape = Shape::root(n.class("Racine"));
    let fragment = fragment_of(Builder::new(other, shape, DataPath::default()));
    let error = document
        .edit()
        .replace(Target::Root, fragment)
        .expect_err("other model");
    assert_eq!(error, IntegrateError::OtherModel);
}

/// Scenario "Fragment lu dans un autre thread".
#[test]
fn fragment_read_in_another_thread() {
    let Network {
        mut document, n, ..
    } = network();
    let model = document.store().model_arc().clone();
    let names = n.clone();
    let handle = std::thread::spawn(move || stations_fragment(&model, &names, &["Lointaine"]));
    let fragment = handle.join().expect("thread");
    let root = document.root();
    let stations = n.reference("Reseau", "stations");
    let target = Target::Reference {
        parent: root,
        reference: stations,
    };
    document.edit().replace(target, fragment).expect("replace");
    let store = document.store();
    let station = store.children(root, stations)[0];
    assert_eq!(name_of(store, &n, station), "Lointaine");
}
