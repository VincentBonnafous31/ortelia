//! The check of `app.json` against its model: the class of the root, the target of each source,
//! and the conflicts between sources. Every error is reported, not only the first one.

use ortelia_model::builder::{DataPath, PathSegment};
use ortelia_model::fragment::Shape;
use ortelia_model::metamodel::{ClassId, Model, RefId};

use crate::app::{AppFile, ConfigError, SourceDef, SourceFormat};

/// An application checked against its model, with names resolved to model identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedApp {
    pub id: String,
    pub name: String,
    /// The class of the root object.
    pub root: ClassId,
    /// The sources, in the order of `app.json`.
    pub sources: Vec<CheckedSource>,
}

/// A source checked against the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedSource {
    pub id: String,
    pub format: SourceFormat,
    /// The data file, relative to the folder of `app.json`.
    pub file: String,
    pub target: TargetPath,
}

/// The branch a source fills, as a static path in the model: the containment references from
/// the root (all single but the last), and the shape of the branch. `/` has no reference, and the
/// shape of the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPath {
    pub refs: Vec<RefId>,
    pub shape: Shape,
}

impl TargetPath {
    /// The path of the branch in the data, which starts the path of the errors of its file:
    /// the names of its references.
    pub fn data_path(&self, model: &Model) -> DataPath {
        DataPath(
            self.refs
                .iter()
                .map(|&r| PathSegment::Name(model.reference(r).name().to_owned()))
                .collect(),
        )
    }

    /// `true` if this branch is `other`, or contains it: the references of this path start
    /// the references of the other one. A `RefId` names its declaring class, so two references
    /// with the same name in two classes are told apart.
    fn contains(&self, other: &TargetPath) -> bool {
        other.refs.starts_with(&self.refs)
    }
}

impl AppFile {
    /// Checks the application against its model: the root must be a concrete class, each
    /// target a branch of the model, and each source must fill its own branch. Gives all the
    /// errors found, in the order of the file.
    pub fn check(&self, model: &Model) -> Result<CheckedApp, Vec<ConfigError>> {
        let mut errors = Vec::new();
        let root = match model.class_id(&self.model.root) {
            None => {
                errors.push(ConfigError::UnknownRoot {
                    class: self.model.root.clone(),
                });
                None
            }
            Some(class) if model.class(class).is_abstract() => {
                errors.push(ConfigError::AbstractRoot {
                    class: self.model.root.clone(),
                });
                None
            }
            Some(class) => Some(class),
        };

        let mut sources: Vec<CheckedSource> = Vec::new();
        for (index, source) in self.sources.iter().enumerate() {
            // An identifier used several times is reported once, at its second use.
            let earlier = self.sources[..index]
                .iter()
                .filter(|s| s.id == source.id)
                .count();
            if earlier == 1 {
                errors.push(ConfigError::DuplicateSource {
                    id: source.id.clone(),
                });
            }
            // Without a root, no target can be resolved.
            let Some(root) = root else { continue };
            match resolve(model, root, source) {
                Ok(target) => {
                    for other in &sources {
                        if other.target.contains(&target) || target.contains(&other.target) {
                            errors.push(ConfigError::SourceConflict {
                                first: other.id.clone(),
                                second: source.id.clone(),
                            });
                        }
                    }
                    sources.push(CheckedSource {
                        id: source.id.clone(),
                        format: source.format,
                        file: source.file.clone(),
                        target,
                    });
                }
                Err(error) => errors.push(error),
            }
        }

        match root {
            Some(root) if errors.is_empty() => Ok(CheckedApp {
                id: self.id.clone(),
                name: self.name.clone(),
                root,
                sources,
            }),
            _ => Err(errors),
        }
    }
}

/// Resolves the target of a source, from the class of the root.
fn resolve(model: &Model, root: ClassId, source: &SourceDef) -> Result<TargetPath, ConfigError> {
    let invalid = || ConfigError::InvalidTarget {
        source: source.id.clone(),
        target: source.target.clone(),
    };
    if source.target == "/" {
        return Ok(TargetPath {
            refs: Vec::new(),
            shape: Shape::root(root),
        });
    }
    // `/annuaire/personnes` gives `annuaire` and `personnes`; an empty segment (`//`, or a
    // trailing `/`) is refused.
    let segments: Vec<&str> = source
        .target
        .strip_prefix('/')
        .ok_or_else(invalid)?
        .split('/')
        .collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(invalid());
    }

    let mut class = root;
    let mut refs = Vec::new();
    for (i, &segment) in segments.iter().enumerate() {
        let class_name = model.class(class).name().to_owned();
        let Some(id) = model.class(class).reference(segment) else {
            return Err(ConfigError::UnknownSegment {
                source: source.id.clone(),
                segment: segment.to_owned(),
                class: class_name,
            });
        };
        refs.push(id);
        let reference = model.reference(id);
        let last = i + 1 == segments.len();
        if last {
            break;
        }
        // An intermediate object is created by the application when it opens, if no source
        // provides it: there must be one at most, of a known class.
        if reference.is_many() {
            return Err(ConfigError::ListTraversed {
                source: source.id.clone(),
                segment: segment.to_owned(),
                class: class_name,
            });
        }
        class = reference.target();
        if model.class(class).is_abstract() {
            return Err(ConfigError::AbstractTraversed {
                source: source.id.clone(),
                segment: segment.to_owned(),
                class: model.class(class).name().to_owned(),
            });
        }
    }
    let last = *refs.last().expect("at least one segment");
    Ok(TargetPath {
        refs,
        shape: Shape::of(model, last),
    })
}
