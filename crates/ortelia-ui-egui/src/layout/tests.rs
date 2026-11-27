//! Tests of the drawing of the layout (specification `window-layout`), without a screen.
//!
//! The separator of egui_tiles has only an unnamed accessibility node (role `Unknown`): its
//! position is computed from the bounds of the table on its left (its filter field and its
//! counter), never written as a number.

use egui::Role;
use egui_kittest::kittest::{By, NodeT as _, Queryable};
use egui_kittest::{Harness, HarnessBuilder, Node};
use ortelia_presentation::{AppState, Intent, MainContent, TableIntent};

use super::VIEW_MARGIN;
use crate::OrteliaApp;
use crate::tests::{
    Recorder, WINDOW, assert_near, directory, directory_copy, directory_with, drag, example,
    filter_field, lock_app_tests, recorder_of,
};

/// A harness with the window of the annuaire that draws `state` and records its intents. A
/// short time step: the two clicks of a double-click come within its delay.
fn layout_recorder(state: AppState) -> Harness<'static, Recorder> {
    recorder_of(
        HarnessBuilder::default()
            .with_size(WINDOW)
            .with_step_dt(1.0 / 60.0),
        state,
    )
}

/// The horizontal extent of the tile of the table whose filter field is `filter` and counter
/// `counter`: from the one to the other, with the margins of the view.
fn tile_of(filter: egui::Rect, counter: egui::Rect) -> egui::Rangef {
    let margin = f32::from(VIEW_MARGIN);
    egui::Rangef::new(filter.left() - margin, counter.right() + margin)
}

/// The horizontal extent of the tile of the only table of the window.
fn table_tile<State>(harness: &Harness<'_, State>) -> egui::Rangef {
    tile_of(
        filter_field(harness).rect(),
        harness.get_by_label("10 / 10").rect(),
    )
}

/// The middle of the separator on the right of the only table of the window: half the gap of
/// one point further than its tile.
fn separator<State>(harness: &Harness<'_, State>) -> egui::Pos2 {
    egui::pos2(table_tile(harness).max + 0.5, WINDOW.y / 2.0)
}

/// The intents of moved separators recorded, as the interface gives them.
fn split_intents(harness: &Harness<'_, Recorder>) -> Vec<Intent> {
    harness
        .state()
        .intents
        .iter()
        .filter(|intent| matches!(intent, Intent::SplitResized { .. }))
        .cloned()
        .collect()
}

/// The split of each intent of moved separator recorded, and its number of shares.
fn resized_splits(harness: &Harness<'_, Recorder>) -> Vec<(String, usize)> {
    harness
        .state()
        .intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::SplitResized { split, shares } => Some((split.clone(), shares.len())),
            _ => None,
        })
        .collect()
}

/// The parts of the split `principal` of `state`.
fn principal_parts(state: &AppState) -> Vec<f32> {
    let MainContent::Layout(layout) = state.content() else {
        panic!("no application")
    };
    layout
        .split("principal")
        .expect("principal")
        .parts()
        .to_vec()
}

/// The parts of the split `principal` once the presentation has received the intents of moved
/// separators recorded: the interface gives the shares of egui_tiles, of any total, and the
/// tests do not compute the parts themselves.
fn parts_received(harness: &mut Harness<'_, Recorder>) -> Vec<f32> {
    let intents = split_intents(harness);
    let state = &mut harness.state_mut().state;
    for intent in intents {
        state.dispatch(intent);
    }
    principal_parts(state)
}

/// Scenario "Table de l'annuaire à gauche".
#[test]
fn table_of_the_directory_on_the_left() {
    let mut harness = layout_recorder(directory());
    harness.run();
    let tile = table_tile(&harness);
    assert_near(tile.min, 0.0);
    assert_near(tile.span(), 650.0);
}

/// A view of the persons in a table, as JSON.
fn table(id: &str) -> String {
    format!(
        r#"{{ "_type_": "View", "id": "{id}", "widget": {{ "_type_": "Table" }},
             "data": "/annuaire/personnes" }}"#
    )
}

/// The annuaire whose window has the content `content`, given as JSON.
fn directory_with_content(content: &str) -> AppState {
    let mut app: serde_json::Value =
        serde_json::from_str(&example("annuaire/app.json")).expect("app.json");
    app["window"]["content"] = serde_json::from_str(content).expect("content");
    directory_with(
        &serde_json::to_string(&app).expect("JSON"),
        &example("annuaire/personnes.json"),
    )
}

