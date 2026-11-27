//! Tests of the form view (specification `form-view`, and the missing labels of `labels`): the
//! form built at the opening, then the values of the object shown.

use ortelia_model::store::Position;
use tempfile::TempDir;

use super::*;
use crate::test_support::{
    capture_log, directory_copy, directory_edited, directory_edited_with, directory_with, example,
    folder, form, lock_log_tests, open_dir, open_example, person,
};
use crate::{Align, AppState, ObjectId, ViewRef};

/// The model of an open application.
fn model(state: &AppState) -> &Model {
    state
        .app
        .as_ref()
        .expect("an application")
        .context
        .document()
        .model()
}

/// A cell, as `(what, row, col, span)`: the text of a label, a title or a text, the attribute
/// and the kind of a widget, `---` for a separator.
type Described = (String, usize, usize, usize);

fn describe(grid: &FormGrid, model: &Model) -> Vec<Described> {
    grid.cells
        .iter()
        .zip(grid.grid.cells())
        .map(|(cell, place)| {
            let what = match cell {
                FormCell::Label(text) => text.clone(),
                FormCell::Widget(field) => {
                    format!("{}:{:?}", model.attribute(field.attr).name(), field.kind)
                }
                FormCell::Group { title, .. } => {
                    format!("Group({})", title.as_deref().unwrap_or(""))
                }
                FormCell::Separator => "---".to_owned(),
                FormCell::Text(text) => format!("Text({text})"),
            };
            (what, place.row, place.col, place.span)
        })
        .collect()
}

/// `(what, row, col, span)` with a `&str`.
fn cell(what: &str, row: usize, col: usize, span: usize) -> Described {
    (what.to_owned(), row, col, span)
}

/// The grid of the first group of a grid.
fn first_group(grid: &FormGrid) -> &FormGrid {
    grid.cells
        .iter()
        .find_map(|cell| match cell {
            FormCell::Group { grid, .. } => Some(&**grid),
            _ => None,
        })
        .expect("a group")
}

/// The widget of the first field of the attribute `name`, in the grid or its groups.
fn field<'a>(grid: &'a FormGrid, model: &Model, name: &str) -> Option<&'a FieldCell> {
    grid.cells.iter().find_map(|cell| match cell {
        FormCell::Widget(field) if model.attribute(field.attr).name() == name => Some(field),
        FormCell::Group { grid, .. } => self::field(grid, model, name),
        _ => None,
    })
}

/// The value of the field of the attribute `name` of the form view `proprietes`.
fn value(state: &AppState, name: &str) -> FieldValue {
    let form = form(state, "proprietes");
    let field = field(form.form(), model(state), name).unwrap_or_else(|| panic!("no {name}"));
    form.value(field)
}

fn text(text: &str) -> FieldValue {
    FieldValue::Text(text.to_owned())
}

fn choice(selected: Option<usize>, text: &str) -> FieldValue {
    FieldValue::Choice {
        selected,
        text: text.to_owned(),
    }
}

/// The elements of the form of `Personne` in an `app.json`.
fn elements(app: &mut serde_json::Value) -> &mut serde_json::Value {
    &mut app["presentations"][0]["form"]["elements"]
}

/// Adds the label `key` of text `text` to the French labels of an `app.json`.
fn add_french_label(app: &mut serde_json::Value, key: &str, text: &str) {
    app["labels"][0]["entries"]
        .as_array_mut()
        .expect("entries")
        .push(serde_json::json!({ "key": key, "text": text }));
}

// --- The form, built at the opening ---

/// Scenario "Vue de droite de l'annuaire" (`form-view`).
#[test]
fn form_view_of_the_directory() {
    let state = open_example();
    let Some(ViewRef::Form(form)) = state.view("proprietes") else {
        panic!("a form view")
    };
    assert_eq!(form.id(), "proprietes");
    assert_eq!(
        form.view.class,
        model(&state).class_id("Personne").expect("Personne")
    );
    assert_eq!(form.object(), None);
}

