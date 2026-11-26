//! The Ortelia executable: parses the command line, sets up logging, opens the application
//! and its main window.

mod cli;
mod logging;

use std::path::Path;
use std::process::ExitCode;

use cli::Command;
use ortelia_presentation::AppState;

/// Exit code when the main window cannot be opened (also the one of an application that does
/// not open, given by the presentation).
const EXIT_WINDOW_FAILED: u8 = 1;
/// Exit code of a command line error (like most Unix tools).
const EXIT_USAGE: u8 = 2;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Like `int main` in C: the returned code is the process exit code.
fn main() -> ExitCode {
    // 1. Command line. `--version`, `--help` and usage errors stop here, without any log.
    let command = match cli::parse(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("ortelia: {error}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    match command {
        Command::Version => {
            println!("ortelia {VERSION}");
            ExitCode::SUCCESS
        }
        Command::Help => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Command::Run { path } => run(path.as_deref()),
    }
}

/// Opens the application at `path`, then its window; returns when the window is closed.
fn run(path: Option<&Path>) -> ExitCode {
    // 2. Logging. The guard flushes the log when dropped, at the end of this function: it
    //    must be bound to a named variable (`_log_guard`), not to `_`.
    let _log_guard = logging::init(
        std::env::var_os(logging::LOG_DIR_VAR),
        std::env::var_os(logging::LOG_FILTER_VAR),
    );
    // 3. Panics are logged too.
    logging::install_panic_hook();
    // 4. Startup line.
    tracing::info!(
        "starting Ortelia {VERSION} ({} {})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // 5. The application. One that does not open still gets its window, with the dialog of
    //    its errors; the errors also go to the standard error and to the log.
    let file = cli::app_file(path);
    // With its session state: the layout of the last closing, saved again at this one.
    let state = AppState::open_with_session(&file);
    for error in state.open_errors() {
        eprintln!("ortelia: {error}");
        tracing::error!("cannot open the application: {error}");
    }
    // Known before the window takes the state: it does not depend on how the window closes.
    let exit_code = state.exit_code();

    // 6. Main window.
    match ortelia_ui_egui::run(state) {
        Ok(()) => {
            // 7. Shutdown line.
            tracing::info!("Ortelia stopped");
            ExitCode::from(exit_code)
        }
        Err(error) => {
            eprintln!("ortelia: cannot open the main window: {error}");
            tracing::error!("cannot open the main window: {error}");
            ExitCode::from(EXIT_WINDOW_FAILED)
        }
    }
}
