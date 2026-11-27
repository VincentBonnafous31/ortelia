//! Tests of the table view (specification `table-view`), on `examples/annuaire` and on
//! applications written in temporary folders.

use ortelia_config::{Align, AppFile, CheckedApp, SortOrder};
use ortelia_model::metamodel::{AttrId, Model, ModelFile};
use ortelia_model::value::Value;
use ortelia_service::app::AppContext;

use super::{TableIntent, TableView};
use crate::format::Cell;
use crate::labels::LabelLookup;
use crate::session::{ColumnSession, SortSession, TableSession};
use crate::test_support::{directory, directory_with, example, lock_log_tests, numbered_persons};

/// The table of an open application, in its first language, prepared.
fn table(context: &AppContext) -> TableView {
    table_of(context.app(), context)
}

/// The table of `app` (a variant of the application of `context`), prepared.
fn table_of(app: &CheckedApp, context: &AppContext) -> TableView {
    let model = context.document().model();
    let mut lookup = LabelLookup::new(&app.labels, model, &app.languages[0]);
    let mut table = TableView::new(app, app.window.content.table_views()[0], model, &mut lookup);
    table.prepare(context.document());
    table
}

/// The names (`nom`) of the rows shown.
fn names(table: &TableView, context: &AppContext) -> Vec<String> {
    let store = context.document().store();
    let nom = attr(context, "nom");
    table
        .rows
        .iter()
        .map(|&i| match store.get(table.base[i as usize], nom) {
            Value::Str(text) => text.to_owned(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// An attribute of `Personne`.
fn attr(context: &AppContext, name: &str) -> AttrId {
    let model = context.document().model();
    let personne = model.class_id("Personne").expect("Personne");
    model.class(personne).attribute(name).expect("attribute")
}

/// The names of the attributes of the columns, in the order shown.
fn column_names(table: &TableView, model: &Model) -> Vec<String> {
    table
        .columns
        .iter()
        .map(|c| model.attribute(c.attr).name().to_owned())
        .collect()
}

/// The position of the column of `name`.
fn position(table: &TableView, context: &AppContext, name: &str) -> usize {
    let attr = attr(context, name);
    table
        .columns
        .iter()
        .position(|c| c.attr == attr)
        .expect("column")
}

fn apply(table: &mut TableView, context: &AppContext, intent: TableIntent) {
    table.apply(intent);
    table.prepare(context.document());
}

/// Clicks on the header of `name`, with Shift if `add`.
fn click(table: &mut TableView, context: &AppContext, name: &str, add: bool) {
    let column = position(table, context, name);
    apply(table, context, TableIntent::SortBy { column, add });
}

/// The annuaire with the given sort, by attribute name.
fn sorted_by(table: &mut TableView, context: &AppContext, sort: &[(&str, SortOrder)]) {
    table.sort = sort.iter().map(|&(n, o)| (attr(context, n), o)).collect();
    table.prepare(context.document());
}

fn filter(table: &mut TableView, context: &AppContext, text: &str) {
    apply(table, context, TableIntent::SetFilter(text.to_owned()));
}

// --- Rows and columns ---

/// Scenario "Personnes de l'annuaire".
#[test]
fn persons_of_the_directory() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[]);
    let names = names(&table, &context);
    assert_eq!(names.len(), 10);
    assert_eq!(names[0], "Dupont");
    assert_eq!(names[9], "Garcia");
    let read = table.read(context.document());
    assert_eq!(read.cell(0, 1), Cell::Text("Marie".into()));
    assert_eq!((read.row_count(), read.total_count()), (10, 10));
}

/// Scenario "Colonnes configurées".
#[test]
fn configured_columns() {
    let context = directory();
    let mut app = context.app().clone();
    let personne = app.window.content.table_views()[0].class;
    let table_def = app
        .presentations
        .get_mut(&personne)
        .and_then(|p| p.table.as_mut())
        .expect("table");
    table_def.columns.truncate(3);
    table_def.sort.clear();
    let table = table_of(&app, &context);
    assert_eq!(
        column_names(&table, context.document().model()),
        ["nom", "prenom", "date_naissance"]
    );
    // The widths of the presentation, or of the type.
    let widths: Vec<f32> = table.columns.iter().map(|c| c.width).collect();
    assert_eq!(widths, [140.0, 120.0, 100.0]);
}

/// Scenario "Annuaire sans présentation" (deduced columns), and scenario "Alignements de
/// l'annuaire" of `value-formats`.
#[test]
fn columns_deduced_from_the_model() {
    let context = directory();
    let mut app = context.app().clone();
    app.presentations.clear();
    let table = table_of(&app, &context);
    let model = context.document().model();
    assert_eq!(
        column_names(&table, model),
        [
            "nom",
            "prenom",
            "date_naissance",
            "nombre_enfants",
            "taille",
            "contact_autorise",
            "situation",
            "inscription",
            "notes"
        ]
    );
    assert!(table.sort.is_empty());
    let aligns: Vec<(String, Align)> = table
        .columns
        .iter()
        .map(|c| (model.attribute(c.attr).name().to_owned(), c.align))
        .collect();
    for (name, align) in aligns {
        let expected = match name.as_str() {
            "nombre_enfants" | "taille" => Align::Right,
            "contact_autorise" | "date_naissance" | "inscription" => Align::Center,
            _ => Align::Left,
        };
        assert_eq!(align, expected, "{name}");
    }
    // Absent notes give an empty cell (scenario "Notes absentes" of `value-formats`).
    let read = table.read(context.document());
    let notes = table.columns.len() - 1;
    let martin = names(&table, &context)
        .iter()
        .position(|n| n == "Martin")
        .expect("Martin");
    assert_eq!(read.cell(martin, notes), Cell::Empty);
}

/// A model where `Employe` inherits from `Personne`, and `app.json` texts of it.
const INHERITED_MODEL: &str = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p",
    "classes": [
      { "name": "Racine", "references": [
          { "name": "employes", "type": "Employe", "upperBound": -1 } ] },
      { "name": "Personne", "attributes": [
          { "name": "nom", "type": "String" }, { "name": "prenom", "type": "String" } ] },
      { "name": "Employe", "superClass": "Personne", "attributes": [
          { "name": "matricule", "type": "Int" } ] } ] } ] }"#;

