//! Tests of the layout of the window (specification `window-layout`): the tree of splits and
//! views, and the shares of the splits.

use super::*;
use crate::test_support::{
    assert_parts, directory_with_content, empty_json, layout, open_dir, open_example, split,
    split_json, table_json,
};
use crate::{AppState, Intent, TableIntent, ViewRef};

/// The identifiers of the children of a split.
fn child_ids(split: &SplitNode) -> Vec<&str> {
    split
        .children()
        .iter()
        .map(|child| match child {
            LayoutNode::Split(split) => split.id(),
            LayoutNode::View(view) => view.id(),
        })
        .collect()
}

/// Scenario "Disposition de l'annuaire".
#[test]
fn layout_of_the_directory() {
    let state = open_example();
    let LayoutNode::Split(root) = layout(&state) else {
        panic!("a split at the root")
    };
    assert_eq!(root.id(), "principal");
    assert_eq!(root.direction(), Direction::Horizontal);
    assert_eq!(child_ids(root), ["personnes", "proprietes"]);
    assert_parts(root.parts(), &[0.65, 0.35]);
    assert!(matches!(state.view("personnes"), Some(ViewRef::Table(_))));
    assert!(matches!(state.view("proprietes"), Some(ViewRef::Empty)));
    assert!(state.view("principal").is_none());
}

/// Scenario "Splits imbriqués".
#[test]
fn nested_splits() {
    let inner = split_json(
        "droite",
        "vertical",
        "",
        &[table_json("b"), table_json("c")],
    );
    let dir = directory_with_content(&split_json(
        "principal",
        "horizontal",
        "",
        &[table_json("a"), inner],
    ));
    let state = open_dir(&dir);
    let LayoutNode::Split(root) = layout(&state) else {
        panic!("a split at the root")
    };
    assert_eq!(root.direction(), Direction::Horizontal);
    assert_eq!(child_ids(root), ["a", "droite"]);
    let LayoutNode::Split(right) = &root.children()[1] else {
        panic!("a split on the right")
    };
    assert_eq!(right.direction(), Direction::Vertical);
    assert_eq!(child_ids(right), ["b", "c"]);
    let views: Vec<&str> = layout(&state).views().iter().map(|v| v.id()).collect();
    assert_eq!(views, ["a", "b", "c"]);
    let splits: Vec<&str> = layout(&state).splits().iter().map(|s| s.id()).collect();
    assert_eq!(splits, ["principal", "droite"]);
    assert_eq!(split(&state, "droite").id(), "droite");
}

