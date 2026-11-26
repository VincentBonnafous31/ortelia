//! Integration tests of the startup: logging and failure to open the window.
//!
//! The tests never change their own environment (`std::env::set_var` is `unsafe` in edition
//! 2024, as other threads may read the environment): they pass variables to the child process.

use std::process::Command;

fn ortelia() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ortelia"));
    // Do not depend on the developer's own settings.
    command
        .env_remove("ORTELIA_LOG_DIR")
        .env_remove("ORTELIA_LOG");
    command
}

#[test]
fn version_writes_no_log() {
    let temp = tempfile::tempdir().unwrap();
    let logs = temp.path().join("logs");
    let output = ortelia()
        .arg("--version")
        .env("ORTELIA_LOG_DIR", &logs)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        !logs.exists(),
        "--version must not create the log directory"
    );
}

/// Launching without a display: winit returns an error instead of panicking. Linux only: on
/// Windows the application would really open a window and the test would never end.
#[cfg(target_os = "linux")]
mod without_display {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// The directory application, which opens: a failure of the tests below is the one of the
    /// window.
    fn annuaire() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/annuaire")
    }

    /// Content of all the files of a log directory.
    fn read_logs(dir: &Path) -> String {
        let mut content = String::new();
        for entry in fs::read_dir(dir).expect("log directory exists") {
            content += &fs::read_to_string(entry.unwrap().path()).unwrap();
        }
        content
    }

    fn ortelia_without_display() -> Command {
        let mut command = ortelia();
        command
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WAYLAND_SOCKET");
        command
    }

    /// The directory application, without a display.
    fn annuaire_without_display() -> Command {
        let mut command = ortelia_without_display();
        command.arg(annuaire());
        command
    }

    #[test]
    fn fails_with_code_1_and_a_message() {
        let temp = tempfile::tempdir().unwrap();
        let output = annuaire_without_display()
            .env("ORTELIA_LOG_DIR", temp.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("cannot open the main window"), "{stderr}");
    }

    #[test]
    fn log_dir_is_created_with_startup_line_and_error() {
        let temp = tempfile::tempdir().unwrap();
        let logs = temp.path().join("not").join("yet");
        let output = annuaire_without_display()
            .env("ORTELIA_LOG_DIR", &logs)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let content = read_logs(&logs);
        let startup = content
            .lines()
            .find(|line| line.contains("starting Ortelia"))
            .unwrap_or_else(|| panic!("no startup line in:\n{content}"));
        assert!(startup.contains(" INFO "), "{startup}");
        assert!(startup.contains(env!("CARGO_PKG_VERSION")), "{startup}");
        assert!(startup.contains("linux"), "{startup}");
        assert!(startup.contains(std::env::consts::ARCH), "{startup}");
        assert!(
            content
                .lines()
                .any(|line| line.contains("ERROR") && line.contains("cannot open the main window")),
            "{content}"
        );
    }

    #[test]
    fn default_log_dir_follows_xdg_state_home() {
        let temp = tempfile::tempdir().unwrap();
        let output = annuaire_without_display()
            .env("XDG_STATE_HOME", temp.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let content = read_logs(&temp.path().join("ortelia").join("logs"));
        assert!(content.contains("starting Ortelia"), "{content}");
    }

    #[test]
    fn unusable_log_dir_gives_a_warning_and_startup_goes_on() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("regular-file");
        fs::write(&file, "").unwrap();
        let logs = file.join("logs");
        let output = annuaire_without_display()
            .env("ORTELIA_LOG_DIR", &logs)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("warning"), "{stderr}");
        assert!(stderr.contains(&logs.display().to_string()), "{stderr}");
        // The startup went on until the window: same failure as without the warning.
        assert!(stderr.contains("cannot open the main window"), "{stderr}");
        assert_eq!(output.status.code(), Some(1));
    }

    /// Scenario "Code de sortie": a folder without `app.json`.
    #[test]
    fn application_that_does_not_open_gives_code_1() {
        let temp = tempfile::tempdir().unwrap();
        let output = ortelia_without_display()
            .arg(temp.path())
            .env("ORTELIA_LOG_DIR", temp.path().join("logs"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let expected = temp.path().join("app.json").display().to_string();
        assert!(stderr.contains(&expected), "{stderr}");
        let log = read_logs(&temp.path().join("logs"));
        assert!(
            log.lines()
                .any(|line| line.contains("ERROR") && line.contains(&expected)),
            "{log}"
        );
    }

    /// Scenario "Sans argument": from the folder of the directory, the application opens
    /// (only the window fails, without a display).
    #[test]
    fn without_argument_the_app_json_of_the_current_folder() {
        let temp = tempfile::tempdir().unwrap();
        let output = ortelia_without_display()
            .current_dir(annuaire())
            .env("ORTELIA_LOG_DIR", temp.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("app.json"), "{stderr}");
        assert!(stderr.contains("cannot open the main window"), "{stderr}");
    }
}
