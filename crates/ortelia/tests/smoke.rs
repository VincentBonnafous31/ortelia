//! Smoke test of the real executable (scenario "Démarrage réel"): the main window opens and
//! draws its first frame without error.
//!
//! It needs a display (WSLg, Xvfb, a Windows desktop), so it is ignored by `cargo test`.
//! Run it with: `cargo test -p ortelia --test smoke -- --ignored`

use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

/// Software rendering can be slow: far more than the expected time.
const TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Kills the child process when dropped, even if an assertion fails (RAII).
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Content of all the files of a log directory (empty if it does not exist yet).
fn read_logs(dir: &Path) -> String {
    let Ok(entries) = fs::read_dir(dir) else {
        return String::new();
    };
    entries
        .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
        .collect()
}

#[test]
#[ignore = "needs a display: cargo test -p ortelia --test smoke -- --ignored"]
fn main_window_is_shown() {
    let temp = tempfile::tempdir().unwrap();
    let logs = temp.path().join("logs");
    let child = Command::new(env!("CARGO_BIN_EXE_ortelia"))
        .env("ORTELIA_LOG_DIR", &logs)
        .env_remove("ORTELIA_LOG")
        .spawn()
        .expect("start ortelia");
    let mut child = KillOnDrop(child);

    // The background log writer writes each line as it comes: no need to stop the process
    // to read them.
    let start = Instant::now();
    // `break value` makes the loop an expression that returns the log content.
    let content = loop {
        let content = read_logs(&logs);
        if content.contains("main window shown") {
            break content;
        }
        if let Some(status) = child.0.try_wait().unwrap() {
            panic!("ortelia exited with {status} before showing its window; log:\n{content}");
        }
        assert!(
            start.elapsed() < TIMEOUT,
            "no 'main window shown' after {TIMEOUT:?}; log:\n{content}"
        );
        thread::sleep(POLL_INTERVAL);
    };

    assert!(
        child.0.try_wait().unwrap().is_none(),
        "ortelia is no longer running; log:\n{content}"
    );
    let errors = ortelia_errors(&content);
    assert!(errors.is_empty(), "Ortelia errors in the log:\n{content}");
}

/// ERROR lines written by Ortelia itself (targets `ortelia`, `ortelia_ui_egui`...).
///
/// The third-party crates (winit, wgpu...) forward their warnings and errors to the log, and
/// some log harmless errors while probing the system (EGL, desktop portal): they are ignored.
fn ortelia_errors(content: &str) -> Vec<&str> {
    content
        .lines()
        .filter(|line| {
            // <timestamp> <LEVEL> [<span>: ...] <target>: <message>
            // The words ending with ':' after the level are the spans and the target.
            let mut words = line.split_whitespace().skip(1);
            words.next() == Some("ERROR")
                && words
                    .take_while(|word| word.ends_with(':'))
                    .any(|word| word.starts_with("ortelia"))
        })
        .collect()
}

#[test]
fn only_ortelia_errors_count() {
    let content = "\
2026-11-24T07:05:07.705314Z  INFO ortelia: starting Ortelia 0.1.0 (linux x86_64)
2026-11-24T07:05:07.746463Z ERROR wgpu_hal::gles::egl: EGL 'eglInitialize' code 0x3003
2026-11-24T07:05:07.864081Z ERROR ortelia::logging: panic at src/main.rs:1:1: boom
2026-11-24T07:05:08.000000Z ERROR load: ortelia_service::json: cannot read
";
    assert_eq!(
        ortelia_errors(content),
        vec![
            "2026-11-24T07:05:07.864081Z ERROR ortelia::logging: panic at src/main.rs:1:1: boom",
            "2026-11-24T07:05:08.000000Z ERROR load: ortelia_service::json: cannot read",
        ]
    );
}