/// Scenarios "Formulaire de l'annuaire", "Notes sous leur libellé", "Groupe de l'annuaire",
/// "Séparateur", "Libellé avec unité" and "Titre du groupe de l'annuaire".
#[test]
fn form_of_the_directory() {
    let state = open_example();
    let form = form(&state, "proprietes").form();
    let model = model(&state);
    assert_eq!(
        describe(form, model),
        [
            cell("Group(Identité)", 0, 0, 2),
            cell("Enfants", 1, 0, 1),
            cell("nombre_enfants:NumberField", 1, 1, 1),
            cell("Taille (m)", 2, 0, 1),
            cell("taille:NumberField", 2, 1, 1),
            cell("Contact", 3, 0, 1),
            cell("contact_autorise:CheckBox", 3, 1, 1),
            cell("Situation", 4, 0, 1),
            cell("situation:ComboBox", 4, 1, 1),
            cell("Inscription", 5, 0, 1),
            cell("inscription:DateTimeField", 5, 1, 1),
            cell("---", 6, 0, 2),
            cell("Notes", 7, 0, 2),
            cell("notes:TextArea { rows: 5 }", 8, 0, 2),
        ]
    );
    assert_eq!(form.grid.rows(), 9);
    assert_eq!(form.grid.grow(), [0.0, 1.0]);
    let group = first_group(form);
    assert_eq!(
        describe(group, model),
        [
            cell("Nom", 0, 0, 1),
            cell("nom:TextField", 0, 1, 1),
            cell("Prénom", 1, 0, 1),
            cell("prenom:TextField", 1, 1, 1),
            cell("Né le", 2, 0, 1),
            cell("date_naissance:DateField", 2, 1, 1),
        ]
    );
    assert_eq!(group.grid.columns(), 2);
    // Each widget knows the cell of its label.
    let notes = field(form, model, "notes").expect("notes");
    assert!(matches!(&form.cells[notes.label_cell()], FormCell::Label(l) if l == "Notes"));
    let situation = field(form, model, "situation").expect("situation");
    assert_eq!(situation.choices()[1], "Marié(e)");
    // The alignment of the type of the attribute, as in a table.
    for (name, align) in [
        ("nom", Align::Left),
        ("taille", Align::Right),
        ("date_naissance", Align::Center),
    ] {
        assert_eq!(
            field(form, model, name).expect(name).align(),
            align,
            "{name}"
        );
    }
}

/// The model and the `app.json` of an application where `Employe` inherits from `Personne`,
/// whose window shows the form of `Employe`; `presentations` is given as JSON.
fn inherited(presentations: &str) -> TempDir {
    let model = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p", "classes": [
        { "name": "Racine", "references": [
            { "name": "employes", "type": "Employe", "upperBound": -1 } ] },
        { "name": "Personne", "attributes": [
            { "name": "nom", "type": "String" }, { "name": "prenom", "type": "String" } ] },
        { "name": "Employe", "superClass": "Personne",
          "attributes": [ { "name": "matricule", "type": "Int" } ] } ] } ] }"#;
    let app = format!(
        r#"{{ "formatVersion": "0.6", "id": "e", "name": "E", "languages": ["fr"],
             "model": {{ "files": ["m.model.json"], "root": "Racine" }},
             "presentations": {presentations},
             "window": {{ "content": {{ "_type_": "View", "id": "proprietes",
                 "widget": {{ "_type_": "Form", "class": "Employe" }} }} }} }}"#
    );
    folder(&[("app.json", &app), ("m.model.json", model)])
}

/// Scenario "Formulaire hérité"; a presentation of the subclass without form gives the form
/// deduced from the model, as for a table.
#[test]
fn inherited_form() {
    let dir = inherited(
        r#"[ { "class": "Personne", "form": { "elements": [
               { "_type_": "Field", "path": "prenom" } ] } } ]"#,
    );
    let state = open_dir(&dir);
    let fields: Vec<String> = describe(form(&state, "proprietes").form(), model(&state))
        .into_iter()
        .map(|(what, ..)| what)
        .collect();
    assert_eq!(fields, ["#Employe.prenom#", "prenom:TextField"]);

    let dir = inherited(
        r#"[ { "class": "Personne", "form": { "elements": [
               { "_type_": "Field", "path": "prenom" } ] } },
             { "class": "Employe" } ]"#,
    );
    let state = open_dir(&dir);
    let fields: Vec<String> = describe(form(&state, "proprietes").form(), model(&state))
        .into_iter()
        .filter_map(|(what, ..)| what.contains(':').then_some(what))
        .collect();
    assert_eq!(
        fields,
        ["nom:TextField", "prenom:TextField", "matricule:NumberField"]
    );
}

