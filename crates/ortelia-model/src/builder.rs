//! The builder: the only API through which a reader creates data, for every reader, present and
//! future (JSON, XML, reading plugins that know the model).
//!
//! It fills a new [`Fragment`] for a branch of a given [`Shape`]: the objects of the branch, its
//! *heads*, then their descendants. The reader calls it in file order, like a SAX
//! `ContentHandler`: beginning of an object, values of its attributes, end of the object. A
//! stack of frames gives the context, and the path of an error, which is only computed when
//! there is an error.
//!
//! The builder does not stop at the first error: it records it, ignores the value or the object
//! refused, and goes on, so that a single load reports every error of a file (the first
//! [`MAX_REPORTED`] of them, and their total). An object without a value for a required
//! attribute is kept, with a [`Diagnostic`]. [`Builder::finish`] then gives the fragment, or the
//! errors.
//!
//! A value given as text is converted according to the type of its attribute ([`set_text`]):
//! it is the only text conversion of the system.
//!
//! [`set_text`]: Builder::set_text

use std::fmt;
use std::sync::Arc;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::fragment::{Fragment, Shape};
use crate::metamodel::{AttrId, AttrType, ClassId, Model, RefId};
use crate::store::{ObjectId, Position, Store, StoreError};
use crate::value::Value;

/// How many errors, and separately how many diagnostics, a load keeps. The others are only
/// counted.
pub const MAX_REPORTED: usize = 100;

/// How many characters of an offending value an error keeps.
const MAX_VALUE_CHARS: usize = 40;

/// Where the builder is in the tree being built.
#[derive(Debug)]
enum Frame {
    /// The bottom of the stack, always there: the heads of the fragment. `count`: heads begun so
    /// far, refused ones included.
    Heads {
        count: usize,
    },
    Object {
        id: ObjectId,
        class: ClassId,
    },
    /// `count`: objects begun so far in this reference, refused ones included, so that a path
    /// gives the position in the file.
    Reference {
        reference: RefId,
        count: usize,
    },
}

/// The first [`MAX_REPORTED`] items of a kind, and how many there were.
#[derive(Debug)]
struct Reported<T> {
    kept: Vec<T>,
    total: usize,
}

impl<T> Reported<T> {
    fn new() -> Reported<T> {
        Reported {
            kept: Vec::new(),
            total: 0,
        }
    }

    /// Counts an item, and says whether to keep it: fewer than [`MAX_REPORTED`] are kept so far.
    /// The caller builds the item only then, so that past the cap a systematic error in a large
    /// file costs no allocation.
    fn count(&mut self) -> bool {
        self.total += 1;
        self.kept.len() < MAX_REPORTED
    }
}

/// Fills a new fragment, in file order.
#[derive(Debug)]
pub struct Builder {
    store: Store,
    shape: Shape,
    /// The path of the branch in the document, which starts the path of every error.
    base: DataPath,
    heads: Vec<ObjectId>,
    stack: Vec<Frame>,
    errors: Reported<LoadError>,
    diagnostics: Reported<Diagnostic>,
}

impl Builder {
    /// A builder that fills a new, empty fragment for a branch of shape `shape`, at path `base`
    /// in the document (`/annuaire/personnes`, or the empty path for the root).
    pub fn new(model: Arc<Model>, shape: Shape, base: DataPath) -> Builder {
        Builder {
            store: Store::new(model),
            shape,
            base,
            heads: Vec::new(),
            stack: vec![Frame::Heads { count: 0 }],
            errors: Reported::new(),
            diagnostics: Reported::new(),
        }
    }

    pub fn model(&self) -> &Model {
        self.store.model()
    }

    /// The shape of the branch being read.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Begins an object of the branch itself, of class `class`: the class of the shape or one of
    /// its subclasses, and concrete. A single shape holds one object at most.
    ///
    /// Returns `false` if the object is refused: the error is recorded, and the reader must skip
    /// the content of the object, without calling [`end_object`](Builder::end_object).
    ///
    /// # Panics
    /// If an object or a reference is begun and not ended.
    pub fn begin_head(&mut self, class: ClassId) -> bool {
        assert!(
            matches!(self.stack.last(), Some(Frame::Heads { .. })),
            "a head is begun inside an object"
        );
        let count = self.count_object();
        if !self.shape.many && count > 1 {
            let reference = self.branch_name();
            self.report(LoadErrorKind::ReferenceAlreadySet { reference });
            return false;
        }
        match self.store.create_head(class, self.shape.class) {
            Ok(id) => {
                self.heads.push(id);
                self.stack.push(Frame::Object { id, class });
                true
            }
            Err(error) => {
                self.report_store_error(error);
                false
            }
        }
    }

