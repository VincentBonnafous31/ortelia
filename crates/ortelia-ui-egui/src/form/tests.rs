//! Tests of the drawing of a form view (specification `form-view`), without a screen: the
//! values read through the accessibility tree, the places of the widgets read in the bounds of
//! their nodes, never written as numbers.

use egui::Role;
use egui::accesskit::Toggled;
use egui_kittest::kittest::{NodeT as _, Queryable};
use egui_kittest::{Harness, HarnessBuilder, Node};
use ortelia_presentation::AppState;

use crate::layout::VIEW_MARGIN;
use crate::tests::{
    Recorder, WINDOW, assert_near, directory, directory_with, example, recorder_of, settle, show,
};

/// The annuaire, its form showing the person `name`, found in its table.
fn showing(name: &str) -> AppState {
    show(directory(), name)
}

/// A harness of size `size` that draws `state` and records its intents, after its first
/// frames: the widths the table measures then are given to the presentation and forgotten, so
/// that only the intents of the interactions of the test remain.
fn harness_of(state: AppState, size: egui::Vec2) -> Harness<'static, Recorder> {
    let mut harness = recorder_of(HarnessBuilder::default().with_size(size), state);
    harness.run();
    settle(&mut harness);
    harness.run();
    harness
}

/// The window of the annuaire.
fn harness(state: AppState) -> Harness<'static, Recorder> {
    harness_of(state, WINDOW)
}

/// The node of the form view: its widgets are looked for in it, apart from those of the table
/// (whose check boxes are named by their column, `Contact`).
fn form<'h>(harness: &'h Harness<'_, Recorder>) -> Node<'h> {
    harness.get_by_role(Role::Form)
}

/// The widget of role `role` and label `label` of the form.
fn widget<'h>(harness: &'h Harness<'_, Recorder>, role: Role, label: &'h str) -> Node<'h> {
    form(harness).get_by_role_and_label(role, label)
}

fn rect(harness: &Harness<'_, Recorder>, role: Role, label: &str) -> egui::Rect {
    widget(harness, role, label).rect()
}

/// The right edge of the form view, the last of the window: that of the window, less the margin
/// of the view.
fn view_right(harness: &Harness<'_, Recorder>) -> f32 {
    harness.ctx.content_rect().right() - f32::from(VIEW_MARGIN)
}

/// Scenario "Formulaire de Dupont affiché".
#[test]
fn form_of_dupont_displayed() {
    let harness = harness(showing("Dupont"));
    let nom = widget(&harness, Role::TextInput, "Nom");
    assert_eq!(nom.value().as_deref(), Some("Dupont"));
    let contact = widget(&harness, Role::CheckBox, "Contact");
    assert_eq!(contact.accesskit_node().toggled(), Some(Toggled::True));
    let situation = widget(&harness, Role::ComboBox, "Situation");
    assert_eq!(situation.value().as_deref(), Some("Marié(e)"));
    assert_eq!(
        widget(&harness, Role::TextInput, "Taille (m)")
            .value()
            .as_deref(),
        Some("1,68")
    );
    let notes = widget(&harness, Role::MultilineTextInput, "Notes");
    assert!(notes.value().is_some_and(|v| !v.is_empty()), "{notes:?}");
    form(&harness).get_by_label("Identité");
}

/// Scenario "Formulaire vide affiché".
#[test]
fn empty_form_displayed() {
    let mut harness = recorder_of(HarnessBuilder::default(), directory());
    harness.run();
    form(&harness).get_by_role_and_label(Role::Label, "Nom");
    let nom = widget(&harness, Role::TextInput, "Nom");
    assert_eq!(nom.value().as_deref(), Some(""));
    let contact = widget(&harness, Role::CheckBox, "Contact");
    // Without value: neither checked nor unchecked.
    assert_eq!(contact.accesskit_node().toggled(), Some(Toggled::Mixed));
}

/// Scenario "Champs alignés".
#[test]
fn fields_aligned() {
    let harness = harness(showing("Dupont"));
    let enfants = rect(&harness, Role::TextInput, "Enfants");
    let taille = rect(&harness, Role::TextInput, "Taille (m)");
    assert_near(enfants.left(), taille.left());
    assert_near(enfants.right(), view_right(&harness));
    assert_near(taille.right(), view_right(&harness));
    // Their labels on their left, in the same rows.
    let label = rect(&harness, Role::Label, "Enfants");
    assert!(label.right() <= enfants.left(), "{label:?} {enfants:?}");
    assert!(enfants.y_range().contains(label.center().y), "{label:?}");
}