/// Scenarios "Annuaire sans formulaire" and "Widgets de l'annuaire déduit".
#[test]
fn form_deduced_from_the_model() {
    let dir = directory_edited(|app| {
        app["presentations"][0]
            .as_object_mut()
            .expect("presentation")
            .remove("form");
    });
    let state = open_dir(&dir);
    let form = form(&state, "proprietes").form();
    let described = describe(form, model(&state));
    let widgets: Vec<&str> = described
        .iter()
        .filter(|(what, ..)| what.contains(':'))
        .map(|(what, ..)| what.as_str())
        .collect();
    assert_eq!(
        widgets,
        [
            "nom:TextField",
            "prenom:TextField",
            "date_naissance:DateField",
            "nombre_enfants:NumberField",
            "taille:NumberField",
            "contact_autorise:CheckBox",
            "situation:ComboBox",
            "inscription:DateTimeField",
            "notes:TextField",
        ]
    );
    // Each field on its row: its label in column 0, its widget in column 1.
    for (index, (_, row, col, span)) in described.iter().enumerate() {
        assert_eq!(
            (*row, *col, *span),
            (index / 2, index % 2, 1),
            "{described:?}"
        );
    }
    // Scenario "Libellé et widget côte à côte".
    assert_eq!(described[0], cell("Nom", 0, 0, 1));
    assert_eq!(form.grid.grow(), [0.0, 1.0]);
}

/// Scenario "Deux groupes côte à côte".
#[test]
fn two_groups_side_by_side() {
    let dir = directory_edited(|app| {
        app["presentations"][0]["form"] = serde_json::json!({
            "columns": [ {}, { "grow": 1 }, {}, { "grow": 1 } ],
            "elements": [
                { "_type_": "Group", "cell": { "span": 2 }, "elements": [
                    { "_type_": "Field", "path": "nom" } ] },
                { "_type_": "Group", "cell": { "span": 2 }, "elements": [
                    { "_type_": "Field", "path": "prenom" } ] }
            ]
        });
    });
    let state = open_dir(&dir);
    assert_eq!(
        describe(form(&state, "proprietes").form(), model(&state)),
        [cell("Group()", 0, 0, 2), cell("Group()", 0, 2, 2)]
    );
}

/// Scenarios "Clé déclarée par le champ" and "Texte sans libellé"; a group without key has no
/// title.
#[test]
fn keys_declared_by_the_form() {
    let _lock = lock_log_tests();
    let dir = directory_edited(|app| {
        add_french_label(app, "Personne.nom_usage", "Nom d'usage");
        app["presentations"][0]["form"]["elements"] = serde_json::json!([
            { "_type_": "Field", "path": "nom", "label": "Personne.nom_usage" },
            { "_type_": "Field", "path": "taille", "label": "Personne.nom_usage" },
            { "_type_": "Text", "label": "Personne.aide" },
            { "_type_": "Group" },
            { "_type_": "Group", "label": "Personne.identite" }
        ]);
    });
    let state = open_dir(&dir);
    let described = describe(form(&state, "proprietes").form(), model(&state));
    let whats: Vec<&str> = described.iter().map(|(what, ..)| what.as_str()).collect();
    assert_eq!(
        whats,
        [
            "Nom d'usage",
            "nom:TextField",
            // The unit follows a declared key too.
            "Nom d'usage (m)",
            "taille:NumberField",
            "Text(#Personne.aide#)",
            "Group()",
            "Group(Identité)"
        ]
    );
    let FormCell::Group { title, .. } = &form(&state, "proprietes").form().cells[5] else {
        panic!("a group")
    };
    assert_eq!(title, &None);
    assert_eq!(state.warnings, ["missing label `Personne.aide` (fr)"]);
}

/// Scenario "Annuaire au lancement": the labels and titles, and widgets without value.
#[test]
fn empty_form() {
    let state = open_example();
    let form = form(&state, "proprietes");
    let model = model(&state);
    let whats: Vec<String> = describe(first_group(form.form()), model)
        .into_iter()
        .chain(describe(form.form(), model))
        .map(|(what, ..)| what)
        .collect();
    for expected in [
        "Nom",
        "Prénom",
        "Taille (m)",
        "Group(Identité)",
        "---",
        "Notes",
    ] {
        assert!(whats.iter().any(|w| w == expected), "{expected}: {whats:?}");
    }
    for name in ["nom", "date_naissance", "taille", "inscription", "notes"] {
        assert_eq!(value(&state, name), text(""), "{name}");
    }
    assert_eq!(value(&state, "contact_autorise"), FieldValue::Check(None));
    assert_eq!(value(&state, "situation"), choice(None, ""));
}

