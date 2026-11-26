//! Opening an application: its `app.json`, its model, its document, and the sources that fill
//! the branches of the document.
//!
//! [`AppContext::open`] reads and checks `app.json` and the model, creates the document with its
//! root and the skeleton of its branches (the intermediate objects of the targets), then reads
//! each source and integrates it into its branch. A source that fails does not prevent the
//! opening: its state gives its error, and its branch stays as the skeleton made it.
//!
//! The sources are read one after the other, in the calling thread. The day they are read in the
//! background, reading (in another thread: a `Fragment` is `Send`) and integrating (in the main
//! thread, in the order of `app.json`) will be split.

use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ortelia_config::{AppFile, CheckedApp, CheckedSource, ConfigError, ConfigWarning};
use ortelia_model::builder::{Diagnostic, LoadFailure};
use ortelia_model::document::{Document, IntegrateError, Target};
use ortelia_model::metamodel::{Model, ModelLoadError, NamedModelFile};
use ortelia_model::store::{ObjectId, Position};
use tracing::{info, warn};

use crate::json::read_json;

/// An open application.
#[derive(Debug)]
pub struct AppContext {
    app: CheckedApp,
    document: Document,
    sources: Vec<SourceState>,
}

/// A source of the application, and what became of it at the opening.
#[derive(Debug)]
pub struct SourceState {
    pub id: String,
    /// The data file, resolved from the folder of `app.json`.
    pub file: PathBuf,
    /// The branch of the document that the source fills.
    pub target: Target,
    pub status: SourceStatus,
}

#[derive(Debug)]
pub enum SourceStatus {
    /// The source was read and integrated, at revision `integrated_at` of the document.
    Loaded {
        integrated_at: u64,
        /// The first diagnostics, and their total.
        diagnostics: Vec<Diagnostic>,
        diagnostic_count: usize,
    },
    Failed(SourceError),
}

/// Why a source could not be loaded.
#[derive(Debug)]
pub enum SourceError {
    /// The file could not be opened.
    Io { file: PathBuf, message: String },
    /// The data of the file were refused.
    Data(LoadFailure),
    /// The fragment read does not fit its branch. Cannot happen for a source checked against
    /// the model; kept rather than a panic.
    Integrate(IntegrateError),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Io { file, message } => write!(f, "{}: {message}", file.display()),
            SourceError::Data(failure) => write!(f, "{failure}"),
            SourceError::Integrate(error) => write!(f, "{error}"),
        }
    }
}