    /// Begins the children of a containment reference of the current object.
    ///
    /// A reference begun a second time (a key repeated in a JSON object) appends to its
    /// children: the positions of the paths continue after them.
    ///
    /// # Panics
    /// If the current frame is not an object.
    pub fn begin_reference(&mut self, reference: RefId) {
        let Some(Frame::Object { id, .. }) = self.stack.last() else {
            panic!("no current object");
        };
        let count = self.store.children(*id, reference).len();
        self.stack.push(Frame::Reference { reference, count });
    }

    /// Begins an object of class `class`, at the end of the current reference.
    ///
    /// Returns `false` if the object is refused, as [`begin_head`](Builder::begin_head) does.
    ///
    /// # Panics
    /// If the current frame is not a reference.
    pub fn begin_object(&mut self, class: ClassId) -> bool {
        let reference = self.current_reference();
        self.count_object();
        let parent = self.parent_object();
        match self
            .store
            .create_child(parent, reference, Position::End, class)
        {
            Ok(id) => {
                self.stack.push(Frame::Object { id, class });
                true
            }
            Err(error) => {
                self.report_store_error(error);
                false
            }
        }
    }

    /// Records that the next object of the current reference, or the next head, cannot be
    /// created, with an error found by the reader, such as an unknown class name. The object
    /// still counts in its list, so that the following paths keep the positions of the file.
    pub fn fail_object(&mut self, kind: LoadErrorKind) {
        self.count_object();
        self.report(kind);
    }

    /// Ends the current object, and records a diagnostic for each of its required attributes
    /// that has no value.
    ///
    /// # Panics
    /// If the current frame is not an object.
    pub fn end_object(&mut self) {
        let (id, class) = match self.stack.last() {
            Some(Frame::Object { id, class }) => (*id, *class),
            _ => panic!("no current object"),
        };
        // Usually empty: nothing to do for a class without required attributes. `model` borrows
        // the field `self.store` only, so `self.diagnostics` can be changed meanwhile.
        let model = self.store.model();
        for &attr in model.class(class).required_attributes() {
            if self.store.get(id, attr) == Value::Unset && self.diagnostics.count() {
                let diagnostic = Diagnostic {
                    path: self.path(),
                    kind: DiagnosticKind::MissingRequired {
                        attribute: model.attribute(attr).name().to_owned(),
                    },
                };
                self.diagnostics.kept.push(diagnostic);
            }
        }
        self.stack.pop();
    }

    /// Ends the current reference.
    pub fn end_reference(&mut self) {
        assert!(
            matches!(self.stack.pop(), Some(Frame::Reference { .. })),
            "no current reference"
        );
    }

    /// The class of the current object, which lets a reader resolve its keys.
    ///
    /// # Panics
    /// If the current frame is not an object.
    pub fn current_class(&self) -> ClassId {
        match self.stack.last() {
            Some(Frame::Object { class, .. }) => *class,
            _ => panic!("no current object"),
        }
    }

    /// The declared type of the current reference.
    ///
    /// # Panics
    /// If the current frame is not a reference.
    pub fn current_reference(&self) -> RefId {
        match self.stack.last() {
            Some(Frame::Reference { reference, .. }) => *reference,
            _ => panic!("no current reference"),
        }
    }

