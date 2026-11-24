//! File logging, set up once by the executable (the composition root).
//!
//! Libraries only use the `tracing` macros (`info!`, `warn!`...): `tracing` is the API, like
//! SLF4J in Java, and this module is the configuration, like Logback.
//!
//! Privacy rule: logs never contain values of the user's data (content of loaded files, typed
//! values). Types, technical identifiers, file paths and counters are fine.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::panic;
use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_log::LogTracer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

/// Replaces the log directory when set and not empty.
pub const LOG_DIR_VAR: &str = "ORTELIA_LOG_DIR";
/// Log filter: a level (`debug`) or per-crate directives (`ortelia_service=trace`).
pub const LOG_FILTER_VAR: &str = "ORTELIA_LOG";

/// Number of daily log files kept.
const MAX_LOG_FILES: usize = 7;

/// Directory of the log files: `ORTELIA_LOG_DIR` if set and not empty, otherwise
/// `<state dir>/ortelia/logs` (Linux: `$XDG_STATE_HOME`, `~/.local/state` by default),
/// otherwise `<local data dir>/ortelia/logs` (Windows: `%LOCALAPPDATA%`, as Windows has no
/// state directory).
///
/// The system directories are parameters, so that tests can give their own.
fn log_dir(
    override_dir: Option<OsString>,
    state_dir: Option<PathBuf>,
    data_local_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(dir) = override_dir.filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let base = state_dir.or(data_local_dir)?;
    Some(base.join("ortelia").join("logs"))
}

/// The log file cannot be opened in `dir`.
#[derive(Debug)]
pub struct LogFileError {
    pub dir: PathBuf,
    pub source: io::Error,
}

impl fmt::Display for LogFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot write log files to {}: {}",
            self.dir.display(),
            self.source
        )
    }
}

/// Opens the daily log file `ortelia.<YYYY-MM-DD>.log` (UTC date) in `dir`, creating the
/// directory if needed and deleting the oldest files beyond `MAX_LOG_FILES`.
fn open_log_file(dir: &Path) -> Result<RollingFileAppender, LogFileError> {
    let error = |source| LogFileError {
        dir: dir.to_path_buf(),
        source,
    };
    // Created here rather than by `build`, which first tries to list the missing directory to
    // delete old files and prints an error about it on stderr.
    std::fs::create_dir_all(dir).map_err(error)?;
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("ortelia")
        .filename_suffix("log")
        .max_log_files(MAX_LOG_FILES)
        .build(dir)
        .map_err(|e| error(io::Error::other(e)))
}

/// Builds the log filter from the value of `ORTELIA_LOG`: `info` by default. An invalid value
/// is ignored (the default applies) and the returned warning must be logged once the
/// subscriber is installed.
///
/// Note: a single word such as `abc` is a valid filter (a module name), not an error.
fn env_filter(spec: Option<&OsStr>) -> (EnvFilter, Option<String>) {
    let default = || EnvFilter::new("info");
    let Some(spec) = spec else {
        return (default(), None);
    };
    let Some(text) = spec.to_str() else {
        let warning = format!("ignoring {LOG_FILTER_VAR}: not valid Unicode; using 'info'");
        return (default(), Some(warning));
    };
    // `ortelia_service=trace` alone would silence every other crate. Starting with `info`
    // keeps that level for the other crates; a later global level (`debug`) replaces it.
    match EnvFilter::builder().parse(format!("info,{text}")) {
        Ok(filter) => (filter, None),
        Err(error) => {
            let warning = format!("ignoring {LOG_FILTER_VAR}={text:?}: {error}; using 'info'");
            (default(), Some(warning))
        }
    }
}

/// The subscriber: text lines without colour codes, UTC timestamps (RFC 3339).
///
/// `BoxMakeWriter` hides the writer type: the application passes the non-blocking writer,
/// tests pass the file appender directly.
fn subscriber(writer: BoxMakeWriter, filter: EnvFilter) -> impl tracing::Subscriber + Send + Sync {
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(filter)
        .finish()
}