/// Scenarios "Poids en pourcentages", "Poids quelconques" and "Poids absents".
#[test]
fn parts_from_the_weights() {
    let three = |weights: &str| {
        let dir = directory_with_content(&split_json(
            "principal",
            "horizontal",
            weights,
            &[table_json("a"), empty_json("b"), empty_json("c")],
        ));
        let state = open_dir(&dir);
        split(&state, "principal").parts().to_vec()
    };
    assert_parts(&three(r#""weights": [2, 1, 1],"#), &[0.5, 0.25, 0.25]);
    assert_parts(&three(""), &[1.0 / 3.0; 3]);
    // [65, 35] is the annuaire.
    assert_parts(split(&open_example(), "principal").parts(), &[0.65, 0.35]);
}

/// `normalize_parts`: the sum is 1, and wrong values are refused.
#[test]
fn normalized_parts() {
    assert_parts(
        &normalize_parts(&[1.1, 0.9], 2).expect("valid"),
        &[0.55, 0.45],
    );
    assert_parts(
        &normalize_parts(&[3.0, 1.0], 2).expect("valid"),
        &[0.75, 0.25],
    );
    // Values so large that their sum would overflow.
    assert_parts(
        &normalize_parts(&[f64::MAX, f64::MAX], 2).expect("valid"),
        &[0.5, 0.5],
    );
    // A share too small for an `f32` stays strictly positive.
    let tiny = normalize_parts(&[1e-300, 1.0], 2).expect("valid");
    assert!(tiny[0] > 0.0, "{tiny:?}");
    assert_parts(&tiny[1..], &[1.0]);
    for wrong in [
        &[1.0, 0.0][..],
        &[-0.2, 1.2],
        &[f64::NAN, 1.0],
        &[f64::INFINITY, 1.0],
        &[1.0],
        &[1.0, 1.0, 1.0],
    ] {
        assert_eq!(normalize_parts(wrong, 2), None, "{wrong:?}");
    }
}

/// The annuaire, after the intent of moving the separator of `split` to `shares`.
fn resized(split_id: &str, shares: &[f32]) -> AppState {
    let mut state = open_example();
    state.dispatch(Intent::SplitResized {
        split: split_id.into(),
        shares: shares.to_vec(),
    });
    state
}

/// Scenario "Parts reçues".
#[test]
fn parts_received() {
    let state = resized("principal", &[1.1, 0.9]);
    assert_parts(split(&state, "principal").parts(), &[0.55, 0.45]);
}

/// Scenarios "Nombre de valeurs faux" and "Valeur invalide": the parts stay.
#[test]
fn wrong_parts_received() {
    for shares in [
        &[0.3, 0.3, 0.4][..],
        &[0.0, 1.0],
        &[-0.5, 1.5],
        &[f32::NAN, 1.0],
        &[f32::INFINITY, 1.0],
    ] {
        let state = resized("principal", shares);
        assert_parts(split(&state, "principal").parts(), &[0.65, 0.35]);
    }
}

/// Scenario "Split inconnu": the layout is unchanged, and the intent names a view or a split
/// that does not exist.
#[test]
fn parts_of_an_unknown_split() {
    for id in ["lateral", "personnes"] {
        let state = resized(id, &[0.5, 0.5]);
        assert_parts(split(&state, "principal").parts(), &[0.65, 0.35]);
    }
}

/// Scenario "Deux tables des mêmes données": a sort of the view `a` does not sort `b`.
#[test]
fn intents_of_a_view_go_to_that_view() {
    let dir = directory_with_content(&split_json(
        "principal",
        "horizontal",
        "",
        &[table_json("a"), table_json("b")],
    ));
    let mut state = open_dir(&dir);
    // The same columns in both tables: those of the presentation of `Personne`.
    let prenom = {
        let Some(ViewRef::Table(table)) = state.view("a") else {
            panic!("a table a")
        };
        table
            .columns()
            .iter()
            .position(|c| c.header() == "Prénom")
            .expect("Prénom")
    };
    let first_names = |state: &AppState, view: &str| {
        let Some(ViewRef::Table(table)) = state.view(view) else {
            panic!("a table {view}")
        };
        (0..table.row_count())
            .map(|row| {
                table
                    .cell(row, prenom)
                    .text()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<Vec<_>>()
    };
    let before = first_names(&state, "b");
    state.dispatch(Intent::table(
        "a",
        TableIntent::SortBy {
            column: prenom,
            add: false,
        },
    ));
    state.prepare();
    // `a` is sorted by `prenom` (the order itself is checked by the tests of the table),
    // `b` keeps the sort of the presentation of `Personne`.
    assert_ne!(first_names(&state, "a"), before);
    assert_eq!(first_names(&state, "b"), before, "b unchanged");
    let indicator = |view: &str| {
        let Some(ViewRef::Table(table)) = state.view(view) else {
            panic!("{view}")
        };
        table.sort_indicator(prenom)
    };
    // The sort of the presentation is `nom` then `prenom`: `prenom` second in `b`, alone in
    // `a` (in either direction: a click on a sorted column turns its direction).
    assert!(
        matches!(indicator("a").as_deref(), Some("⏶" | "⏷")),
        "{:?}",
        indicator("a")
    );
    assert_eq!(indicator("b").as_deref(), Some("⏶ 2"));
    // An intent of a view that does not exist, or that is not a table, changes nothing.
    for view in ["zzz", "principal"] {
        state.dispatch(Intent::table(view, TableIntent::SetFilter("dup".into())));
    }
    state.prepare();
    assert_eq!(first_names(&state, "b"), before);
}
