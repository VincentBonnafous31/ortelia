//! The builder: the only API through which a reader creates data, for every reader, present and
//! future (JSON, XML, reading plugins that know the model).
//!
//! The reader calls it in file order, like a SAX `ContentHandler`: beginning of an object,
//! values of its attributes, end of the object. A stack of frames gives the context, and the
//! path of an error, which is only computed when there is an error.
//!
//! A value given as text is converted according to the type of its attribute ([`set_text`]):
//! it is the only text conversion of the system.
//!
//! [`set_text`]: Builder::set_text

use std::fmt;
use std::sync::Arc;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::metamodel::{AttrId, AttrType, ClassId, Model, RefId};
use crate::store::{ObjectId, Position, Store, StoreError};
use crate::value::Value;

/// Where the builder is in the tree being built.
#[derive(Debug)]
enum Frame {
    Object {
        id: ObjectId,
        class: ClassId,
    },
    /// `count`: objects begun so far in this reference.
    Reference {
        reference: RefId,
        count: usize,
    },
}

/// Fills a new store, in file order.
#[derive(Debug)]
pub struct Builder {
    store: Store,
    stack: Vec<Frame>,
}

impl Builder {
    /// A builder that fills a new, empty store.
    pub fn new(model: Arc<Model>) -> Builder {
        Builder {
            store: Store::new(model),
            stack: Vec::new(),
        }
    }

    pub fn model(&self) -> &Model {
        self.store.model()
    }

    /// Begins the root object.
    pub fn begin_root(&mut self, class: ClassId) -> Result<(), LoadError> {
        let id = self
            .store
            .create_root(class)
            .map_err(|e| self.store_error(e))?;
        self.stack.push(Frame::Object { id, class });
        Ok(())
    }

    /// Begins the children of a containment reference of the current object.
    ///
    /// # Panics
    /// If the current frame is not an object.
    pub fn begin_reference(&mut self, reference: RefId) {
        assert!(
            matches!(self.stack.last(), Some(Frame::Object { .. })),
            "no current object"
        );
        self.stack.push(Frame::Reference {
            reference,
            count: 0,
        });
    }

    /// Begins an object of class `class`, at the end of the current reference.
    ///
    /// # Panics
    /// If the current frame is not a reference.
    pub fn begin_object(&mut self, class: ClassId) -> Result<(), LoadError> {
        let reference = self.next_object();
        let parent = self.parent_object();
        let id = self
            .store
            .create_child(parent, reference, Position::End, class)
            .map_err(|e| self.store_error(e))?;
        self.stack.push(Frame::Object { id, class });
        Ok(())
    }

    /// Records that the next object of the current reference cannot be created, and returns the
    /// error with its path. For an error found by the reader, such as an unknown class name.
    pub fn fail_object(&mut self, kind: LoadErrorKind) -> LoadError {
        self.next_object();
        self.error(kind)
    }

    /// Ends the current object.
    pub fn end_object(&mut self) {
        assert!(
            matches!(self.stack.pop(), Some(Frame::Object { .. })),
            "no current object"
        );
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
    pub fn set_text(&mut self, attr: AttrId, text: &str) -> Result<(), LoadError> {
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
            Some(value) => self.set(attr, value),
            None => Err(self.wrong_type(attr)),
        }
    }

    /// Sets an attribute from an integer: accepted for an `Int` and for a `Float`.
    pub fn set_int(&mut self, attr: AttrId, value: i64) -> Result<(), LoadError> {
        match self.model().attribute(attr).attr_type() {
            AttrType::Int => self.set(attr, Value::Int(value)),
            AttrType::Float => self.set(attr, Value::Float(value as f64)),
            _ => Err(self.wrong_type(attr)),
        }
    }

    /// Sets an attribute from a float: accepted for a `Float` only (the store checks the type).
    pub fn set_float(&mut self, attr: AttrId, value: f64) -> Result<(), LoadError> {
        self.set(attr, Value::Float(value))
    }

    /// Sets an attribute from a boolean: accepted for a `Boolean` only (the store checks the
    /// type).
    pub fn set_bool(&mut self, attr: AttrId, value: bool) -> Result<(), LoadError> {
        self.set(attr, Value::Bool(value))
    }