/// Scenario "Split vertical imbriqué".
#[test]
fn nested_vertical_split() {
    let content = format!(
        r#"{{ "_type_": "Split", "id": "principal", "direction": "horizontal", "children": [
              {},
              {{ "_type_": "Split", "id": "droite", "direction": "vertical", "weights": [1, 3],
                 "children": [ {}, {} ] }} ] }}"#,
        table("a"),
        table("b"),
        table("c")
    );
    let mut harness = layout_recorder(directory_with_content(&content));
    harness.run();

    // The three tables, by their filter fields and counters, from left to right and from top
    // to bottom.
    let by_position = |a: &egui::Rect, b: &egui::Rect| {
        (a.left(), a.top())
            .partial_cmp(&(b.left(), b.top()))
            .expect("finite")
    };
    let mut filters: Vec<egui::Rect> = harness
        .get_all_by_role(Role::TextInput)
        .map(|n| n.rect())
        .collect();
    filters.sort_by(by_position);
    let mut counters: Vec<egui::Rect> = harness
        .get_all_by_label("10 / 10")
        .map(|n| n.rect())
        .collect();
    counters.sort_by(by_position);
    let [a, b, c] = [0, 1, 2].map(|i| tile_of(filters[i], counters[i]));

    // `a` on the left, `b` and `c` on the right, one above the other; the split shares its
    // width equally.
    assert_near(a.min, 0.0);
    assert_near(a.span(), WINDOW.x / 2.0);
    assert!(a.max < b.min, "{a:?} {b:?}");
    assert_near(b.min, c.min);
    assert_near(b.max, WINDOW.x);
    // The top of `b` and of `a` is that of the window; `c` starts at a quarter of its height:
    // `b` has a quarter of it, `c` three quarters, three times more.
    let [top_a, top_b, top_c] = [0, 1, 2].map(|i| filters[i].top());
    assert_near(top_a, top_b);
    assert_near(top_c - top_b, WINDOW.y / 4.0);
    // `a` reaches below the top of `c`: its last row (all the rows fit in its height) is drawn
    // under it.
    let last_row_of_a = harness
        .get_all_by_label("Robert")
        .map(|n| n.rect())
        .find(|r| a.contains(r.center().x))
        .expect("the last row of a");
    assert!(last_row_of_a.top() > top_c, "{last_row_of_a:?}");
}

/// Two tables side by side: a header dropped on a header of the other table moves no column,
/// a header dropped in its own table moves its column.
#[test]
fn header_dropped_on_another_table() {
    let content = format!(
        r#"{{ "_type_": "Split", "id": "principal", "direction": "horizontal",
             "children": [ {}, {} ] }}"#,
        table("a"),
        table("b")
    );
    let mut harness = layout_recorder(directory_with_content(&content));
    harness.run();
    // The centres of the headers of `label`, of `a` then of `b`.
    let headers = |harness: &Harness<'_, Recorder>, label: &str| -> Vec<egui::Pos2> {
        let mut centres: Vec<egui::Pos2> = harness
            .get_all_by_role_and_label(Role::ColumnHeader, label)
            .map(|n| n.rect().center())
            .collect();
        centres.sort_by(|p, q| p.x.total_cmp(&q.x));
        centres
    };
    let moves = |harness: &Harness<'_, Recorder>| -> Vec<(String, TableIntent)> {
        harness
            .state()
            .intents
            .iter()
            .filter_map(|intent| match intent {
                Intent::Table {
                    view,
                    intent: intent @ TableIntent::MoveColumn { .. },
                } => Some((view.clone(), intent.clone())),
                _ => None,
            })
            .collect()
    };
    let (from, to) = (headers(&harness, "Né le")[0], headers(&harness, "Nom")[1]);
    drag(&mut harness, from, to);
    assert_eq!(moves(&harness), []);
    let (from, to) = (headers(&harness, "Né le")[1], headers(&harness, "Nom")[1]);
    drag(&mut harness, from, to);
    assert_eq!(
        moves(&harness),
        [("b".to_owned(), TableIntent::MoveColumn { from: 2, to: 0 })]
    );
}

/// Scenario "Fenêtre élargie".
#[test]
fn window_widened() {
    let mut harness = layout_recorder(directory());
    harness.run();
    harness.set_size(egui::vec2(1500.0, WINDOW.y));
    harness.run();
    assert_near(table_tile(&harness).span(), 975.0);
    assert_eq!(split_intents(&harness), []);
}

/// Scenario "Pas d'onglets".
#[test]
fn no_tabs() {
    let mut harness = layout_recorder(directory());
    harness.run();
    assert!(harness.query_by_role(Role::Tab).is_none());
    for id in ["principal", "personnes", "proprietes"] {
        assert!(harness.query_by_label_contains(id).is_none(), "{id}");
    }
}

/// Scenario "Vue glissée".
#[test]
fn view_dragged() {
    let mut harness = layout_recorder(directory());
    harness.run();
    let before = table_tile(&harness);
    let middle_of_the_form = egui::pos2((before.max + WINDOW.x) / 2.0, WINDOW.y / 2.0);
    drag(
        &mut harness,
        middle_of_the_form,
        egui::pos2(1.0, WINDOW.y / 2.0),
    );
    let after = table_tile(&harness);
    assert_near(after.min, before.min);
    assert_near(after.span(), before.span());
    assert_eq!(split_intents(&harness), []);
}