/// An application of the inherited model, with the given presentations and labels.
fn inherited_app(presentations: &str, labels: &str) -> (Model, CheckedApp) {
    let model =
        Model::compile(&[ModelFile::parse(INHERITED_MODEL).expect("parse")]).expect("compile");
    let text = format!(
        r#"{{ "formatVersion": "0.5", "id": "a", "name": "A", "languages": ["fr"],
             "labels": [ {{ "lang": "fr", "entries": {labels} }} ],
             "model": {{ "files": [], "root": "Racine" }},
             "presentations": {presentations},
             "window": {{ "content": {{ "_type_": "View", "id": "v",
                 "widget": {{ "_type_": "Table" }}, "data": "/employes" }} }} }}"#
    );
    let app = AppFile::parse(&text)
        .expect("parse")
        .check(&model)
        .expect("check");
    (model, app)
}

/// Scenario "Présentation héritée", and the label of a subclass (scenario "Sous-classe qui
/// renomme" of `labels`).
#[test]
fn inherited_presentation() {
    let _lock = lock_log_tests();
    let (model, app) = inherited_app(
        r#"[ { "class": "Personne", "table": { "columns": [ { "path": "prenom" } ] } } ]"#,
        r#"[ { "key": "Personne.prenom", "text": "Prénom" },
             { "key": "Employe.prenom", "text": "Prénom d'usage" } ]"#,
    );
    let mut lookup = LabelLookup::new(&app.labels, &model, "fr");
    let table = TableView::new(
        &app,
        app.window.content.table_views()[0],
        &model,
        &mut lookup,
    );
    assert_eq!(column_names(&table, &model), ["prenom"]);
    assert_eq!(table.columns[0].header, "Prénom d'usage");
    assert!(lookup.missing().is_empty());
}

