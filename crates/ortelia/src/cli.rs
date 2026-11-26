//! Command line parsing. With two options and one path, a parsing library brings nothing.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

/// What the user asked for on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Opens the application at `path` (a folder or an `app.json`), or the `app.json` of the
    /// current folder.
    Run { path: Option<PathBuf> },
    /// `--version`
    Version,
    /// `--help`
    Help,
}

/// Text printed by `--help`.
pub const HELP: &str = "\
Ortelia: build data applications without code.

Usage: ortelia [OPTION]
       ortelia [PATH]

Arguments:
  PATH        the application to open: a folder that contains an app.json, or the
              app.json file itself (default: the app.json of the current folder)

Options:
  --version   print the version and exit
  --help      print this help and exit
";

/// The name of the file of an application, in its folder.
const APP_FILE: &str = "app.json";

/// An argument that is not recognised (exit code 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    /// The argument as typed, made printable if it is not valid Unicode.
    pub argument: String,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unrecognised argument '{}'\nRun 'ortelia --help' for the list of options.",
            self.argument
        )
    }
}

/// Parses the arguments, program name excluded.
///
/// The arguments are `OsString`s (as returned by `std::env::args_os`) because `std::env::args`
/// panics on an argument that is not valid Unicode, and a path need not be valid Unicode.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    let command = match args.next() {
        None => return Ok(Command::Run { path: None }),
        Some(first) => match first.to_str() {
            Some("--version") => Command::Version,
            Some("--help") => Command::Help,
            // Any other option is unknown; anything else is the path of the application.
            Some(option) if option.starts_with('-') => return Err(usage_error(&first)),
            _ => Command::Run {
                path: Some(PathBuf::from(first)),
            },
        },
    };
    // A single option or path at most.
    match args.next() {
        None => Ok(command),
        Some(extra) => Err(usage_error(&extra)),
    }
}

fn usage_error(argument: &OsString) -> UsageError {
    UsageError {
        argument: argument.to_string_lossy().into_owned(),
    }
}

/// The `app.json` to open: the one of a folder, the file given, or the one of the current
/// folder (a relative path, which the system resolves against the current folder).
pub fn app_file(path: Option<&Path>) -> PathBuf {
    match path {
        Some(path) if path.is_dir() => path.join(APP_FILE),
        Some(path) => path.to_owned(),
        None => PathBuf::from(APP_FILE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    fn examples() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    #[test]
    fn no_argument_runs_the_application() {
        assert_eq!(parse_strs(&[]), Ok(Command::Run { path: None }));
    }

    #[test]
    fn path_argument() {
        assert_eq!(
            parse_strs(&["examples/annuaire"]),
            Ok(Command::Run {
                path: Some(PathBuf::from("examples/annuaire"))
            })
        );
    }

    #[test]
    fn version_and_help() {
        assert_eq!(parse_strs(&["--version"]), Ok(Command::Version));
        assert_eq!(parse_strs(&["--help"]), Ok(Command::Help));
    }

    /// Scenario "Option inconnue".
    #[test]
    fn unknown_argument_is_an_error() {
        let error = parse_strs(&["--frobnicate"]).unwrap_err();
        assert_eq!(error.argument, "--frobnicate");
        let message = error.to_string();
        assert!(message.contains("--frobnicate"), "{message}");
        assert!(message.contains("--help"), "{message}");
    }

    #[test]
    fn extra_argument_is_an_error() {
        let error = parse_strs(&["--version", "extra"]).unwrap_err();
        assert_eq!(error.argument, "extra");
    }

    /// Scenario "Deux chemins".
    #[test]
    fn two_paths_are_an_error() {
        let error = parse_strs(&["examples/annuaire", "autre"]).unwrap_err();
        assert_eq!(error.argument, "autre");
        assert!(error.to_string().contains("--help"));
    }

    /// Scenario "Aide demandée" (the text; the exit code is tested on the executable).
    #[test]
    fn help_lists_the_options_and_the_path() {
        assert!(HELP.contains("--version"));
        assert!(HELP.contains("--help"));
        assert!(HELP.contains("PATH"));
    }

    /// Scenario "Dossier".
    #[test]
    fn folder_gives_its_app_json() {
        let folder = examples().join("annuaire");
        assert_eq!(app_file(Some(&folder)), folder.join("app.json"));
    }

    /// Scenario "Fichier".
    #[test]
    fn file_gives_itself() {
        let file = examples().join("annuaire/app.json");
        assert_eq!(app_file(Some(&file)), file);
    }

    /// Scenario "Sans argument": `app.json`, relative to the current folder.
    #[test]
    fn no_path_gives_the_app_json_of_the_current_folder() {
        assert_eq!(app_file(None), PathBuf::from("app.json"));
    }
}