/// Sets up file logging for the whole process.
///
/// Returns the guard of the background writing thread: it must stay alive until the end of
/// `main`, since dropping it flushes the pending lines (RAII, like a C++ destructor). Write
/// `let _guard = init(..)`, not `let _ = init(..)`, which would drop it at once.
///
/// If the log file cannot be opened, prints a warning on stderr and returns `None`: the
/// application goes on without a log file.
pub fn init(dir_override: Option<OsString>, filter_spec: Option<OsString>) -> Option<WorkerGuard> {
    let Some(dir) = log_dir(dir_override, dirs::state_dir(), dirs::data_local_dir()) else {
        eprintln!("ortelia: warning: no directory for log files; continuing without a log file");
        return None;
    };
    let appender = match open_log_file(&dir) {
        Ok(appender) => appender,
        Err(error) => {
            eprintln!("ortelia: warning: {error}; continuing without a log file");
            return None;
        }
    };
    // A background thread writes to the disk, like Log4j's AsyncAppender, so that a slow disk
    // never slows the user interface down. Not lossy: when its queue is full, a burst of
    // events waits instead of dropping lines (a panic line must never be lost).
    let (writer, guard) = NonBlockingBuilder::default().lossy(false).finish(appender);
    let (filter, warning) = env_filter(filter_spec.as_deref());
    if let Err(error) =
        tracing::subscriber::set_global_default(subscriber(BoxMakeWriter::new(writer), filter))
    {
        eprintln!("ortelia: warning: cannot install the logger: {error}");
        return None;
    }
    if let Err(error) = forward_log_records() {
        tracing::warn!("cannot forward the records of the log crate: {error}");
    }
    if let Some(warning) = warning {
        tracing::warn!("{warning}");
    }
    Some(guard)
}

/// eframe, wgpu and other third-party crates log through the `log` crate, not `tracing`:
/// forwards their warnings and errors to the current subscriber, for diagnosis. Their `info`
/// records are too verbose (adapter lists...) and stay out, whatever `ORTELIA_LOG` says.
/// (winit logs through `tracing` directly: it follows `ORTELIA_LOG` like Ortelia.) Some of
/// these crates log harmless ERROR lines while probing the system (EGL, desktop portal): only
/// Ortelia's own lines must be free of errors.
///
/// Can be called once per process (it installs the global logger of the `log` crate).
fn forward_log_records() -> Result<(), tracing_log::log::SetLoggerError> {
    LogTracer::init_with_filter(tracing_log::log::LevelFilter::Warn)
}