/// A presentation without table: the columns of the model, inherited attributes first.
#[test]
fn presentation_without_table() {
    let _lock = lock_log_tests();
    let (model, app) = inherited_app(r#"[ { "class": "Employe" } ]"#, "[]");
    let mut lookup = LabelLookup::new(&app.labels, &model, "fr");
    let table = TableView::new(
        &app,
        app.window.content.table_views()[0],
        &model,
        &mut lookup,
    );
    assert_eq!(column_names(&table, &model), ["nom", "prenom", "matricule"]);
    assert_eq!(table.columns[2].header, "#Employe.matricule#");
}

/// Scenario "En-tête avec unité", and scenario "Première langue" of `labels`.
#[test]
fn header_with_a_unit() {
    let context = directory();
    let table = table(&context);
    let header = |name| {
        table.columns[position(&table, &context, name)]
            .header
            .clone()
    };
    assert_eq!(header("taille"), "Taille (m)");
    assert_eq!(header("nom"), "Nom");
    assert_eq!(header("prenom"), "Prénom");
}

/// Scenario "Défilement d'une grande table": only the rows asked are formatted.
#[test]
fn only_visible_rows_are_formatted() {
    let dir = directory_with(&example("annuaire/app.json"), &numbered_persons(50_000));
    let context = AppContext::open(&dir.path().join("app.json")).expect("open");
    let table = table(&context);
    let read = table.read(context.document());
    assert_eq!(read.total_count(), 50_000);
    assert_eq!(table.formatted_cells.get(), 0, "the sort formats nothing");
    for row in 120..=160 {
        for column in 0..table.columns.len() {
            read.cell(row, column);
        }
    }
    assert_eq!(table.formatted_cells.get(), 41 * table.columns.len());
}

// --- Sort ---

/// Scenario "Tri initial de l'annuaire": by `nom`, then `prenom`.
#[test]
fn initial_sort() {
    let context = directory();
    let table = table(&context);
    let names = names(&table, &context);
    assert_eq!(names.first().map(String::as_str), Some("Bernard"));
    assert_eq!(names.last().map(String::as_str), Some("Robert"));
}

/// Scenario "Cycle du tri".
#[test]
fn sort_cycle() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[]);
    let list_order = names(&table, &context);
    let mut ascending = list_order.clone();
    ascending.sort();
    click(&mut table, &context, "nom", false);
    assert_eq!(names(&table, &context), ascending);
    click(&mut table, &context, "nom", false);
    let descending: Vec<String> = ascending.iter().rev().cloned().collect();
    assert_eq!(names(&table, &context), descending);
    click(&mut table, &context, "nom", false);
    assert_eq!(names(&table, &context), list_order);
    assert!(table.sort.is_empty());
    click(&mut table, &context, "nom", false);
    assert_eq!(names(&table, &context), ascending);
}

/// Scenario "Remplacement du tri".
#[test]
fn sort_replaced() {
    let context = directory();
    let mut table = table(&context);
    assert_eq!(table.sort.len(), 2);
    click(&mut table, &context, "date_naissance", false);
    assert_eq!(
        table.sort,
        [(attr(&context, "date_naissance"), SortOrder::Asc)]
    );
    assert_eq!(names(&table, &context)[0], "Richard");
}

/// Scenario "Ajout d'un second critère".
#[test]
fn second_criterion_added() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[("situation", SortOrder::Asc)]);
    click(&mut table, &context, "nom", true);
    assert_eq!(
        table.sort,
        [
            (attr(&context, "situation"), SortOrder::Asc),
            (attr(&context, "nom"), SortOrder::Asc)
        ]
    );
    assert_eq!(
        names(&table, &context),
        [
            "Garcia", "Martin", "Petit", // single
            "Dupont", "Durand", "Moreau", // married
            "Robert", "Bernard", "Richard", "Lefebvre"
        ]
    );
}

/// Scenario "Retrait d'un critère", and Shift+click that advances a criterion in place.
#[test]
fn criterion_removed() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(
        &mut table,
        &context,
        &[("situation", SortOrder::Asc), ("nom", SortOrder::Asc)],
    );
    click(&mut table, &context, "nom", true);
    assert_eq!(
        table.sort,
        [
            (attr(&context, "situation"), SortOrder::Asc),
            (attr(&context, "nom"), SortOrder::Desc)
        ]
    );
    click(&mut table, &context, "nom", true);
    assert_eq!(table.sort, [(attr(&context, "situation"), SortOrder::Asc)]);
}

