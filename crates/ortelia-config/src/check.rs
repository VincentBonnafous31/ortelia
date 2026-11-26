//! The check of `app.json` against its model: the languages, the class of the root, the target
//! of each source and the conflicts between sources, the presentations, the window and its view,
//! and the labels. Every error is reported, not only the first one.

use std::collections::HashMap;

use ortelia_model::builder::{DataPath, PathSegment};
use ortelia_model::fragment::Shape;
use ortelia_model::metamodel::{AttrId, AttrType, ClassId, Model, RefId};

use crate::app::{
    Align, AppFile, ColumnDef, ConfigError, ConfigWarning, Content, MAX_COLUMN_WIDTH,
    MAX_WINDOW_SIZE, MIN_COLUMN_WIDTH, MIN_WINDOW_SIZE, PresentationDef, SortDef, SortOrder,
    SourceDef, SourceFormat, ViewDef, WindowDef,
};
use crate::labels::{Labels, check_labels};
use crate::pattern::DatePattern;

/// An application checked against its model, with names resolved to model identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedApp {
    pub id: String,
    pub name: String,
    /// The languages, the current one first.
    pub languages: Vec<String>,
    pub labels: Labels,
    /// The class of the root object.
    pub root: ClassId,
    /// The sources, in the order of `app.json`.
    pub sources: Vec<CheckedSource>,
    /// The presentations, by class.
    pub presentations: HashMap<ClassId, CheckedPresentation>,
    pub window: CheckedWindow,
    /// What is odd but does not prevent the opening, in the order found.
    pub warnings: Vec<ConfigWarning>,
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

/// The presentation of a class, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedPresentation {
    /// `None`: the columns are deduced from the model.
    pub table: Option<CheckedTable>,
}

/// The table of a presentation, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedTable {
    /// At least one column, each attribute once.
    pub columns: Vec<CheckedColumn>,
    /// The initial sort: attributes of columns of the table, each once.
    pub sort: Vec<(AttrId, SortOrder)>,
}

/// A column of a table, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedColumn {
    pub attr: AttrId,
    /// In points; `None`: the width of its type.
    pub width: Option<u32>,
    /// `None`: the alignment of its type.
    pub align: Option<Align>,
    /// Only for a date or a date-time.
    pub format: Option<DatePattern>,
}

/// The main window, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedWindow {
    /// The title of `window`, or the name of the application.
    pub title: String,
    /// In points.
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub view: CheckedView,
}

/// A view that shows the objects of a list in a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedView {
    pub id: String,
    /// The containment references from the root, all single but the last, a list.
    pub data: Vec<RefId>,
    /// The class shown: the type of the list.
    pub class: ClassId,
}

impl AppFile {
    /// Checks the application against its model: the root must be a concrete class, each
    /// target a branch of the model, and each source must fill its own branch; the view, the
    /// presentations and the labels must name what the model has. Gives all the errors found,
    /// in the order of the file.
    pub fn check(&self, model: &Model) -> Result<CheckedApp, Vec<ConfigError>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        check_languages(&self.languages, &mut errors);
        let labels = check_labels(
            &self.labels,
            &self.languages,
            model,
            &mut errors,
            &mut warnings,
        );

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

        let sources = check_sources(&self.sources, model, root, &mut errors);
        let presentations = check_presentations(&self.presentations, model, &mut errors);
        let window = check_window(&self.window, &self.name, model, root, &mut errors);

        match (root, window) {
            (Some(root), Some(window)) if errors.is_empty() => Ok(CheckedApp {
                id: self.id.clone(),
                name: self.name.clone(),
                languages: self.languages.clone(),
                labels,
                root,
                sources,
                presentations,
                window,
                warnings,
            }),
            _ => Err(errors),
        }
    }
}

fn check_languages(languages: &[String], errors: &mut Vec<ConfigError>) {
    if languages.is_empty() {
        errors.push(ConfigError::NoLanguage);
    }
    for (index, lang) in languages.iter().enumerate() {
        // Reported once, at the second occurrence.
        if languages[..index].iter().filter(|l| *l == lang).count() == 1 {
            errors.push(ConfigError::DuplicateLanguage { lang: lang.clone() });
        }
    }
}