/// An error that prevents an application from opening. Each one names the file in cause.
#[derive(Debug)]
pub enum OpenError {
    /// `app.json` or a model file could not be read.
    Io { file: PathBuf, message: String },
    /// `app.json` is malformed, or does not fit its model.
    App {
        file: PathBuf,
        errors: Vec<ConfigError>,
    },
    /// A model file is malformed, or the model does not compile; the error names the file.
    Model(ModelLoadError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Io { file, message } => write!(f, "{}: {message}", file.display()),
            OpenError::App { file, errors } => {
                write!(f, "{}: ", file.display())?;
                for (i, error) in errors.iter().enumerate() {
                    if i > 0 {
                        f.write_str("; ")?;
                    }
                    write!(f, "{error}")?;
                }
                Ok(())
            }
            OpenError::Model(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for OpenError {}

impl AppContext {
    /// Opens the application of `app.json` at `path`.
    pub fn open(path: &Path) -> Result<AppContext, OpenError> {
        let app_error = |errors| OpenError::App {
            file: path.to_owned(),
            errors,
        };
        let app = AppFile::parse(&read_text(path)?).map_err(|e| app_error(vec![e]))?;
        // The files of `app.json` are relative to its folder.
        let folder = path.parent().unwrap_or(Path::new(""));
        let files = app
            .model
            .files
            .iter()
            .map(|file| {
                let path = folder.join(file);
                Ok(NamedModelFile {
                    text: read_text(&path)?,
                    name: path.display().to_string(),
                })
            })
            .collect::<Result<Vec<_>, OpenError>>()?;
        let model = Arc::new(Model::load(&files).map_err(OpenError::Model)?);
        let app = app.check(&model).map_err(app_error)?;
        for warning in &app.warnings {
            // Names of the configuration (label keys), never data of the user.
            warn!(file = %path.display(), "configuration: {warning}");
        }

        let mut document =
            Document::new(model.clone(), app.root).expect("the root was checked to be concrete");
        let targets = create_skeleton(&mut document, &app);
        let sources = app
            .sources
            .iter()
            .zip(targets)
            .map(|(source, target)| load_source(&mut document, &model, folder, source, target))
            .collect();
        info!(
            app = %app.id,
            sources = app.sources.len(),
            "application opened"
        );
        Ok(AppContext {
            app,
            document,
            sources,
        })
    }

    /// The application, as checked against its model.
    pub fn app(&self) -> &CheckedApp {
        &self.app
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The sources, in the order of `app.json`.
    pub fn sources(&self) -> &[SourceState] {
        &self.sources
    }

    /// The warnings of the check of `app.json`: odd but harmless, like a label key that names
    /// nothing of the model.
    pub fn warnings(&self) -> &[ConfigWarning] {
        &self.app.warnings
    }
}

fn read_text(path: &Path) -> Result<String, OpenError> {
    std::fs::read_to_string(path).map_err(|error| OpenError::Io {
        file: path.to_owned(),
        message: error.to_string(),
    })
}

/// Creates, in one transaction, the intermediate objects of the targets, each of the declared
/// type of its reference, and returns the branch of each source. Two targets that share a
/// beginning (`/a/b` and `/a/c`) share its objects.
fn create_skeleton(document: &mut Document, app: &CheckedApp) -> Vec<Target> {
    let mut edit = document.edit();
    let root = edit.root();
    app.sources
        .iter()
        .map(|source| {
            let Some((&last, intermediates)) = source.target.refs.split_last() else {
                return Target::Root;
            };
            let mut parent: ObjectId = root;
            for &reference in intermediates {
                // A single reference (checked): its child, if already created for another
                // target, or a new one.
                parent = match edit.store().children(parent, reference).first() {
                    Some(&child) => child,
                    None => {
                        let class = edit.store().model().reference(reference).target();
                        edit.create_child(parent, reference, Position::End, class)
                            .expect("an intermediate class was checked to be concrete")
                    }
                };
            }
            Target::Reference {
                parent,
                reference: last,
            }
        })
        .collect()
    // `edit` is dropped here: the skeleton is recorded as one transaction.
}

/// Reads a source and integrates it into its branch, in one transaction.
fn load_source(
    document: &mut Document,
    model: &Arc<Model>,
    folder: &Path,
    source: &CheckedSource,
    target: Target,
) -> SourceState {
    let file = folder.join(&source.file);
    let status = match read_and_integrate(document, model, &file, source, target) {
        Ok((objects, integrated_at, diagnostics, diagnostic_count)) => {
            info!(
                source = %source.id,
                file = %file.display(),
                objects,
                diagnostics = diagnostic_count,
                "source loaded"
            );
            SourceStatus::Loaded {
                integrated_at,
                diagnostics,
                diagnostic_count,
            }
        }
        Err(error) => {
            // The `Display` form of the errors, which never holds a value of the data.
            let errors = match &error {
                SourceError::Data(failure) => failure.error_count,
                _ => 1,
            };
            warn!(
                source = %source.id,
                file = %file.display(),
                errors,
                "source failed: {error}"
            );
            SourceStatus::Failed(error)
        }
    };
    SourceState {
        id: source.id.clone(),
        file,
        target,
        status,
    }
}

/// What a source loaded gives: its number of objects, the revision of its integration, and its
/// diagnostics with their total.
type Integrated = (usize, u64, Vec<Diagnostic>, usize);

fn read_and_integrate(
    document: &mut Document,
    model: &Arc<Model>,
    file: &Path,
    source: &CheckedSource,
    target: Target,
) -> Result<Integrated, SourceError> {
    let reader = File::open(file).map_err(|error| SourceError::Io {
        file: file.to_owned(),
        message: error.to_string(),
    })?;
    let base = source.target.data_path(model);
    let loaded =
        read_json(reader, model.clone(), source.target.shape, base).map_err(SourceError::Data)?;
    let objects = loaded.fragment.object_count();
    document
        .edit()
        .replace(target, loaded.fragment)
        .map_err(SourceError::Integrate)?;
    Ok((
        objects,
        document.revision(),
        loaded.diagnostics,
        loaded.diagnostic_count,
    ))
}

#[cfg(test)]
mod tests;