/// Scenario "Deux critères", and a single criterion, shown without rank.
#[test]
fn sort_indicators() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(
        &mut table,
        &context,
        &[("situation", SortOrder::Asc), ("nom", SortOrder::Desc)],
    );
    let indicators: Vec<Option<String>> = (0..table.columns.len())
        .map(|c| table.read(context.document()).sort_indicator(c))
        .collect();
    let at = |name| indicators[position(&table, &context, name)].clone();
    assert_eq!(at("situation").as_deref(), Some("⏶ 1"));
    assert_eq!(at("nom").as_deref(), Some("⏷ 2"));
    assert_eq!(indicators.iter().flatten().count(), 2);
    sorted_by(&mut table, &context, &[("nom", SortOrder::Desc)]);
    let nom = position(&table, &context, "nom");
    assert_eq!(
        table
            .read(context.document())
            .sort_indicator(nom)
            .as_deref(),
        Some("⏷")
    );
}

/// Scenario "Valeur absente en dernier": Lefebvre has no situation.
#[test]
fn absent_value_last() {
    let context = directory();
    let mut table = table(&context);
    for order in [SortOrder::Asc, SortOrder::Desc] {
        sorted_by(&mut table, &context, &[("situation", order)]);
        assert_eq!(
            names(&table, &context).last().map(String::as_str),
            Some("Lefebvre"),
            "{order:?}"
        );
    }
}

/// Scenario "Énumérés dans l'ordre du modèle", stable inside each literal.
#[test]
fn enums_in_the_order_of_the_model() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[("situation", SortOrder::Asc)]);
    assert_eq!(
        names(&table, &context),
        [
            "Martin", "Petit", "Garcia", // single, in the order of the list
            "Dupont", "Durand", "Moreau", // married
            "Robert", "Bernard", "Richard", "Lefebvre"
        ]
    );
}

/// Scenario "Égalité": false before true, and the order of the list kept in each group;
/// Moreau, without value, last.
#[test]
fn stable_sort() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(
        &mut table,
        &context,
        &[("contact_autorise", SortOrder::Asc)],
    );
    assert_eq!(
        names(&table, &context),
        [
            "Martin", "Richard", "Garcia", // false
            "Dupont", "Bernard", "Petit", "Robert", "Durand", "Lefebvre", // true
            "Moreau"
        ]
    );
}

/// Numbers, dates and date-times in their natural order.
#[test]
fn natural_order_of_values() {
    let context = directory();
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[("taille", SortOrder::Desc)]);
    assert_eq!(names(&table, &context)[0], "Moreau");
    assert_eq!(names(&table, &context)[9], "Lefebvre", "absent last");
    sorted_by(&mut table, &context, &[("inscription", SortOrder::Desc)]);
    assert_eq!(names(&table, &context)[0], "Garcia");
    sorted_by(&mut table, &context, &[("nombre_enfants", SortOrder::Desc)]);
    assert_eq!(names(&table, &context)[0], "Richard");
}

/// Scenario "Accents et casse".
#[test]
fn accents_and_case() {
    let persons = r#"[ { "nom": "Émile" }, { "nom": "de La Tour" }, { "nom": "Dupont" },
                       { "nom": "éric" }, { "nom": "Durand" } ]"#;
    let dir = directory_with(&example("annuaire/app.json"), persons);
    let context = AppContext::open(&dir.path().join("app.json")).expect("open");
    let mut table = table(&context);
    sorted_by(&mut table, &context, &[("nom", SortOrder::Asc)]);
    assert_eq!(
        names(&table, &context),
        ["de La Tour", "Dupont", "Durand", "Émile", "éric"]
    );
}

// --- Filter ---

/// Scenario "Deux mots dans deux colonnes".
#[test]
fn two_words_in_two_columns() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "marie dup");
    assert_eq!(names(&table, &context), ["Dupont"]);
}

/// Scenario "Recherche dans le texte affiché".
#[test]
fn search_in_the_text_shown() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "1985");
    assert_eq!(names(&table, &context), ["Dupont"]);
    // `15/01/2024` is shown, not `2024-01-15`.
    filter(&mut table, &context, "2024-01");
    assert!(names(&table, &context).is_empty());
}

/// Scenario "Recherche dans le libellé d'un énuméré".
#[test]
fn search_in_the_label_of_an_enum() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "pacsé");
    assert_eq!(names(&table, &context), ["Robert"]);
    filter(&mut table, &context, "PACSE");
    assert_eq!(names(&table, &context), ["Robert"]);
}