    /// Sets an attribute of the current object from a text, converted according to its type:
    /// a number for `Int` and `Float`, `true` or `false` for `Boolean`, an ISO 8601 date for
    /// `Date`, an RFC 3339 instant with an offset for `DateTime`, a literal name for an enum.
    ///
    /// A text that cannot be converted is ignored, with an error. The same goes for the other
    /// `set_*` methods.
    pub fn set_text(&mut self, attr: AttrId, text: &str) {
        let attr_type = self.model().attribute(attr).attr_type();
        let value = match attr_type {
            AttrType::String => Some(Value::Str(text)),
            AttrType::Int => text.parse().ok().map(Value::Int),
            AttrType::Float => text.parse().ok().map(Value::Float),
            AttrType::Boolean => text.parse().ok().map(Value::Bool),
            AttrType::Date => text.parse::<Date>().ok().map(Value::Date),
            // `Timestamp` requires an offset (`Z` or `+01:00`). The store refuses an instant finer
            // than the microsecond rather than truncating it, which would lose information.
            AttrType::DateTime => text.parse::<Timestamp>().ok().map(Value::DateTime),
            AttrType::Enum(id) => self.model().enum_type(id).literal(text).map(Value::Enum),
        };
        match value {
            Some(value) => self.set(attr, value, || text.to_owned()),
            None => self.report_wrong_type(attr, Some(text)),
        }
    }

    /// Sets an attribute from an integer: accepted for an `Int` and for a `Float`.
    pub fn set_int(&mut self, attr: AttrId, value: i64) {
        match self.model().attribute(attr).attr_type() {
            AttrType::Int => self.set(attr, Value::Int(value), || value.to_string()),
            AttrType::Float => self.set(attr, Value::Float(value as f64), || value.to_string()),
            _ => self.report_wrong_type(attr, Some(&value.to_string())),
        }
    }

    /// Sets an attribute from a float: accepted for a `Float` only (the store checks the type).
    pub fn set_float(&mut self, attr: AttrId, value: f64) {
        self.set(attr, Value::Float(value), || value.to_string());
    }

    /// Sets an attribute from a boolean: accepted for a `Boolean` only (the store checks the
    /// type).
    pub fn set_bool(&mut self, attr: AttrId, value: bool) {
        self.set(attr, Value::Bool(value), || value.to_string());
    }

    /// A null value: the attribute stays unset.
    pub fn set_null(&mut self, attr: AttrId) {
        self.set(attr, Value::Unset, String::new);
    }

    /// Records an error at the current position: the current object, or the current reference.
    /// For an error found by the reader, such as an unknown key.
    pub fn report(&mut self, kind: LoadErrorKind) {
        if self.errors.count() {
            let error = LoadError {
                path: self.path(),
                kind,
                value: None,
            };
            self.errors.kept.push(error);
        }
    }

    /// Records an error of type for an attribute of the current object, whose name ends the
    /// path. `value` is the offending value as the file gives it, shortened to 40 characters; the
    /// reader gives `None` for a value it cannot show, such as an object.
    pub fn report_wrong_type(&mut self, attr: AttrId, value: Option<&str>) {
        if !self.errors.count() {
            return;
        }
        let attribute = self.model().attribute(attr);
        let kind = LoadErrorKind::WrongValueType {
            attribute: attribute.name().to_owned(),
            expected: self.model().type_name(attribute.attr_type()).to_owned(),
        };
        let mut path = self.path();
        path.0.push(PathSegment::Name(attribute.name().to_owned()));
        self.errors.kept.push(LoadError {
            path,
            kind,
            value: value.map(shorten),
        });
    }

    /// The filled fragment with its diagnostics, or, if there was at least one error, the
    /// errors. The fragment is then dropped.
    pub fn finish(self) -> Result<Loaded, LoadFailure> {
        if self.errors.total == 0 {
            Ok(Loaded {
                fragment: Fragment::new(self.store, self.heads, self.shape),
                diagnostics: self.diagnostics.kept,
                diagnostic_count: self.diagnostics.total,
            })
        } else {
            Err(LoadFailure {
                errors: self.errors.kept,
                error_count: self.errors.total,
                diagnostics: self.diagnostics.kept,
                diagnostic_count: self.diagnostics.total,
            })
        }
    }

    /// Sets an attribute of the current object. `shown` gives the offending value as a text; it
    /// is only called if the store refuses the value (a closure, like a lambda in C++ or Java,
    /// so that nothing is formatted when all goes well).
    fn set(&mut self, attr: AttrId, value: Value<'_>, shown: impl FnOnce() -> String) {
        let obj = match self.stack.last() {
            Some(Frame::Object { id, .. }) => *id,
            _ => panic!("no current object"),
        };
        match self.store.set(obj, attr, value) {
            Ok(()) => {}
            Err(StoreError::WrongValueType { .. }) => self.report_wrong_type(attr, Some(&shown())),
            Err(other) => self.report_store_error(other),
        }
    }

