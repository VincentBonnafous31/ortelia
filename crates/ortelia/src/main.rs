//! The Ortelia executable: parses the command line, sets up logging and opens the main window.

mod cli;
mod logging;

use std::process::ExitCode;

use cli::Command;
use ortelia_presentation::AppState;

/// Exit code when the main window cannot be opened.
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
        Command::Run => run(),
    }
}

/// Opens the main window and returns when it is closed.
fn run() -> ExitCode {
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

    // 5. Main window.
    let state = AppState::new();
    match ortelia_ui_egui::run(state) {
        Ok(()) => {
            // 6. Shutdown line.
            tracing::info!("Ortelia stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("ortelia: cannot open the main window: {error}");
            tracing::error!("cannot open the main window: {error}");
            ExitCode::from(EXIT_WINDOW_FAILED)
        }
    }
}
