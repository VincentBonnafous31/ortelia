//! Command line parsing. With two options, a parsing library brings nothing; clap may come
//! with the application folder argument (increment 1).

use std::ffi::OsString;
use std::fmt;

/// What the user asked for on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// No argument: open the main window.
    Run,
    /// `--version`
    Version,
    /// `--help`
    Help,
}

/// Text printed by `--help`.
pub const HELP: &str = "\
Ortelia: build data applications without code.

Usage: ortelia [OPTION]

Options:
  --version   print the version and exit
  --help      print this help and exit

Without option, Ortelia opens its main window.
";

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
/// panics on an argument that is not valid Unicode.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    let command = match args.next() {
        None => return Ok(Command::Run),
        Some(first) => match first.to_str() {
            Some("--version") => Command::Version,
            Some("--help") => Command::Help,
            _ => return Err(usage_error(&first)),
        },
    };
    // A single option at most.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_argument_runs_the_application() {
        assert_eq!(parse_strs(&[]), Ok(Command::Run));
    }

    #[test]
    fn version_and_help() {
        assert_eq!(parse_strs(&["--version"]), Ok(Command::Version));
        assert_eq!(parse_strs(&["--help"]), Ok(Command::Help));
    }

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

    #[test]
    fn help_lists_both_options() {
        assert!(HELP.contains("--version"));
        assert!(HELP.contains("--help"));
    }
}