/// The annuaire whose view `proprietes`, on the right, is an empty view instead of a form.
fn directory_with_empty_view() -> AppState {
    let app = example("annuaire/app.json").replace(
        r#""widget": { "_type_": "Form", "class": "Personne" }"#,
        r#""widget": { "_type_": "Empty" }"#,
    );
    directory_with(&app, &example("annuaire/personnes.json"))
}

/// Scenario "Vue de droite de l'annuaire", on a copy of the annuaire whose view `proprietes` is
/// empty: its tile holds only the containers of egui, without text, while that of the table holds its widgets. (The table may
/// have nodes beyond the separator: its columns that do not fit, cut by its horizontal scroll;
/// the screen cannot tell them apart, the tree can.)
#[test]
fn empty_view_draws_nothing() {
    let mut harness = layout_recorder(directory_with_empty_view());
    harness.run();
    let separator = separator(&harness);
    let is_separator = |node: &Node<'_>| {
        node.accesskit_node().role() == Role::Unknown && node_contains(node, separator)
    };
    // The container of the split: the parent of the separator; its other children are its
    // tiles, in the order of `app.json`.
    let split = harness
        .query_all(By::new())
        .find(|node| node.children().any(|child| is_separator(&child)))
        .expect("the container of the split, around its separator");
    let tiles: Vec<Node<'_>> = split.children().filter(|c| !is_separator(c)).collect();
    assert_eq!(tiles.len(), 2);
    assert!(
        tiles[0]
            .children_recursive()
            .any(|node| node.accesskit_node().role() == Role::TextInput),
        "the tile of the table holds its filter field"
    );
    let drawn: Vec<String> = tiles[1]
        .children_recursive()
        .map(|node| node.accesskit_node())
        .filter(|node| {
            node.role() != Role::GenericContainer
                || node.label().is_some()
                || node.value().is_some()
        })
        .map(|node| format!("{:?} {:?}", node.role(), node.label()))
        .collect();
    assert!(drawn.is_empty(), "{drawn:?}");
}

/// `true` if `node` has bounds, and `pos` is within them.
fn node_contains(node: &Node<'_>, pos: egui::Pos2) -> bool {
    node.accesskit_node().bounding_box().is_some() && node.rect().contains(pos)
}

/// Scenario "Séparation glissée".
#[test]
fn separator_dragged() {
    let mut harness = layout_recorder(directory());
    harness.run();
    let from = separator(&harness);
    drag(&mut harness, from, from - egui::vec2(100.0, 0.0));
    let resized = resized_splits(&harness);
    assert!(!resized.is_empty(), "no intent of the split");
    assert!(
        resized.iter().all(|r| *r == ("principal".to_owned(), 2)),
        "{resized:?}"
    );
    let parts = parts_received(&mut harness);
    assert!((parts[0] - 0.55).abs() < 0.005, "{parts:?}");
    assert_near(table_tile(&harness).span(), 550.0);
}

/// Scenario "Double-clic sur la séparation".
#[test]
fn separator_double_clicked() {
    let mut harness = layout_recorder(directory());
    harness.run();
    let pos = separator(&harness);
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false, true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(resized_splits(&harness), [("principal".to_owned(), 2)]);
    let parts = parts_received(&mut harness);
    assert!(parts.iter().all(|p| (p - 0.5).abs() < 1e-6), "{parts:?}");
}

/// Scenario "Aucune séparation déplacée".
#[test]
fn no_separator_moved() {
    let mut harness = layout_recorder(directory());
    harness.run_steps(10);
    assert_eq!(split_intents(&harness), []);
}

/// Scenario "Séparation retrouvée" (`session-state`): the whole round trip, in the application,
/// on a copy of the annuaire.
#[test]
fn separator_found_again() {
    use eframe::App as _;
    let _lock = lock_app_tests();
    let dir = directory_copy();
    let app = dir.path().join("app.json");
    let open = || {
        HarnessBuilder::default()
            .with_size(WINDOW)
            .with_step_dt(1.0 / 60.0)
            .build_eframe(|_cc| OrteliaApp::new(AppState::open_with_session(&app)))
    };
    let mut harness = open();
    harness.run();
    let from = separator(&harness);
    drag(&mut harness, from, from - egui::vec2(100.0, 0.0));
    harness.state_mut().on_exit();

    let mut harness = open();
    harness.run();
    let parts = principal_parts(&harness.state().state);
    assert!((parts[0] - 0.55).abs() < 0.005, "{parts:?}");
    assert_near(table_tile(&harness).span(), 550.0);
}