    /// Counts one more object in the current reference, or one more head, and returns the new
    /// count.
    fn count_object(&mut self) -> usize {
        match self.stack.last_mut() {
            Some(Frame::Reference { count, .. } | Frame::Heads { count }) => {
                *count += 1;
                *count
            }
            _ => panic!("no current reference"),
        }
    }

    /// The object that holds the current reference.
    fn parent_object(&self) -> ObjectId {
        match self.stack.iter().rev().nth(1) {
            Some(Frame::Object { id, .. }) => *id,
            _ => panic!("reference without an object"),
        }
    }

    /// The name of the branch, for an error on a second head: the last reference of the base
    /// path, or `/` for the root.
    fn branch_name(&self) -> String {
        let last_name = self.base.0.iter().rev().find_map(|segment| match segment {
            PathSegment::Name(name) => Some(name.clone()),
            PathSegment::Index(_) => None,
        });
        last_name.unwrap_or_else(|| "/".to_owned())
    }

    fn report_store_error(&mut self, error: StoreError) {
        let kind = match error {
            StoreError::AbstractClass { class } => LoadErrorKind::AbstractClass { class },
            StoreError::NonConformingClass { class, expected } => {
                LoadErrorKind::NonConformingClass { class, expected }
            }
            StoreError::ReferenceAlreadySet { reference } => {
                LoadErrorKind::ReferenceAlreadySet { reference }
            }
            // `set` reports a wrong type itself, with the attribute in the path; the builder
            // creates no root, always appends and deletes nothing: these cannot reach here.
            StoreError::WrongValueType { .. }
            | StoreError::RootAlreadyExists
            | StoreError::PositionOutOfRange { .. }
            | StoreError::RootNotDeletable => {
                unreachable!("builder misuse of the store: {error}")
            }
        };
        self.report(kind);
    }

    /// The path of the current position, from the root of the document: the base path, then
    /// reference names and positions in lists.
    fn path(&self) -> DataPath {
        let model = self.model();
        let mut segments = self.base.0.clone();
        for frame in &self.stack {
            match frame {
                Frame::Heads { count } if self.shape.many && *count > 0 => {
                    segments.push(PathSegment::Index(*count - 1));
                }
                Frame::Reference { reference, count } => {
                    let compiled = model.reference(*reference);
                    segments.push(PathSegment::Name(compiled.name().to_owned()));
                    if compiled.is_many() && *count > 0 {
                        segments.push(PathSegment::Index(*count - 1));
                    }
                }
                _ => {}
            }
        }
        DataPath(segments)
    }
}

/// The first [`MAX_VALUE_CHARS`] characters of a text, followed by `…` if it is longer.
/// Characters, not bytes, so that a UTF-8 character is never cut.
fn shorten(text: &str) -> String {
    let mut chars = text.chars();
    let mut short: String = chars.by_ref().take(MAX_VALUE_CHARS).collect();
    if chars.next().is_some() {
        short.push('…');
    }
    short
}

/// A load that succeeded: the fragment, and the diagnostics on the data it holds.
#[derive(Debug)]
pub struct Loaded {
    pub fragment: Fragment,
    /// The first [`MAX_REPORTED`] diagnostics, in file order.
    pub diagnostics: Vec<Diagnostic>,
    pub diagnostic_count: usize,
}

/// A load refused: its errors, and the diagnostics found as well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadFailure {
    /// The first [`MAX_REPORTED`] errors, in file order.
    pub errors: Vec<LoadError>,
    pub error_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub diagnostic_count: usize,
}

/// The form for the log: the first error and the number of the others, without any value.
impl fmt::Display for LoadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.errors.first() {
            Some(first) if self.error_count > 1 => {
                write!(f, "{first} (and {} more errors)", self.error_count - 1)
            }
            Some(first) => write!(f, "{first}"),
            None => f.write_str("load refused"),
        }
    }
}