/// Scenarios "Titre de groupe manquant" and "Libellé manquant dans la table et le formulaire"
/// (`labels`): each missing key once, logged at the `WARN` level.
#[test]
fn missing_labels_of_a_form() {
    let _lock = lock_log_tests();
    let without = |key: &'static str| {
        directory_edited(move |app| {
            let entries = app["labels"][0]["entries"].as_array_mut().expect("entries");
            entries.retain(|entry| entry["key"] != key);
        })
    };
    let dir = without("Personne.identite");
    let (state, log) = capture_log(|| open_dir(&dir));
    assert_eq!(state.warnings, ["missing label `Personne.identite` (fr)"]);
    assert_eq!(log.matches("missing label").count(), 1, "{log}");
    assert!(
        log.contains(" WARN ") && log.contains("Personne.identite"),
        "{log}"
    );
    let FormCell::Group { title, .. } = &form(&state, "proprietes").form().cells[0] else {
        panic!("a group")
    };
    assert_eq!(title.as_deref(), Some("#Personne.identite#"));

    // Shown by the table and by the form: one warning.
    let dir = without("Personne.nom");
    let state = open_dir(&dir);
    assert_eq!(state.warnings, ["missing label `Personne.nom` (fr)"]);
    assert_eq!(state.status_bar(), Some("⚠ 1 avertissement"));
}

// --- The object shown ---

/// The directory opened, its form showing the person `name`.
fn showing(name: &str) -> AppState {
    let mut state = open_example();
    show(&mut state, name);
    state
}

/// Shows the person `name` in the form view `proprietes` of `state`; gives the person.
fn show(state: &mut AppState, name: &str) -> ObjectId {
    let person = person(state, name);
    assert!(state.set_form_object("proprietes", Some(person)));
    person
}

/// Scenarios "Personne Dupont", "Contact autorisé" and "Situation en liste".
#[test]
fn values_of_dupont() {
    let state = showing("Dupont");
    assert_eq!(value(&state, "nom"), text("Dupont"));
    assert_eq!(value(&state, "prenom"), text("Marie"));
    assert_eq!(value(&state, "date_naissance"), text("12 mars 1985"));
    assert_eq!(value(&state, "nombre_enfants"), text("2"));
    assert_eq!(value(&state, "taille"), text("1,68"));
    assert_eq!(value(&state, "inscription"), text("15/01/2024 09:30:00"));
    assert_eq!(
        value(&state, "contact_autorise"),
        FieldValue::Check(Some(true))
    );
    assert_eq!(value(&state, "situation"), choice(Some(1), "Marié(e)"));
    let form = form(&state, "proprietes");
    let situation = field(form.form(), model(&state), "situation").expect("situation");
    assert_eq!(situation.choices()[1], "Marié(e)");
}

