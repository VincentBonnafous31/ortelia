//! Snapshot tests: the rendered image of the interface is compared to a reference PNG under
//! `tests/snapshots/` (path relative to the crate root, the working directory of `cargo test`).
//!
//! Tolerances come from `kittest.toml`, at the root of the repository. The images are only
//! compared on Linux, with lavapipe (the Vulkan software renderer), where they are reproducible;
//! see `docs/essais/captures-images.md`. To update a reference, in WSL2:
//! `UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui` (see `CLAUDE.md`).
//!
//! This is a test module of the crate, not an integration test under `tests/`: like a Java test
//! in the same package, it sees the private items (`OrteliaApp`, `draw_main_window`).

use super::*;
use crate::tests::lock_app_tests;
use egui_kittest::{Harness, HarnessBuilder, OsThreshold, SnapshotError, SnapshotOptions};
use ortelia_presentation::MainWindowViewModel;

/// Size of every snapshot, in points (and in pixels, at scale 1).
const SNAPSHOT_SIZE: egui::Vec2 = egui::vec2(640.0, 400.0);

/// Builder of every snapshot harness: fixed size, scale and theme, so that the image does not
/// depend on the screen or on the theme of the system.
fn snapshot_harness<State>() -> HarnessBuilder<State> {
    Harness::builder()
        .with_size(SNAPSHOT_SIZE)
        .with_pixels_per_point(1.0)
        // The theme egui picks when it does not know the one of the system.
        .with_theme(egui::Theme::Dark)
}

// `cfg_attr` adds the attribute only when the condition holds (an `#ifdef` on an annotation):
// outside Linux, the test is compiled and listed as ignored, with its reason, and
// `cargo test -- --include-ignored` still runs it.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn main_window() {
    // The whole application logs "main window shown": see `APP_TESTS`.
    let _lock = lock_app_tests();
    let mut harness = snapshot_harness().build_eframe(|_cc| OrteliaApp::new(AppState::new()));
    harness.run();
    // Records the result; the harness panics when dropped if the image does not match.
    harness.snapshot("main_window");
}

/// Proves that the comparison is not empty: a tolerance too wide or a blank render would let
/// a different heading pass.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn different_heading_fails_the_comparison() {
    // In update mode (the values egui_kittest reads as such), it would replace the temporary
    // reference and return `Ok`: the test would prove nothing.
    let update_mode = std::env::var("UPDATE_SNAPSHOTS")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on" | "force"));
    if update_mode {
        return;
    }

    // One harness, whose state is the view-model: the test changes the heading between renders.
    // The view-model is written here, independent of the application version.
    let vm = MainWindowViewModel {
        title: "Ortelia".to_string(),
        heading: "Ortelia".to_string(),
    };
    let mut harness = snapshot_harness().build_ui_state(
        |ui, vm: &mut MainWindowViewModel| draw_main_window(ui, vm),
        vm,
    );
    let dir = tempfile::tempdir().expect("a temporary directory should be created");
    let options = SnapshotOptions::new().output_path(dir.path());

    // 1. The reference, in the temporary directory: the repository references are not touched.
    harness.run();
    let reference = harness.render().expect("the main window should render");
    reference
        .save(dir.path().join("main_window.png"))
        .expect("the temporary reference should be written");

    // 2. The same heading again matches: the difference below comes from the text, not from
    //    the renderer.
    harness.run();
    let same = harness.render().expect("the main window should render");
    egui_kittest::try_image_snapshot_options(&same, "main_window", &options)
        .expect("a second render of the same heading should match the reference");

    // 3. Another heading does not match.
    harness.state_mut().heading = "Ortelia 2".to_string();
    harness.run();
    let other = harness.render().expect("the main window should render");
    match egui_kittest::try_image_snapshot_options(&other, "main_window", &options) {
        Err(SnapshotError::Diff { diff, .. }) => {
            assert!(diff > 0, "a failed comparison should count failing pixels");
        }
        other => panic!("expected a pixel difference, got {other:?}"),
    }
}

/// The tolerances of `kittest.toml` stay strict enough for a moved one-pixel line or a small
/// broken icon to fail a snapshot: per-pixel threshold and number of failing pixels bounded,
/// for every system.
#[test]
fn snapshot_tolerances_are_strict() {
    // egui_kittest uses the first kittest.toml found going up from the working directory (the
    // crate root during `cargo test`): it must be the single one, at the repository root.
    let root_file =
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../kittest.toml"))
            .canonicalize()
            .expect("kittest.toml should be at the repository root");
    let used_file = std::env::current_dir()
        .expect("the working directory should be readable")
        .ancestors()
        .map(|dir| dir.join("kittest.toml"))
        .find(|file| file.exists())
        .and_then(|file| file.canonicalize().ok());
    assert_eq!(
        used_file.as_ref(),
        Some(&root_file),
        "egui_kittest would not use the kittest.toml of the repository root"
    );

    // The values egui_kittest applies, read by its own loader, which merges the [windows],
    // [mac] and [linux] sections and panics on an unknown key or section (such as [macos]).
    let threshold = OsThreshold::<f32>::default();
    let max_failed_pixels = OsThreshold::<usize>::default();
    let systems = [
        ("windows", threshold.windows, max_failed_pixels.windows),
        ("mac", threshold.macos, max_failed_pixels.macos),
        ("linux", threshold.linux, max_failed_pixels.linux),
        ("other", threshold.fallback, max_failed_pixels.fallback),
    ];
    for (system, threshold, max_failed_pixels) in systems {
        // At most the default threshold of egui_kittest, which the trial validated.
        assert!(
            threshold <= 0.6,
            "kittest.toml: threshold {threshold} for {system}, more than 0.6"
        );
        assert!(
            max_failed_pixels <= 10,
            "kittest.toml: max_failed_pixels {max_failed_pixels} for {system}, more than 10"
        );
    }
}
