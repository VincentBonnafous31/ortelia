//! Layer 1: the format of `app.json`, the definition of an application.
//!
//! Depends only on `ortelia-model`. Two steps, as for a model file:
//! - [`AppFile::parse`] reads the file (format 0.4), and refuses a malformed one;
//! - [`AppFile::check`] checks it against its model (the languages, the class of the root, the
//!   branch of each source, the presentations, the view of the window, the labels) before any
//!   data is read, and gives a [`CheckedApp`], resolved to model identifiers.

mod app;
mod check;
mod labels;
mod pattern;

pub use app::{
    APP_FORMAT_VERSION, Align, AppFile, ColumnDef, ConfigError, ConfigWarning, Content, LabelEntry,
    LabelGroup, MAX_COLUMN_WIDTH, MAX_WINDOW_SIZE, MIN_COLUMN_WIDTH, MIN_WINDOW_SIZE, ModelSection,
    PresentationDef, SortDef, SortOrder, SourceDef, SourceFormat, TableDef, TableWidget, ViewDef,
    Widget, WindowDef,
};
pub use check::{
    CheckedApp, CheckedColumn, CheckedPresentation, CheckedSource, CheckedTable, CheckedView,
    CheckedWindow, TargetPath,
};
pub use labels::Labels;
pub use pattern::{DatePattern, Field, PatternError, PatternItem};
