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
use crate::tests::{directory, lock_app_tests, recorder_of};
use egui_kittest::{Harness, HarnessBuilder, OsThreshold, SnapshotError, SnapshotOptions};
use ortelia_presentation::{AppState, Intent, TableIntent};

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
    let mut harness = snapshot_harness().build_eframe(|_cc| OrteliaApp::new(directory()));
    harness.run();
    // Records the result; the harness panics when dropped if the image does not match.
    harness.snapshot("main_window");
}

/// The dialog of an application that could not be opened, with two errors fixed here (not
/// those of a real file: the image does not change with the messages of the lower layers).
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn open_error() {
    let _lock = lock_app_tests();
    let errors = vec![
        "app.json: unknown class `Personne` in `presentations`".to_owned(),
        "app.json: the window has no view".to_owned(),
    ];
    let mut harness =
        snapshot_harness().build_eframe(|_cc| OrteliaApp::new(AppState::with_open_errors(errors)));
    harness.run();
    harness.snapshot("open_error");
}

/// Proves that the comparison is not empty: a tolerance too wide or a blank render would let
/// a different table pass.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
fn different_table_fails_the_comparison() {
    // In update mode (the values egui_kittest reads as such), it would replace the temporary
    // reference and return `Ok`: the test would prove nothing.
    let update_mode = std::env::var("UPDATE_SNAPSHOTS")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on" | "force"));
    if update_mode {
        return;
    }

    // One harness, which holds the presentation: the test changes the filter between renders.
    let mut harness = recorder_of(snapshot_harness(), directory());
    let dir = tempfile::tempdir().expect("a temporary directory should be created");
    let options = SnapshotOptions::new().output_path(dir.path());

    // 1. The reference, in the temporary directory: the repository references are not touched.
    harness.run();
    let reference = harness.render().expect("the main window should render");
    reference
        .save(dir.path().join("main_window.png"))
        .expect("the temporary reference should be written");

    // 2. The same table again matches: the difference below comes from the table, not from
    //    the renderer.
    harness.run();
    let same = harness.render().expect("the main window should render");
    egui_kittest::try_image_snapshot_options(&same, "main_window", &options)
        .expect("a second render of the same table should match the reference");

    // 3. The table filtered by `dup` does not match.
    harness.state_mut().state.dispatch(Intent::table(
        "personnes",
        TableIntent::SetFilter("dup".into()),
    ));
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
