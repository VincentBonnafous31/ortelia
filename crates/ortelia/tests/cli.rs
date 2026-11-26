//! Integration tests of the command line: they run the real executable. Cargo gives its path
//! to the integration tests of a crate that has a binary (`CARGO_BIN_EXE_<name>`).

use std::process::{Command, Output};

fn ortelia(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ortelia"))
        .args(args)
        .output()
        .expect("run ortelia")
}

#[test]
fn version_is_printed() {
    let output = ortelia(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("ortelia {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_the_options() {
    let output = ortelia(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--version"), "{stdout}");
    assert!(stdout.contains("--help"), "{stdout}");
    assert!(stdout.contains("PATH"), "{stdout}");
}

#[test]
fn unknown_argument_is_refused() {
    let output = ortelia(&["--frobnicate"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--frobnicate"), "{stderr}");
    assert!(stderr.contains("--help"), "{stderr}");
}

/// Scenario "Deux chemins": refused before any window.
#[test]
fn two_paths_are_refused() {
    let output = ortelia(&["examples/annuaire", "autre"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("autre"), "{stderr}");
    assert!(stderr.contains("--help"), "{stderr}");
}
