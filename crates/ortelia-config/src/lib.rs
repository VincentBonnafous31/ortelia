//! Layer 1: the format of `app.json`, the definition of an application.
//!
//! Depends only on `ortelia-model`. Two steps, as for a model file:
//! - [`AppFile::parse`] reads the file (format 0.6), and refuses a malformed one;
//! - [`AppFile::check`] checks it against its model (the languages, the class of the root, the
//!   branch of each source, the presentations and their forms, the splits and views of the
//!   window, the labels) before any data is read, and gives a [`CheckedApp`], resolved to model
//!   identifiers.

mod app;
mod check;
mod form;
mod labels;
mod pattern;

pub use app::{
    APP_FORMAT_VERSION, Align, AppFile, ColumnDef, ConfigError, ConfigWarning, Content, Direction,
    EmptyWidget, FormWidget, LabelEntry, LabelGroup, MAX_COLUMN_WIDTH, MAX_WINDOW_SIZE,
    MIN_COLUMN_WIDTH, MIN_WINDOW_SIZE, ModelSection, PresentationDef, SortDef, SortOrder,
    SourceDef, SourceFormat, SplitDef, TableDef, TableWidget, ViewDef, Widget, WindowDef,
};
pub use check::{
    CheckedApp, CheckedColumn, CheckedContent, CheckedPresentation, CheckedSource, CheckedSplit,
    CheckedTable, CheckedTableView, CheckedView, CheckedWindow, TargetPath,
};
pub use form::{
    CellDef, CheckedCell, CheckedElement, CheckedField, CheckedGrid, CheckedWidget,
    DEFAULT_TEXT_AREA_ROWS, DateFieldDef, Element, FieldDef, FieldWidget, FormDef, GridColumnDef,
    GroupDef, NoOptions, SeparatorDef, TEXT_AREA_ROWS, TextAreaDef, TextDef, accepts, default_grid,
    default_widget,
};
pub use labels::Labels;
pub use pattern::{DatePattern, Field, PatternError, PatternItem};