/// Logs panics at the `error` level, then calls the previous hook, which still prints the
/// panic on stderr.
pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let message = info.payload_as_str().unwrap_or("(no message)");
        match info.location() {
            Some(location) => tracing::error!("panic at {location}: {message}"),
            None => tracing::error!("panic: {message}"),
        }
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn override_dir_replaces_default() {
        let dir = log_dir(
            Some(OsString::from("/custom/logs")),
            Some(PathBuf::from("/state")),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(dir, Some(PathBuf::from("/custom/logs")));
    }

    #[test]
    fn empty_override_is_ignored() {
        let dir = log_dir(Some(OsString::new()), Some(PathBuf::from("/state")), None);
        assert_eq!(dir, Some(Path::new("/state").join("ortelia").join("logs")));
    }

    #[test]
    fn default_dir_is_under_state_dir() {
        let dir = log_dir(
            None,
            Some(PathBuf::from("/tmp/etat")),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(
            dir,
            Some(Path::new("/tmp/etat").join("ortelia").join("logs"))
        );
    }

    #[test]
    fn local_data_dir_when_no_state_dir() {
        // The Windows case: dirs::state_dir() returns None.
        let dir = log_dir(None, None, Some(PathBuf::from("/data")));
        assert_eq!(dir, Some(Path::new("/data").join("ortelia").join("logs")));
    }

    /// Runs `body` with a subscriber that writes to the daily file of `dir`, for this thread
    /// only, then returns the names and the content of the log files.
    fn log_to(dir: &Path, spec: Option<&str>, body: impl FnOnce()) -> (Vec<String>, String) {
        let appender = open_log_file(dir).expect("open log file");
        let (filter, warning) = env_filter(spec.map(OsStr::new));
        let subscriber = subscriber(BoxMakeWriter::new(appender), filter);
        tracing::subscriber::with_default(subscriber, || {
            if let Some(warning) = warning {
                tracing::warn!("{warning}");
            }
            body();
        });
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        let content = names
            .iter()
            .map(|name| fs::read_to_string(dir.join(name)).unwrap())
            .collect();
        (names, content)
    }

    #[test]
    fn file_is_named_after_utc_date() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("logs");
        let (names, content) = log_to(&dir, None, || tracing::info!("hello"));
        assert_eq!(names.len(), 1, "{names:?}");
        // ortelia.YYYY-MM-DD.log
        let name = &names[0];
        assert!(
            name.starts_with("ortelia.") && name.ends_with(".log"),
            "{name}"
        );
        let date = &name["ortelia.".len()..name.len() - ".log".len()];
        assert_eq!(date.len(), 10, "{name}");
        // The line timestamp is in UTC (RFC 3339, ending with Z) and has the same date.
        let line = content.lines().next().unwrap();
        assert!(line.starts_with(date), "{line}");
        let timestamp = line.split_whitespace().next().unwrap();
        assert!(timestamp.ends_with('Z'), "{line}");
    }

    #[test]
    fn debug_is_filtered_by_default() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), None, || {
            tracing::info!("info event");
            tracing::debug!("debug event");
        });
        assert!(content.contains("info event"), "{content}");
        assert!(!content.contains("debug event"), "{content}");
    }

    #[test]
    fn debug_is_written_with_debug_filter() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), Some("debug"), || {
            tracing::debug!("debug event");
        });
        assert!(content.contains("debug event"), "{content}");
    }

    #[test]
    fn global_level_replaces_the_default() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), Some("warn"), || {
            tracing::info!("info event");
            tracing::warn!("warn event");
        });
        assert!(content.contains("warn event"), "{content}");
        assert!(!content.contains("info event"), "{content}");
    }

    #[test]
    fn per_crate_level_keeps_info_for_the_others() {
        let temp = tempfile::tempdir().unwrap();
        // The events of this test have the target `ortelia::logging::tests`.
        let (_, content) = log_to(temp.path(), Some("some_other_crate=trace"), || {
            tracing::info!("info event");
            tracing::debug!("debug event");
        });
        assert!(content.contains("info event"), "{content}");
        assert!(!content.contains("debug event"), "{content}");

        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), Some("ortelia::logging=debug"), || {
            tracing::debug!("debug event");
        });
        assert!(content.contains("debug event"), "{content}");
    }

    #[test]
    fn invalid_filter_falls_back_to_info_with_warning() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), Some("ortelia=verbeux"), || {
            tracing::info!("info event");
            tracing::debug!("debug event");
        });
        assert!(content.contains("WARN"), "{content}");
        assert!(content.contains("ortelia=verbeux"), "{content}");
        assert!(content.contains("info event"), "{content}");
        assert!(!content.contains("debug event"), "{content}");
    }

    #[test]
    fn at_most_seven_files_are_kept() {
        let temp = tempfile::tempdir().unwrap();
        for day in 1..=8 {
            fs::write(
                temp.path().join(format!("ortelia.2020-01-0{day}.log")),
                "old\n",
            )
            .unwrap();
        }
        let (names, _) = log_to(temp.path(), None, || tracing::info!("new day"));
        assert_eq!(names.len(), MAX_LOG_FILES, "{names:?}");
        // Today's file is among them.
        assert!(
            names.iter().any(|name| !name.starts_with("ortelia.2020-")),
            "{names:?}"
        );
    }

    #[test]
    fn directory_under_a_regular_file_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("not-a-directory");
        fs::write(&file, "").unwrap();
        let dir = file.join("logs");
        let error = open_log_file(&dir).unwrap_err();
        assert_eq!(error.dir, dir);
        assert!(
            error.to_string().contains(&dir.display().to_string()),
            "{error}"
        );
    }

    /// The only test that installs the global logger of the `log` crate (once per process).
    #[test]
    fn third_party_warnings_are_forwarded() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), None, || {
            forward_log_records().expect("first installation of the log forwarder");
            tracing_log::log::warn!(target: "some_library", "library warning");
            tracing_log::log::info!(target: "some_library", "library info");
        });
        let line = content
            .lines()
            .find(|line| line.contains("library warning"))
            .unwrap_or_else(|| panic!("warning not forwarded:\n{content}"));
        assert!(line.contains(" WARN some_library:"), "{line}");
        assert!(!content.contains("library info"), "{content}");
    }

    #[test]
    fn panics_are_logged() {
        let temp = tempfile::tempdir().unwrap();
        let (_, content) = log_to(temp.path(), None, || {
            install_panic_hook();
            let result = panic::catch_unwind(|| panic!("test panic"));
            // Back to the default hook: take_hook() unregisters the current one.
            drop(panic::take_hook());
            assert!(result.is_err());
        });
        let line = content
            .lines()
            .find(|line| line.contains("ERROR"))
            .unwrap_or_else(|| panic!("no ERROR line in:\n{content}"));
        assert!(line.contains("test panic"), "{line}");
        assert!(line.contains("logging.rs"), "{line}");
    }
}
