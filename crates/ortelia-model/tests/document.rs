//! A document read from outside the crate, as the other layers do (specification
//! `change-tracking`).

mod common;

use ortelia_model::document::Document;
use ortelia_model::store::Position;
use ortelia_model::value::Value;

use common::{Names, directory_model};

/// Scenario "Lecture par le Store": the values and children recorded through transactions are
/// read through `Document::store()`.
#[test]
fn reading_through_the_store() {
    let model = directory_model();
    let n = Names::new(&model);
    let mut document = Document::new(model, n.class("Racine")).expect("document");
    let root = document.root();
    let mut edit = document.edit();
    let directory = edit
        .create_child(
            root,
            n.reference("Racine", "annuaire"),
            Position::End,
            n.class("Annuaire"),
        )
        .expect("annuaire");
    let mut persons = Vec::new();
    for (name, children) in [("Dupont", 2), ("Martin", 0)] {
        let person = edit
            .create_child(
                directory,
                n.reference("Annuaire", "personnes"),
                Position::End,
                n.class("Personne"),
            )
            .expect("person");
        edit.set(person, n.attr("Personne", "nom"), Value::Str(name))
            .expect("nom");
        edit.set(
            person,
            n.attr("Personne", "nombre_enfants"),
            Value::Int(children),
        )
        .expect("nombre_enfants");
        persons.push(person);
    }
    drop(edit);

    let store = document.store();
    assert_eq!(
        store.children(root, n.reference("Racine", "annuaire")),
        [directory]
    );
    assert_eq!(
        store.children(directory, n.reference("Annuaire", "personnes")),
        persons
    );
    assert_eq!(
        store.get(persons[0], n.attr("Personne", "nom")),
        Value::Str("Dupont")
    );
    assert_eq!(
        store.get(persons[1], n.attr("Personne", "nombre_enfants")),
        Value::Int(0)
    );
    assert_eq!(store.container(persons[1]).map(|c| c.0), Some(directory));
}
