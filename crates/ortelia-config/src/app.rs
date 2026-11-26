//! The file `app.json`, as read by serde, and the errors of `app.json`.

use std::fmt;

use ortelia_model::metamodel::without_position;
use serde::{Deserialize, Serialize};

use crate::pattern::PatternError;

/// The version of the format of `app.json` that this version of Ortelia reads.
pub const APP_FORMAT_VERSION: &str = "0.4";

/// The range of the width and the height of the window, in points.
pub const MIN_WINDOW_SIZE: u32 = 200;
pub const MAX_WINDOW_SIZE: u32 = 10_000;
/// The range of the width of a column, in points.
pub const MIN_COLUMN_WIDTH: u32 = 20;
pub const MAX_COLUMN_WIDTH: u32 = 2_000;

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
    /// The languages of the application, as codes (`fr`, `en`). The first one is the current
    /// language.
    pub languages: Vec<String>,
    /// The labels of the classes, attributes and enum literals, one group per language.
    #[serde(default)]
    pub labels: Vec<LabelGroup>,
    pub model: ModelSection,
    #[serde(default)]
    pub sources: Vec<SourceDef>,
    /// How the objects of a class are presented: its table.
    #[serde(default)]
    pub presentations: Vec<PresentationDef>,
    /// The main window.
    pub window: WindowDef,
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

/// The labels of one language.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LabelGroup {
    /// The language, one of `languages`.
    pub lang: String,
    pub entries: Vec<LabelEntry>,
}

/// A label: the text shown for a class (`Personne`), an attribute (`Personne.nom`) or an enum
/// literal (`SituationFamiliale.MARIE`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LabelEntry {
    pub key: String,
    pub text: String,
}

/// The presentation of a class: how its objects are shown.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PresentationDef {
    /// The name of the class. A subclass without presentation takes the one of its nearest
    /// super class.
    pub class: String,
    /// The table of the objects of the class. Without it, the columns are the attributes of the
    /// class, in the order of the model.
    #[serde(default)]
    pub table: Option<TableDef>,
}

/// The table of a class: its columns and its initial sort.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TableDef {
    pub columns: Vec<ColumnDef>,
    /// The initial sort: columns of the table, the first one first.
    #[serde(default)]
    pub sort: Vec<SortDef>,
}

/// A column of a table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ColumnDef {
    /// The name of an attribute of the class, inherited ones included.
    pub path: String,
    /// The width of the column, in points (20 to 2000). By default, a width for its type.
    #[serde(default)]
    pub width: Option<u32>,
    /// By default, numbers are aligned to the right, booleans and dates centred, and texts to
    /// the left.
    #[serde(default)]
    pub align: Option<Align>,
    /// For a date or a date-time, an LDML pattern: `dd/MM/yyyy`, `d MMMM yyyy HH:mm`…
    #[serde(default)]
    pub format: Option<String>,
}

/// The alignment of the values of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A criterion of the initial sort.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SortDef {
    /// The path of a column of the table.
    pub path: String,
    #[serde(default)]
    pub order: SortOrder,
}

/// The direction of a sort.
// `Serialize`: the presentation writes it in the session state file, with the same names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending, the default.
    #[default]
    Asc,
    Desc,
}

/// The main window.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WindowDef {
    /// The title of the window; by default, the name of the application.
    #[serde(default)]
    pub title: Option<String>,
    /// The initial width, in points (200 to 10000).
    #[serde(default)]
    pub width: Option<u32>,
    /// The initial height, in points (200 to 10000).
    #[serde(default)]
    pub height: Option<u32>,
    pub content: Content,
}

/// The content of a window, chosen by `_type_`.
//
// An internally tagged enum: serde reads `_type_` and gives the other keys to the variant, like
// `@JsonTypeInfo(use = NAME, property = "_type_")` in Jackson. Containers (`Split`, tab stacks)
// will be other variants.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(tag = "_type_", deny_unknown_fields)]
pub enum Content {
    /// A view: a widget that shows the data of a path.
    View(ViewDef),
}

/// A view: a widget that shows the data of a path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ViewDef {
    /// The identifier of the view.
    pub id: String,
    pub widget: Widget,
    /// The data shown: `/r1/…/rn`, containment references from the root, the last one a list.
    pub data: String,
}

/// The widget of a view, chosen by `_type_`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(tag = "_type_", deny_unknown_fields)]
pub enum Widget {
    /// A table of the objects of a list, presented as their class says.
    Table(TableWidget),
}