/// Scenario "Cases à cocher ignorées".
#[test]
fn check_boxes_ignored() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "true");
    assert!(names(&table, &context).is_empty());
}

/// An empty filter, or made of spaces, keeps all the rows.
#[test]
fn empty_filter() {
    let context = directory();
    let mut table = table(&context);
    for text in ["", "   "] {
        filter(&mut table, &context, text);
        assert_eq!(table.rows.len(), 10, "{text:?}");
    }
}

/// Scenario "Compteur", and the filter applied to the sorted order.
#[test]
fn filter_and_sort_combined() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "marié");
    assert_eq!(names(&table, &context), ["Dupont", "Durand", "Moreau"]);
    assert_eq!(table.read(context.document()).counter(), "3 / 10");
    // `nom` was ascending: a click makes it descending.
    click(&mut table, &context, "nom", false);
    assert_eq!(names(&table, &context), ["Moreau", "Durand", "Dupont"]);
}

// --- Widths, order, revisions ---

/// Scenario "Largeur qui suit sa colonne".
#[test]
fn width_follows_its_column() {
    let context = directory();
    let mut table = table(&context);
    let widths: Vec<f32> = table.columns.iter().map(|c| c.width).collect();
    apply(
        &mut table,
        &context,
        TableIntent::ResizeColumn {
            column: 0,
            width: 200.0,
        },
    );
    apply(
        &mut table,
        &context,
        TableIntent::MoveColumn { from: 0, to: 2 },
    );
    assert_eq!(position(&table, &context, "nom"), 2);
    assert_eq!(table.columns[2].width, 200.0);
    let mut expected = widths.clone();
    expected.remove(0);
    expected.insert(2, 200.0);
    let now: Vec<f32> = table.columns.iter().map(|c| c.width).collect();
    assert_eq!(now, expected);
}

/// A width beyond the bounds of `app.json` is brought back to them: it could not be written
/// there.
#[test]
fn width_kept_within_bounds() {
    let context = directory();
    let mut table = table(&context);
    for (width, kept) in [(5.0, 20.0), (5_000.0, 2_000.0)] {
        apply(
            &mut table,
            &context,
            TableIntent::ResizeColumn { column: 0, width },
        );
        assert_eq!(table.columns[0].width, kept);
    }
}

/// Scenario "Déplacement d'une colonne": the rows, the sort and the filter do not change; the
/// layout number does.
#[test]
fn column_moved() {
    let context = directory();
    let mut table = table(&context);
    filter(&mut table, &context, "a");
    let rows = names(&table, &context);
    let sort = table.sort.clone();
    let computations = table.row_computations;
    apply(
        &mut table,
        &context,
        TableIntent::MoveColumn { from: 2, to: 0 },
    );
    let model = context.document().model();
    assert_eq!(
        column_names(&table, model)[..3],
        ["date_naissance", "nom", "prenom"]
    );
    assert_eq!(names(&table, &context), rows);
    assert_eq!(table.sort, sort);
    assert_eq!(table.layout, 1);
    assert_eq!(table.row_computations, computations);
    // A move to its own place changes nothing.
    apply(
        &mut table,
        &context,
        TableIntent::MoveColumn { from: 1, to: 1 },
    );
    assert_eq!(table.layout, 1);
}

/// Scenario "Modification d'une valeur triée": a transaction of the document renames Bernard.
#[test]
fn sorted_value_modified() {
    let mut context = directory();
    let mut table = table(&context);
    assert_eq!(names(&table, &context)[0], "Bernard");
    let nom = attr(&context, "nom");
    let bernard = table.base[table.rows[0] as usize];
    context
        .document_mut()
        .edit()
        .set(bernard, nom, Value::Str("Zola"))
        .expect("set");
    table.prepare(context.document());
    assert_eq!(
        names(&table, &context).last().map(String::as_str),
        Some("Zola")
    );
}

/// Scenario "Pas de recalcul sans changement".
#[test]
fn no_computation_without_change() {
    let context = directory();
    let mut table = table(&context);
    for _ in 0..5 {
        table.prepare(context.document());
    }
    assert_eq!(table.row_computations, 1);
    filter(&mut table, &context, "dup");
    table.prepare(context.document());
    assert_eq!(table.row_computations, 2);
}