/// Scenario "Zone de texte sous son libellé".
#[test]
fn text_area_under_its_label() {
    let harness = harness(showing("Dupont"));
    let label = rect(&harness, Role::Label, "Notes");
    let notes = rect(&harness, Role::MultilineTextInput, "Notes");
    assert!(notes.top() >= label.bottom(), "{label:?} {notes:?}");
    assert_near(notes.left(), label.left());
    // On the two columns of the form: to the right edge of the view.
    assert_near(notes.right(), view_right(&harness));
}

/// Scenario "Groupe dans son cadre".
#[test]
fn group_in_its_frame() {
    let harness = harness(showing("Dupont"));
    let title = form(&harness).get_by_label("Identité").rect();
    let nom = rect(&harness, Role::TextInput, "Nom");
    let ne_le = rect(&harness, Role::TextInput, "Né le");
    let enfants = rect(&harness, Role::TextInput, "Enfants");
    assert!(title.bottom() <= nom.top(), "{title:?} {nom:?}");
    assert!(enfants.top() > ne_le.bottom(), "{ne_le:?} {enfants:?}");
    // Inside its frame: the labels of the group start further right than those outside.
    let label_nom = rect(&harness, Role::Label, "Nom");
    let label_enfants = rect(&harness, Role::Label, "Enfants");
    assert!(
        label_nom.left() > label_enfants.left(),
        "{label_nom:?} {label_enfants:?}"
    );
}

/// Scenario "Saisie dans un champ".
#[test]
fn typing_in_a_field() {
    let mut harness = harness(showing("Dupont"));
    widget(&harness, Role::TextInput, "Nom").click();
    harness.run();
    widget(&harness, Role::TextInput, "Nom").type_text("x");
    harness.run();
    harness.key_press(egui::Key::Backspace);
    harness.run();
    let nom = widget(&harness, Role::TextInput, "Nom");
    assert_eq!(nom.value().as_deref(), Some("Dupont"));
    assert!(nom.accesskit_node().is_read_only());
    assert_eq!(harness.state().intents, []);
}

/// Scenario "Clic sur une case"; the list and the radio buttons are disabled too.
#[test]
fn click_on_a_check_box() {
    let mut harness = harness(showing("Dupont"));
    widget(&harness, Role::CheckBox, "Contact").click();
    harness.run();
    let contact = widget(&harness, Role::CheckBox, "Contact");
    assert_eq!(contact.accesskit_node().toggled(), Some(Toggled::True));
    assert!(contact.accesskit_node().is_disabled());
    assert!(
        widget(&harness, Role::ComboBox, "Situation")
            .accesskit_node()
            .is_disabled()
    );
    assert_eq!(harness.state().intents, []);
}

/// The annuaire showing Dupont, the situation in radio buttons.
fn with_radio_buttons() -> AppState {
    let app = example("annuaire/app.json").replace(
        r#""_type_": "Field", "path": "situation""#,
        r#""_type_": "Field", "path": "situation", "widget": { "_type_": "RadioGroup" }"#,
    );
    let state = directory_with(&app, &example("annuaire/personnes.json"));
    show(state, "Dupont")
}

/// The radio buttons of an enum: one per literal, that of the value selected, all disabled,
/// in a group named by the label of the field.
#[test]
fn radio_buttons() {
    let harness = harness(with_radio_buttons());
    let group = widget(&harness, Role::RadioGroup, "Situation");
    let buttons: Vec<Node<'_>> = group.get_all_by_role(Role::RadioButton).collect();
    assert_eq!(buttons.len(), 5);
    let selected: Vec<String> = buttons
        .iter()
        .filter(|b| b.accesskit_node().toggled() == Some(Toggled::True))
        .filter_map(|b| b.accesskit_node().label())
        .collect();
    assert_eq!(selected, ["Marié(e)"]);
    assert!(buttons.iter().all(|b| b.accesskit_node().is_disabled()));
}

/// Scenario "Notes atteintes par défilement".
#[test]
fn notes_reached_by_scrolling() {
    // Low enough for the notes to be below the view: the form is about 350 points high.
    let mut harness = harness_of(showing("Dupont"), egui::vec2(640.0, 240.0));
    // The form view, at the bottom right of the window: from its top to its bottom margins.
    let size = harness.ctx.content_rect().size();
    let margin = f32::from(VIEW_MARGIN);
    let view = egui::Rangef::new(margin, size.y - margin);
    let notes = rect(&harness, Role::MultilineTextInput, "Notes");
    assert!(notes.top() >= view.max, "{view:?} {notes:?}");

    let label = rect(&harness, Role::Label, "Enfants");
    harness.hover_at(label.center());
    harness.run();
    for _ in 0..5 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -10.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
    // The scroll is smoothed over a few frames.
    harness.run_steps(30);
    let notes = rect(&harness, Role::MultilineTextInput, "Notes");
    assert!(
        notes.top() < view.max && notes.bottom() > view.min,
        "{view:?} {notes:?}"
    );
}