/// The options of a table proper to its view: none yet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TableWidget {}

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
    /// `languages` is empty.
    NoLanguage,
    /// A language is declared twice in `languages`.
    DuplicateLanguage { lang: String },
    /// The width or the height of the window is out of its range.
    WindowSize { key: &'static str, value: u32 },
    /// The data of a view is not a path `/r1/…/rn` of names.
    InvalidData { view: String, data: String },
    /// A segment of the data of a view names no containment reference of the class reached.
    UnknownDataSegment {
        view: String,
        segment: String,
        class: String,
    },
    /// An intermediate segment of the data of a view is a list.
    DataListTraversed {
        view: String,
        segment: String,
        class: String,
    },
    /// The last segment of the data of a view is not a list.
    DataNotList { view: String, segment: String },
    /// A presentation names no class of the model.
    UnknownPresentationClass { class: String },
    /// Two presentations name the same class.
    DuplicatePresentation { class: String },
    /// The table of a presentation has no column.
    NoColumn { class: String },
    /// A column names no attribute of the class.
    UnknownColumn { class: String, path: String },
    /// Two columns name the same attribute.
    DuplicateColumn { class: String, path: String },
    /// The width of a column is out of its range.
    ColumnWidth {
        class: String,
        path: String,
        width: u32,
    },
    /// A column that is neither a date nor a date-time has a format.
    FormatNotDate { class: String, path: String },
    /// The format of a date column is not a valid pattern.
    InvalidPattern {
        class: String,
        path: String,
        pattern: String,
        error: PatternError,
    },
    /// A criterion of the sort names no column of the table.
    UnknownSortColumn { class: String, path: String },
    /// A column appears twice in the sort.
    DuplicateSort { class: String, path: String },
    /// A group of labels is in a language not in `languages`.
    UndeclaredLabelLanguage { lang: String },
    /// Two groups of labels have the same language.
    DuplicateLabelGroup { lang: String },
    /// A key appears twice in the group of a language.
    DuplicateLabel { lang: String, key: String },
}

/// Something odd in `app.json` that does not prevent the opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    /// A label key names no class, attribute or enum literal of the model.
    UnknownLabelKey { lang: String, key: String },
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigWarning::UnknownLabelKey { lang, key } => write!(
                f,
                "label `{key}` ({lang}) names no class, attribute or enum literal of the model"
            ),
        }
    }
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
            ConfigError::NoLanguage => f.write_str("languages: at least one language is needed"),
            ConfigError::DuplicateLanguage { lang } => {
                write!(f, "languages: `{lang}` is declared twice")
            }
            ConfigError::WindowSize { key, value } => write!(
                f,
                "window.{key}: {value} is out of range ({MIN_WINDOW_SIZE} to {MAX_WINDOW_SIZE})"
            ),
            ConfigError::InvalidData { view, data } => write!(
                f,
                "view `{view}`: invalid data `{data}` (expected `/r1/.../rn`, the last one a list)"
            ),
            ConfigError::UnknownDataSegment {
                view,
                segment,
                class,
            } => write!(
                f,
                "view `{view}`: `{segment}` is not a containment reference of `{class}`"
            ),
            ConfigError::DataListTraversed {
                view,
                segment,
                class,
            } => write!(
                f,
                "view `{view}`: `{class}.{segment}` is a list and cannot be traversed"
            ),
            ConfigError::DataNotList { view, segment } => {
                write!(f, "view `{view}`: `{segment}` is not a list")
            }
            ConfigError::UnknownPresentationClass { class } => {
                write!(
                    f,
                    "presentation of `{class}`: no class `{class}` in the model"
                )
            }
            ConfigError::DuplicatePresentation { class } => {
                write!(f, "two presentations of the class `{class}`")
            }
            ConfigError::NoColumn { class } => {
                write!(f, "presentation of `{class}`: the table has no column")
            }
            ConfigError::UnknownColumn { class, path } => write!(
                f,
                "presentation of `{class}`: column `{path}` is not an attribute of `{class}`"
            ),
            ConfigError::DuplicateColumn { class, path } => {
                write!(
                    f,
                    "presentation of `{class}`: column `{path}` appears twice"
                )
            }
            ConfigError::ColumnWidth { class, path, width } => write!(
                f,
                "presentation of `{class}`: column `{path}`: width {width} is out of range \
                 ({MIN_COLUMN_WIDTH} to {MAX_COLUMN_WIDTH})"
            ),
            ConfigError::FormatNotDate { class, path } => write!(
                f,
                "presentation of `{class}`: column `{path}`: a format applies only to a date or \
                 a date-time"
            ),
            ConfigError::InvalidPattern {
                class,
                path,
                pattern,
                error,
            } => write!(
                f,
                "presentation of `{class}`: column `{path}`: invalid format `{pattern}`: {error}"
            ),
            ConfigError::UnknownSortColumn { class, path } => write!(
                f,
                "presentation of `{class}`: the sort names `{path}`, which is not a column of the \
                 table"
            ),
            ConfigError::DuplicateSort { class, path } => {
                write!(
                    f,
                    "presentation of `{class}`: `{path}` appears twice in the sort"
                )
            }
            ConfigError::UndeclaredLabelLanguage { lang } => {
                write!(f, "labels: language `{lang}` is not in `languages`")
            }
            ConfigError::DuplicateLabelGroup { lang } => {
                write!(f, "labels: two groups for the language `{lang}`")
            }
            ConfigError::DuplicateLabel { lang, key } => {
                write!(
                    f,
                    "labels: key `{key}` appears twice in the language `{lang}`"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}
