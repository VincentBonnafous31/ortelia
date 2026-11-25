//! The file `app.json`, as read by serde, and the errors of `app.json`.

use std::fmt;

use ortelia_model::metamodel::without_position;
use serde::Deserialize;

/// The version of the format of `app.json` that this version of Ortelia reads.
pub const APP_FORMAT_VERSION: &str = "0.3";

/// A whole `app.json`: the definition of an application.
//
// Like the model file, it refuses a misspelt key (`deny_unknown_fields`), and
// `rename_all = "camelCase"` maps `formatVersion` to `format_version`. Plain comments, not doc
// comments: those become the descriptions of the JSON Schema, shown by text editors.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AppFile {
    /// The JSON Schema of the file, for text editors: read and ignored.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    pub format_version: String,
    /// The identifier of the application.
    pub id: String,
    /// Its name, for the user.
    pub name: String,
    pub model: ModelSection,
    #[serde(default)]
    pub sources: Vec<SourceDef>,
}

/// The model of the application.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModelSection {
    /// The model files, relative to the folder of `app.json`.
    pub files: Vec<String>,
    /// The name of the class of the root object.
    pub root: String,
}

/// A source of data: a file that fills a branch of the data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceDef {
    pub id: String,
    pub format: SourceFormat,
    /// The data file, relative to the folder of `app.json`.
    pub file: String,
    /// The branch it fills: `/` for the root, or `/r1/…/rn`, names of containment references.
    pub target: String,
}

/// The format of a source.
// serde refuses any other value, and its error names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Json,
}

impl AppFile {
    /// Reads the text of an `app.json`, and checks its version.
    pub fn parse(text: &str) -> Result<AppFile, ConfigError> {
        let file: AppFile = serde_json::from_str(text).map_err(|error| ConfigError::Syntax {
            line: error.line(),
            column: error.column(),
            // serde_json names the key in its message ("unknown field `sourcse`", "missing
            // field `target`", "unknown variant `xml`"); the position is given apart.
            message: without_position(&error),
        })?;
        // serde checks the keys and the types, not the values: the version is checked here.
        if file.format_version != APP_FORMAT_VERSION {
            return Err(ConfigError::Version {
                found: file.format_version,
            });
        }
        Ok(file)
    }
}

/// An error of `app.json`: when reading it, or when checking it against its model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file is not valid JSON, or does not have the expected keys and types.
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    /// `formatVersion` is not the version this version of Ortelia reads.
    Version { found: String },
    /// `model.root` names no class of the model.
    UnknownRoot { class: String },
    /// `model.root` names an abstract class.
    AbstractRoot { class: String },
    /// A target is not `/` nor a path `/r1/…/rn` of names.
    InvalidTarget { source: String, target: String },
    /// A segment of a target names no containment reference of the class reached.
    UnknownSegment {
        source: String,
        segment: String,
        class: String,
    },
    /// An intermediate segment of a target is a list, which cannot be traversed.
    ListTraversed {
        source: String,
        segment: String,
        class: String,
    },
    /// An intermediate segment of a target has an abstract type: the object to create there
    /// would have no class.
    AbstractTraversed {
        source: String,
        segment: String,
        class: String,
    },
    /// Two sources have the same identifier.
    DuplicateSource { id: String },
    /// Two sources fill the same branch, or one fills a branch inside the other's.
    SourceConflict { first: String, second: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Syntax {
                line,
                column,
                message,
            } => write!(f, "line {line}, column {column}: {message}"),
            ConfigError::Version { found } => write!(
                f,
                "unsupported formatVersion `{found}` (expected `{APP_FORMAT_VERSION}`)"
            ),
            ConfigError::UnknownRoot { class } => {
                write!(f, "model.root: no class `{class}` in the model")
            }
            ConfigError::AbstractRoot { class } => {
                write!(f, "model.root: class `{class}` is abstract")
            }
            ConfigError::InvalidTarget { source, target } => write!(
                f,
                "source `{source}`: invalid target `{target}` (expected `/` or `/r1/.../rn`)"
            ),
            ConfigError::UnknownSegment {
                source,
                segment,
                class,
            } => write!(
                f,
                "source `{source}`: `{segment}` is not a containment reference of `{class}`"
            ),
            ConfigError::ListTraversed {
                source,
                segment,
                class,
            } => write!(
                f,
                "source `{source}`: `{class}.{segment}` is a list and cannot be traversed"
            ),
            ConfigError::AbstractTraversed {
                source,
                segment,
                class,
            } => write!(
                f,
                "source `{source}`: `{segment}` cannot be traversed, its class `{class}` is \
                 abstract"
            ),
            ConfigError::DuplicateSource { id } => {
                write!(f, "two sources have the identifier `{id}`")
            }
            ConfigError::SourceConflict { first, second } => write!(
                f,
                "sources `{first}` and `{second}` fill the same branch, or one inside the other"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}
