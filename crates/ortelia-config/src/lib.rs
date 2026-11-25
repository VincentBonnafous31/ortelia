//! Layer 1: the format of `app.json`, the definition of an application.
//!
//! Depends only on `ortelia-model`. Two steps, as for a model file:
//! - [`AppFile::parse`] reads the file (format 0.3), and refuses a malformed one;
//! - [`AppFile::check`] checks it against its model (the class of the root, the branch of each
//!   source) before any data is read, and gives a [`CheckedApp`], resolved to model identifiers.
//!
//! Only the model and the sources are described for now; the window layout, the views and the
//! labels will come with the increments that design them.

mod app;
mod check;

pub use app::{APP_FORMAT_VERSION, AppFile, ConfigError, ModelSection, SourceDef, SourceFormat};
pub use check::{CheckedApp, CheckedSource, TargetPath};