    /// A null value: the attribute stays unset.
    pub fn set_null(&mut self, attr: AttrId) -> Result<(), LoadError> {
        self.set(attr, Value::Unset)
    }

    /// An error at the current position (the current object, or the current reference).
    pub fn error(&self, kind: LoadErrorKind) -> LoadError {
        LoadError {
            path: self.path(),
            kind,
        }
    }

    /// An error on an attribute of the current object: its name ends the path.
    pub fn error_at_attribute(&self, attr: AttrId, kind: LoadErrorKind) -> LoadError {
        let mut path = self.path();
        path.0.push(PathSegment::Name(
            self.model().attribute(attr).name().to_owned(),
        ));
        LoadError { path, kind }
    }

    /// An error of type for an attribute of the current object.
    pub fn wrong_type(&self, attr: AttrId) -> LoadError {
        let attribute = self.model().attribute(attr);
        let kind = LoadErrorKind::WrongValueType {
            attribute: attribute.name().to_owned(),
            expected: self.model().type_name(attribute.attr_type()).to_owned(),
        };
        self.error_at_attribute(attr, kind)
    }

    /// The filled store.
    pub fn finish(self) -> Store {
        self.store
    }

    fn set(&mut self, attr: AttrId, value: Value<'_>) -> Result<(), LoadError> {
        let obj = match self.stack.last() {
            Some(Frame::Object { id, .. }) => *id,
            _ => panic!("no current object"),
        };
        match self.store.set(obj, attr, value) {
            Ok(()) => Ok(()),
            Err(StoreError::WrongValueType { .. }) => Err(self.wrong_type(attr)),
            Err(other) => Err(self.store_error(other)),
        }
    }

    /// Counts one more object in the current reference, and returns the reference.
    fn next_object(&mut self) -> RefId {
        match self.stack.last_mut() {
            Some(Frame::Reference { reference, count }) => {
                *count += 1;
                *reference
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

    fn store_error(&self, error: StoreError) -> LoadError {
        let kind = match error {
            StoreError::AbstractClass { class } => LoadErrorKind::AbstractClass { class },
            StoreError::NonConformingClass { class, expected } => {
                LoadErrorKind::NonConformingClass { class, expected }
            }
            StoreError::ReferenceAlreadySet { reference } => {
                LoadErrorKind::ReferenceAlreadySet { reference }
            }
            // `set` reports a wrong type itself, with the attribute in the path; the builder
            // creates one root, and always appends: these cannot reach here.
            StoreError::WrongValueType { .. }
            | StoreError::RootAlreadyExists
            | StoreError::PositionOutOfRange { .. } => {
                unreachable!("builder misuse of the store: {error}")
            }
        };
        self.error(kind)
    }

    /// The path of the current position: reference names, and positions in lists.
    fn path(&self) -> DataPath {
        let model = self.model();
        let mut segments = Vec::new();
        for frame in &self.stack {
            if let Frame::Reference { reference, count } = frame {
                let compiled = model.reference(*reference);
                segments.push(PathSegment::Name(compiled.name().to_owned()));
                if compiled.is_many() && *count > 0 {
                    segments.push(PathSegment::Index(*count - 1));
                }
            }
        }
        DataPath(segments)
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

/// An error that refuses a load. It never holds the offending value, which must not reach the
/// log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: DataPath,
    pub kind: LoadErrorKind,
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
    /// The file could not be read.
    Io { message: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.path)?;
        match &self.kind {
            LoadErrorKind::UnknownKey { key } => write!(f, "unknown key `{key}`"),
            LoadErrorKind::WrongValueType {
                attribute,
                expected,
            } => {
                write!(
                    f,
                    "attribute `{attribute}` expects a value of type `{expected}`"
                )
            }
            LoadErrorKind::UnknownClass { name } => write!(f, "unknown class `{name}`"),
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
            LoadErrorKind::Syntax { line, column } => {
                write!(f, "invalid JSON at line {line}, column {column}")
            }
            LoadErrorKind::Io { message } => write!(f, "read error: {message}"),
        }
    }
}

impl std::error::Error for LoadError {}
