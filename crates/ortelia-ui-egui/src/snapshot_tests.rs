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
use egui_kittest::{Harness, HarnessBuilder, SnapshotError, SnapshotOptions};
use ortelia_presentation::MainWindowViewModel;

/// Size of every snapshot, in points (and in pixels, at scale 1).
const SNAPSHOT_SIZE: egui::Vec2 = egui::vec2(640.0, 400.0);

/// Builder of every snapshot harness: fixed size, scale and theme, so that the image does not
/// depend on the screen or on the theme of the system.
pub(crate) fn snapshot_harness<State>() -> HarnessBuilder<State> {
    Harness::builder()
        .with_size(SNAPSHOT_SIZE)
        .with_pixels_per_point(1.0)
        // The theme egui picks when it does not know the one of the system.
        .with_theme(egui::Theme::Dark)
}

/// A main window view-model written in the test, independent of the application version.
fn main_window_view_model(heading: &str) -> MainWindowViewModel {
    MainWindowViewModel {
        title: "Ortelia".to_string(),
        heading: heading.to_string(),
    }
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
    // In update mode, egui_kittest would replace the temporary reference and return `Ok`, so
    // the comparisons below would prove nothing. Stop at once: `UPDATE_SNAPSHOTS=1 cargo test`
    // must succeed. (Reading the environment is safe; only changing it is `unsafe` in Rust 2024.)
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        return;
    }

    // A closure rather than a function: its return type (`image::RgbaImage`) is inferred, so
    // the test needs no direct dependency on the `image` crate.
    let render = |heading: &str| {
        let vm = main_window_view_model(heading);
        let mut harness = snapshot_harness().build_ui(|ui| draw_main_window(ui, &vm));
        harness.run();
        harness.render().expect("the main window should render")
    };
    let dir = tempfile::tempdir().expect("a temporary directory should be created");
    let options = SnapshotOptions::new().output_path(dir.path());

    // 1. The reference, in the temporary directory: the repository references are not touched.
    render("Ortelia")
        .save(dir.path().join("main_window.png"))
        .expect("the temporary reference should be written");

    // 2. The same heading again matches: the difference below comes from the text, not from
    //    the renderer.
    egui_kittest::try_image_snapshot_options(&render("Ortelia"), "main_window", &options)
        .expect("a second render of the same heading should match the reference");

    // 3. Another heading does not match.
    let result =
        egui_kittest::try_image_snapshot_options(&render("Ortelia 2"), "main_window", &options);
    match result {
        Err(SnapshotError::Diff { diff, .. }) => {
            assert!(diff > 0, "a failed comparison should count failing pixels");
        }
        other => panic!("expected a pixel difference, got {other:?}"),
    }
}

/// The tolerances of `kittest.toml` stay strict enough for a moved one-pixel line or a small
/// broken icon to fail a snapshot.
#[test]
fn snapshot_tolerances_are_strict() {
    const MAX_FAILED_PIXELS: i64 = 10;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../kittest.toml");
    let text =
        std::fs::read_to_string(path).expect("kittest.toml should be at the repository root");
    let config: toml::Table = toml::from_str(&text).expect("kittest.toml should be valid TOML");

    // The README of egui_kittest writes `[macos]`, but its code only reads `[mac]`.
    assert!(
        !config.contains_key("macos"),
        "kittest.toml: egui_kittest reads [mac], not [macos]"
    );
    for key in ["threshold", "max_failed_pixels"] {
        assert!(config.contains_key(key), "kittest.toml: `{key}` is missing");
    }

    let mut sections = vec![("root", &config)];
    for name in ["windows", "mac", "linux"] {
        if let Some(table) = config.get(name).and_then(toml::Value::as_table) {
            sections.push((name, table));
        }
    }
    for (section, table) in sections {
        // `failed_pixel_count_threshold` is the old name, still accepted by egui_kittest.
        for key in ["max_failed_pixels", "failed_pixel_count_threshold"] {
            if let Some(value) = table.get(key) {
                let count = value.as_integer().unwrap_or_else(|| {
                    panic!("kittest.toml: `{key}` of section {section} is not an integer")
                });
                assert!(
                    count <= MAX_FAILED_PIXELS,
                    "kittest.toml: `{key}` of section {section} is {count}, \
                     more than {MAX_FAILED_PIXELS}"
                );
            }
        }
    }
}