impl std::error::Error for LoadFailure {}

/// A remark on data that is loaded all the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The path of the object.
    pub path: DataPath,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// A required attribute has no value.
    MissingRequired { attribute: String },
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DiagnosticKind::MissingRequired { attribute } => {
                write!(
                    f,
                    "{}: required attribute `{attribute}` has no value",
                    self.path
                )
            }
        }
    }
}

/// One step of a [`DataPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// The name of a reference or of an attribute.
    Name(String),
    /// A position in a list.
    Index(usize),
}

/// The path of a datum in the model, from the root object: reference names and positions in
/// lists. Displayed as `/stations/0/equipements/1/mesures/2/valeur`; the root is `/`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataPath(pub Vec<PathSegment>);

impl fmt::Display for DataPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("/");
        }
        for segment in &self.0 {
            match segment {
                PathSegment::Name(name) => write!(f, "/{name}")?,
                PathSegment::Index(index) => write!(f, "/{index}")?,
            }
        }
        Ok(())
    }
}

/// An error that refuses a load.
///
/// Two forms: `Display` for the log, which never shows a text taken from the data file (user
/// data must not reach the log): neither the offending value, nor an unknown key or class name;
/// and [`user_message`](LoadError::user_message) for the user, which shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: DataPath,
    pub kind: LoadErrorKind,
    /// The offending value of an error of type, shortened to 40 characters followed by `…`.
    pub value: Option<String>,
}

impl LoadError {
    /// The message for the user, with what the file gives: the offending value, the unknown key
    /// or the unknown class name, shortened to 40 characters.
    pub fn user_message(&self) -> String {
        let found = match (&self.kind, &self.value) {
            (_, Some(value)) => value.clone(),
            (LoadErrorKind::UnknownKey { key: text }, None)
            | (LoadErrorKind::UnknownClass { name: text }, None) => shorten(text),
            _ => return self.to_string(),
        };
        format!("{self}, found `{found}`")
    }
}

/// What went wrong while loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadErrorKind {
    /// A key is neither `_type_` nor the name of an attribute or a reference of the class.
    UnknownKey { key: String },
    /// A value cannot be converted to the type of its attribute.
    WrongValueType { attribute: String, expected: String },
    /// `_type_` names no class of the model.
    UnknownClass { name: String },
    /// The class of an object is abstract.
    AbstractClass { class: String },
    /// The class of an object is neither the declared type nor one of its subclasses.
    NonConformingClass { class: String, expected: String },
    /// A single reference holds more than one object.
    ReferenceAlreadySet { reference: String },
    /// The file is not well-formed, or has an unexpected structure.
    Syntax { line: usize, column: usize },
    /// The file holds a value of another kind than expected: an object where an array was
    /// expected, for example. `expected` describes what was expected ("an array of objects").
    WrongStructure { expected: String },
    /// The file could not be read.
    Io { message: String },
}

/// The form for the log, without the offending value.
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.path)?;
        match &self.kind {
            // A key and a class name come from the file: they may be data (a dictionary used
            // by mistake), so only the message for the user shows them.
            LoadErrorKind::UnknownKey { .. } => f.write_str("unknown key"),
            LoadErrorKind::WrongValueType {
                attribute,
                expected,
            } => {
                write!(
                    f,
                    "attribute `{attribute}` expects a value of type `{expected}`"
                )
            }
            LoadErrorKind::UnknownClass { .. } => f.write_str("unknown class name in `_type_`"),
            LoadErrorKind::AbstractClass { class } => write!(f, "class `{class}` is abstract"),
            LoadErrorKind::NonConformingClass { class, expected } => {
                write!(f, "class `{class}` is not a `{expected}`")
            }
            LoadErrorKind::ReferenceAlreadySet { reference } => {
                write!(
                    f,
                    "single reference `{reference}` holds more than one object"
                )
            }
            LoadErrorKind::WrongStructure { expected } => write!(f, "expected {expected}"),
            LoadErrorKind::Syntax { line, column } => {
                write!(f, "invalid JSON at line {line}, column {column}")
            }
            LoadErrorKind::Io { message } => write!(f, "read error: {message}"),
        }
    }
}

impl std::error::Error for LoadError {}