fn check_sources(
    defs: &[SourceDef],
    model: &Model,
    root: Option<ClassId>,
    errors: &mut Vec<ConfigError>,
) -> Vec<CheckedSource> {
    let mut sources: Vec<CheckedSource> = Vec::new();
    for (index, source) in defs.iter().enumerate() {
        // An identifier used several times is reported once, at its second use.
        let earlier = defs[..index].iter().filter(|s| s.id == source.id).count();
        if earlier == 1 {
            errors.push(ConfigError::DuplicateSource {
                id: source.id.clone(),
            });
        }
        // Without a root, no target can be resolved.
        let Some(root) = root else { continue };
        match resolve_target(model, root, source) {
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
    sources
}

/// Why a path `/r1/…/rn` of the model cannot be followed.
enum PathError {
    /// Not `/r1/…/rn`: no leading `/`, or an empty segment.
    Invalid,
    UnknownSegment {
        segment: String,
        class: String,
    },
    ListTraversed {
        segment: String,
        class: String,
    },
    AbstractTraversed {
        segment: String,
        class: String,
    },
}

/// Follows a path `/r1/…/rn` of containment references from the class of the root: all the
/// references but the last must be single. With `abstract_allowed` false, a reference whose
/// type is abstract cannot be traversed either. Gives the references.
fn follow(
    model: &Model,
    root: ClassId,
    path: &str,
    abstract_allowed: bool,
) -> Result<Vec<RefId>, PathError> {
    // `/annuaire/personnes` gives `annuaire` and `personnes`; an empty segment (`//`, or a
    // trailing `/`) is refused.
    let segments: Vec<&str> = path
        .strip_prefix('/')
        .ok_or(PathError::Invalid)?
        .split('/')
        .collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(PathError::Invalid);
    }

    let mut class = root;
    let mut refs = Vec::new();
    for (i, &segment) in segments.iter().enumerate() {
        let class_name = model.class(class).name().to_owned();
        let Some(id) = model.class(class).reference(segment) else {
            return Err(PathError::UnknownSegment {
                segment: segment.to_owned(),
                class: class_name,
            });
        };
        refs.push(id);
        let reference = model.reference(id);
        if i + 1 == segments.len() {
            break;
        }
        if reference.is_many() {
            return Err(PathError::ListTraversed {
                segment: segment.to_owned(),
                class: class_name,
            });
        }
        class = reference.target();
        if !abstract_allowed && model.class(class).is_abstract() {
            return Err(PathError::AbstractTraversed {
                segment: segment.to_owned(),
                class: model.class(class).name().to_owned(),
            });
        }
    }
    Ok(refs)
}

/// Resolves the target of a source, from the class of the root.
fn resolve_target(
    model: &Model,
    root: ClassId,
    source: &SourceDef,
) -> Result<TargetPath, ConfigError> {
    if source.target == "/" {
        return Ok(TargetPath {
            refs: Vec::new(),
            shape: Shape::root(root),
        });
    }
    // An intermediate object is created by the application when it opens, if no source
    // provides it: there must be one at most, of a known class.
    let refs = follow(model, root, &source.target, false).map_err(|error| {
        let id = source.id.clone();
        match error {
            PathError::Invalid => ConfigError::InvalidTarget {
                source: id,
                target: source.target.clone(),
            },
            PathError::UnknownSegment { segment, class } => ConfigError::UnknownSegment {
                source: id,
                segment,
                class,
            },
            PathError::ListTraversed { segment, class } => ConfigError::ListTraversed {
                source: id,
                segment,
                class,
            },
            PathError::AbstractTraversed { segment, class } => ConfigError::AbstractTraversed {
                source: id,
                segment,
                class,
            },
        }
    })?;
    let last = *refs.last().expect("at least one segment");
    Ok(TargetPath {
        refs,
        shape: Shape::of(model, last),
    })
}

fn check_window(
    window: &WindowDef,
    name: &str,
    model: &Model,
    root: Option<ClassId>,
    errors: &mut Vec<ConfigError>,
) -> Option<CheckedWindow> {
    for (key, value) in [("width", window.width), ("height", window.height)] {
        if let Some(value) = value
            && !(MIN_WINDOW_SIZE..=MAX_WINDOW_SIZE).contains(&value)
        {
            errors.push(ConfigError::WindowSize { key, value });
        }
    }
    let Content::View(view) = &window.content;
    // Without a root, the data of the view cannot be followed.
    let view = check_view(view, model, root?)
        .map_err(|e| errors.push(e))
        .ok()?;
    Some(CheckedWindow {
        title: window.title.clone().unwrap_or_else(|| name.to_owned()),
        width: window.width,
        height: window.height,
        view,
    })
}

/// Checks the data of a table view: a path like a target, whose last reference is a list.
fn check_view(view: &ViewDef, model: &Model, root: ClassId) -> Result<CheckedView, ConfigError> {
    let id = view.id.clone();
    // The objects shown are not created by the view: an abstract type can be traversed, its
    // object having a concrete class.
    let data = follow(model, root, &view.data, true).map_err(|error| match error {
        PathError::Invalid => ConfigError::InvalidData {
            view: id.clone(),
            data: view.data.clone(),
        },
        PathError::UnknownSegment { segment, class } => ConfigError::UnknownDataSegment {
            view: id.clone(),
            segment,
            class,
        },
        PathError::ListTraversed { segment, class } => ConfigError::DataListTraversed {
            view: id.clone(),
            segment,
            class,
        },
        PathError::AbstractTraversed { .. } => unreachable!("abstract types are allowed"),
    })?;
    let last = model.reference(*data.last().expect("at least one segment"));
    if !last.is_many() {
        return Err(ConfigError::DataNotList {
            view: id,
            segment: last.name().to_owned(),
        });
    }
    Ok(CheckedView {
        id,
        class: last.target(),
        data,
    })
}

fn check_presentations(
    defs: &[PresentationDef],
    model: &Model,
    errors: &mut Vec<ConfigError>,
) -> HashMap<ClassId, CheckedPresentation> {
    let mut presentations = HashMap::new();
    for def in defs {
        let Some(class) = model.class_id(&def.class) else {
            errors.push(ConfigError::UnknownPresentationClass {
                class: def.class.clone(),
            });
            continue;
        };
        let table = def
            .table
            .as_ref()
            .and_then(|table| check_table(model, class, &table.columns, &table.sort, errors));
        if presentations
            .insert(class, CheckedPresentation { table })
            .is_some()
        {
            errors.push(ConfigError::DuplicatePresentation {
                class: def.class.clone(),
            });
        }
    }
    presentations
}

/// Checks the columns and the sort of the table of `class`. `None` if an error was found.
fn check_table(
    model: &Model,
    class: ClassId,
    columns: &[ColumnDef],
    sort: &[SortDef],
    errors: &mut Vec<ConfigError>,
) -> Option<CheckedTable> {
    let class_name = model.class(class).name();
    let error_count = errors.len();
    if columns.is_empty() {
        errors.push(ConfigError::NoColumn {
            class: class_name.to_owned(),
        });
    }
    let mut checked: Vec<CheckedColumn> = Vec::new();
    for column in columns {
        let named = |path: &str| (class_name.to_owned(), path.to_owned());
        let Some(attr) = model.class(class).attribute(&column.path) else {
            let (class, path) = named(&column.path);
            errors.push(ConfigError::UnknownColumn { class, path });
            continue;
        };
        if checked.iter().any(|c| c.attr == attr) {
            let (class, path) = named(&column.path);
            errors.push(ConfigError::DuplicateColumn { class, path });
            continue;
        }
        if let Some(width) = column.width
            && !(MIN_COLUMN_WIDTH..=MAX_COLUMN_WIDTH).contains(&width)
        {
            let (class, path) = named(&column.path);
            errors.push(ConfigError::ColumnWidth { class, path, width });
        }
        let format = column.format.as_ref().and_then(|pattern| {
            let with_time = match model.attribute(attr).attr_type() {
                AttrType::Date => false,
                AttrType::DateTime => true,
                _ => {
                    let (class, path) = named(&column.path);
                    errors.push(ConfigError::FormatNotDate { class, path });
                    return None;
                }
            };
            DatePattern::parse(pattern, with_time)
                .map_err(|error| {
                    let (class, path) = named(&column.path);
                    errors.push(ConfigError::InvalidPattern {
                        class,
                        path,
                        pattern: pattern.clone(),
                        error,
                    });
                })
                .ok()
        });
        checked.push(CheckedColumn {
            attr,
            width: column.width,
            align: column.align,
            format,
        });
    }

    let mut criteria: Vec<(AttrId, SortOrder)> = Vec::new();
    for criterion in sort {
        let column = model
            .class(class)
            .attribute(&criterion.path)
            .filter(|&attr| checked.iter().any(|c| c.attr == attr));
        let Some(attr) = column else {
            errors.push(ConfigError::UnknownSortColumn {
                class: class_name.to_owned(),
                path: criterion.path.clone(),
            });
            continue;
        };
        if criteria.iter().any(|&(a, _)| a == attr) {
            errors.push(ConfigError::DuplicateSort {
                class: class_name.to_owned(),
                path: criterion.path.clone(),
            });
            continue;
        }
        criteria.push((attr, criterion.order));
    }
    (errors.len() == error_count).then_some(CheckedTable {
        columns: checked,
        sort: criteria,
    })
}