/// The persons of a `personnes.json`: one, with the given extra keys.
fn one_person(keys: &str) -> String {
    format!(r#"[ {{ "nom": "Seul" {keys} }} ]"#)
}

/// Scenarios "Notes sur plusieurs lignes" and "Texte sur une ligne".
#[test]
fn notes_on_several_lines() {
    let persons = one_person(r#", "notes": "Ligne 1\nLigne 2""#);
    let dir = directory_with(&example("annuaire/app.json"), &persons);
    let mut state = open_dir(&dir);
    show(&mut state, "Seul");
    assert_eq!(value(&state, "notes"), text("Ligne 1\nLigne 2"));

    let dir = directory_edited_with(
        |app| elements(app)[7]["widget"] = serde_json::json!({ "_type_": "TextField" }),
        &persons,
    );
    let mut state = open_dir(&dir);
    show(&mut state, "Seul");
    assert_eq!(value(&state, "notes"), text("Ligne 1 Ligne 2"));
}

/// Scenario "Booléen absent", and the other values absent.
#[test]
fn absent_values() {
    let dir = directory_with(&example("annuaire/app.json"), &one_person(""));
    let mut state = open_dir(&dir);
    show(&mut state, "Seul");
    assert_eq!(value(&state, "nom"), text("Seul"));
    assert_eq!(value(&state, "contact_autorise"), FieldValue::Check(None));
    assert_eq!(value(&state, "situation"), choice(None, ""));
    assert_eq!(value(&state, "taille"), text(""));
}

/// Scenario "Situation en boutons radio".
#[test]
fn situation_in_radio_buttons() {
    let dir = directory_edited_with(
        |app| elements(app)[4]["widget"] = serde_json::json!({ "_type_": "RadioGroup" }),
        &example("annuaire/personnes.json"),
    );
    let mut state = open_dir(&dir);
    show(&mut state, "Dupont");
    let form = form(&state, "proprietes");
    let situation = field(form.form(), model(&state), "situation").expect("situation");
    assert_eq!(situation.kind(), FieldKind::RadioGroup);
    assert_eq!(
        situation.choices(),
        [
            "Célibataire",
            "Marié(e)",
            "Pacsé(e)",
            "Divorcé(e)",
            "Veuf, veuve"
        ]
    );
    assert_eq!(form.value(situation), choice(Some(1), "Marié(e)"));
}

/// Scenario "Changement d'objet"; then no object, and a view that is not a form.
#[test]
fn change_of_object() {
    let mut state = showing("Dupont");
    let martin = person(&state, "Martin");
    assert!(state.set_form_object("proprietes", Some(martin)));
    assert_eq!(value(&state, "nom"), text("Martin"));
    assert_eq!(value(&state, "prenom"), text("Lucas"));
    assert!(state.set_form_object("proprietes", None));
    assert_eq!(value(&state, "nom"), text(""));
    assert!(!state.set_form_object("personnes", Some(martin)));
    assert!(!state.set_form_object("inconnue", Some(martin)));
}

/// Scenario "Objet d'une autre classe".
#[test]
fn object_of_another_class() {
    let mut state = open_example();
    let document = state.app.as_ref().expect("app").context.document();
    let store = document.store();
    let racine = store.class_of(document.root());
    let annuaire_ref = model(&state)
        .class(racine)
        .reference("annuaire")
        .expect("ref");
    let annuaire = store.children(document.root(), annuaire_ref)[0];
    assert!(state.set_form_object("proprietes", Some(annuaire)));
    assert_eq!(form(&state, "proprietes").object(), None);
    assert_eq!(value(&state, "nom"), text(""));
}

/// Scenario "Objet retiré": the list of the persons replaced by another one, by a transaction
/// of the document of a copy of the directory. The new objects get new identifiers.
#[test]
fn object_removed() {
    let dir = directory_copy();
    let mut state = open_dir(&dir);
    let dupont = show(&mut state, "Dupont");
    assert_eq!(value(&state, "nom"), text("Dupont"));

    let document = state.app.as_mut().expect("app").context.document_mut();
    let personne = document.model().class_id("Personne").expect("Personne");
    let nom = document
        .model()
        .class(personne)
        .attribute("nom")
        .expect("nom");
    let (annuaire, personnes) = document.store().container(dupont).expect("container");
    let persons = document.store().children(annuaire, personnes).to_vec();
    let mut edit = document.edit();
    edit.delete_many(&persons).expect("delete");
    let other = edit
        .create_child(annuaire, personnes, Position::End, personne)
        .expect("create");
    edit.set(other, nom, Value::Str("Autre")).expect("set");
    drop(edit);
    state.prepare();
    assert_eq!(form(&state, "proprietes").object(), None);
    assert_eq!(value(&state, "nom"), text(""));
}

/// Scenario "Prénom modifié", on a copy of the directory.
#[test]
fn values_up_to_date() {
    let dir = directory_copy();
    let mut state = open_dir(&dir);
    let dupont = show(&mut state, "Dupont");
    let document = state.app.as_mut().expect("app").context.document_mut();
    let personne = document.model().class_id("Personne").expect("Personne");
    let prenom = document
        .model()
        .class(personne)
        .attribute("prenom")
        .expect("prenom");
    document
        .edit()
        .set(dupont, prenom, Value::Str("Marie-Anne"))
        .expect("set");
    state.prepare();
    assert_eq!(value(&state, "prenom"), text("Marie-Anne"));
}