// --- Session state ---

/// The table of the annuaire, restored from `saved`, then prepared.
fn restored(context: &AppContext, saved: &TableSession) -> TableView {
    let model = context.document().model();
    let app = context.app();
    let mut lookup = LabelLookup::new(&app.labels, model, &app.languages[0]);
    let mut table = TableView::new(app, app.window.content.table_views()[0], model, &mut lookup);
    table.restore(saved, model);
    table.prepare(context.document());
    table
}

fn columns(saved: &[(&str, Option<f32>)]) -> Vec<ColumnSession> {
    saved
        .iter()
        .map(|&(path, width)| ColumnSession {
            path: path.to_owned(),
            width,
        })
        .collect()
}

fn sort(saved: &[(&str, SortOrder)]) -> Option<Vec<SortSession>> {
    Some(
        saved
            .iter()
            .map(|&(path, order)| SortSession {
                path: path.to_owned(),
                order,
            })
            .collect(),
    )
}

/// The width of the column of `name`.
fn width(table: &TableView, context: &AppContext, name: &str) -> f32 {
    table.columns[position(table, context, name)].width
}

/// The columns of the presentation of `Personne`, in their order.
const PRESENTATION_COLUMNS: [&str; 8] = [
    "nom",
    "prenom",
    "date_naissance",
    "nombre_enfants",
    "taille",
    "contact_autorise",
    "situation",
    "inscription",
];

/// Scenario "Disposition modifiée" of `session-state`, for the table: what is computed after
/// interactions.
#[test]
fn session_of_a_modified_layout() {
    let context = directory();
    let mut table = table(&context);
    let from = position(&table, &context, "date_naissance");
    apply(
        &mut table,
        &context,
        TableIntent::MoveColumn { from, to: 0 },
    );
    let column = position(&table, &context, "nom");
    apply(
        &mut table,
        &context,
        TableIntent::ResizeColumn {
            column,
            width: 200.0,
        },
    );
    click(&mut table, &context, "situation", false);
    click(&mut table, &context, "nom", true);
    click(&mut table, &context, "nom", true);
    filter(&mut table, &context, "dup");
    let session = table.session(context.document().model());
    let names: Vec<&str> = session.columns.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        names,
        [
            "date_naissance",
            "nom",
            "prenom",
            "nombre_enfants",
            "taille",
            "contact_autorise",
            "situation",
            "inscription"
        ]
    );
    assert_eq!(session.columns[1].width, Some(200.0));
    assert_eq!(session.columns[2].width, Some(120.0));
    assert_eq!(
        session.sort,
        sort(&[("situation", SortOrder::Asc), ("nom", SortOrder::Desc)])
    );
    assert_eq!(session.filter, "dup");
}

/// Scenario "Disposition inchangée" of `session-state`, for the table.
#[test]
fn session_of_an_unchanged_layout() {
    let context = directory();
    let table = table(&context);
    let session = table.session(context.document().model());
    let names: Vec<&str> = session.columns.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(names, PRESENTATION_COLUMNS);
    assert_eq!(session.columns[0].width, Some(140.0));
    assert_eq!(
        session.sort,
        sort(&[("nom", SortOrder::Asc), ("prenom", SortOrder::Asc)])
    );
    assert_eq!(session.filter, "");
}

/// What `session` gives, `restore` restores.
#[test]
fn session_restored() {
    let context = directory();
    let mut table = table(&context);
    let from = position(&table, &context, "situation");
    apply(
        &mut table,
        &context,
        TableIntent::MoveColumn { from, to: 1 },
    );
    let column = position(&table, &context, "prenom");
    apply(
        &mut table,
        &context,
        TableIntent::ResizeColumn {
            column,
            width: 250.0,
        },
    );
    click(&mut table, &context, "taille", false);
    filter(&mut table, &context, "dup");
    let session = table.session(context.document().model());
    let again = restored(&context, &session);
    assert_eq!(again.session(context.document().model()), session);
    assert_eq!(names(&again, &context), names(&table, &context));
}

/// Scenario "Tri de l'état de session" of `table-view`.
#[test]
fn sort_of_the_session_state() {
    let context = directory();
    let saved = TableSession {
        sort: sort(&[("nom", SortOrder::Desc)]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    let names = names(&table, &context);
    assert_eq!(names[0], "Robert");
    assert_eq!(names[9], "Bernard");
}

/// Scenario "Tri retiré par l'utilisateur" of `table-view`; without the key `sort`, the sort of
/// the presentation stays.
#[test]
fn sort_removed_by_the_user() {
    let context = directory();
    let saved = TableSession {
        sort: Some(Vec::new()),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert!(table.sort.is_empty());
    assert_eq!(names(&table, &context)[0], "Dupont");
    let table = restored(&context, &TableSession::default());
    assert_eq!(names(&table, &context)[0], "Bernard");
}

/// Scenario "Largeur de l'état de session" of `table-view`.
#[test]
fn width_of_the_session_state() {
    let context = directory();
    let saved = TableSession {
        columns: columns(&[("nom", Some(180.0))]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert_eq!(width(&table, &context, "nom"), 180.0);
}

/// Scenario "Colonne disparue".
#[test]
fn column_gone() {
    let context = directory();
    let saved = TableSession {
        columns: columns(&[
            ("prenom", Some(150.0)),
            ("surnom", Some(90.0)),
            ("nom", Some(180.0)),
        ]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    let names = column_names(&table, context.document().model());
    assert_eq!(names.len(), 8);
    assert_eq!(names[..2], ["prenom", "nom"]);
    assert_eq!(width(&table, &context, "prenom"), 150.0);
    assert_eq!(width(&table, &context, "nom"), 180.0);
}

/// Scenario "Tri sur une colonne disparue".
#[test]
fn sort_on_a_column_gone() {
    let context = directory();
    let saved = TableSession {
        sort: sort(&[("surnom", SortOrder::Asc), ("nom", SortOrder::Desc)]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert_eq!(table.sort, [(attr(&context, "nom"), SortOrder::Desc)]);
}

/// Scenario "Colonne en double"; for a criterion twice too, the first counts.
#[test]
fn column_twice() {
    let context = directory();
    let saved = TableSession {
        columns: columns(&[("nom", Some(180.0)), ("nom", Some(300.0))]),
        sort: sort(&[("prenom", SortOrder::Desc), ("prenom", SortOrder::Asc)]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert_eq!(
        column_names(&table, context.document().model()),
        PRESENTATION_COLUMNS
    );
    assert_eq!(width(&table, &context, "nom"), 180.0);
    assert_eq!(table.sort, [(attr(&context, "prenom"), SortOrder::Desc)]);
}

/// Scenario "Colonne nouvelle".
#[test]
fn new_columns() {
    let context = directory();
    let saved = TableSession {
        columns: columns(&[("prenom", Some(150.0)), ("nom", Some(180.0))]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert_eq!(
        column_names(&table, context.document().model()),
        [
            "prenom",
            "nom",
            "date_naissance",
            "nombre_enfants",
            "taille",
            "contact_autorise",
            "situation",
            "inscription"
        ]
    );
    // The widths of the presentation, or of the type, for the others.
    let fresh = table_of(context.app(), &context);
    for name in &PRESENTATION_COLUMNS[2..] {
        assert_eq!(width(&table, &context, name), width(&fresh, &context, name));
    }
}

/// Scenario "Largeur hors bornes"; a width missing or not finite is ignored.
#[test]
fn width_out_of_bounds() {
    let context = directory();
    let saved = TableSession {
        columns: columns(&[
            ("nom", Some(5.0)),
            ("prenom", Some(9000.0)),
            ("taille", Some(f32::NAN)),
            ("situation", None),
        ]),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    let fresh = table_of(context.app(), &context);
    assert_eq!(width(&table, &context, "nom"), 20.0);
    assert_eq!(width(&table, &context, "prenom"), 2000.0);
    for name in ["taille", "situation"] {
        assert_eq!(width(&table, &context, name), width(&fresh, &context, name));
    }
}

/// The filter is restored.
#[test]
fn filter_restored() {
    let context = directory();
    let saved = TableSession {
        filter: "dup".to_owned(),
        ..TableSession::default()
    };
    let table = restored(&context, &saved);
    assert_eq!(names(&table, &context), ["Dupont"]);
    assert_eq!(table.read(context.document()).filter(), "dup");
}
