//! The metamodel: what the data of an application look like, known only at run time.
//!
//! A model is read from one or more model files ([`file`]), then compiled ([`Model::compile`])
//! into a form made for fast access:
//! - every class, attribute and reference gets a small identifier, resolved once from its name;
//! - every attribute and reference gets a fixed position in its class, the same in all
//!   subclasses (inherited features come first), so a store can find a value without looking
//!   anything up by name;
//! - a subclass test takes constant time (single inheritance, see [`Model::is_subclass_of`]).

mod compile;
pub mod file;

use std::collections::HashMap;
use std::fmt;

pub use file::ModelFile;

/// Identifier of a class in a compiled [`Model`].
///
/// A *newtype*: a `u16` wrapped in its own type, so that a class identifier cannot be mixed up
/// with another small integer, as two C `typedef`s could be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassId(u16);

impl ClassId {
    /// Position of the class in [`Model::classes`].
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// Identifier of an enum in a compiled [`Model`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumId(u16);

/// Identifier of an attribute: the class that declares it, and its position in the layout of
/// that class and of all its subclasses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttrId {
    owner: ClassId,
    index: u16,
}

impl AttrId {
    /// The class that declares the attribute.
    pub fn owner(self) -> ClassId {
        self.owner
    }

    /// Position of the attribute among the attributes of any class that has it.
    pub fn index(self) -> usize {
        usize::from(self.index)
    }
}

/// Identifier of a containment reference, like [`AttrId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefId {
    owner: ClassId,
    index: u16,
}

impl RefId {
    /// The class that declares the reference.
    pub fn owner(self) -> ClassId {
        self.owner
    }

    /// Position of the reference among the references of any class that has it.
    pub fn index(self) -> usize {
        usize::from(self.index)
    }
}

/// How the value of an attribute is stored. A data type is reduced to its base type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrType {
    String,
    /// Signed 64-bit integer.
    Int,
    /// 64-bit float.
    Float,
    Boolean,
    /// Civil date, without time or time zone.
    Date,
    /// UTC instant, to the microsecond.
    DateTime,
    Enum(EnumId),
}

/// The names of the base types, available in every model without declaration.
pub const BASE_TYPES: [(&str, AttrType); 6] = [
    ("String", AttrType::String),
    ("Int", AttrType::Int),
    ("Float", AttrType::Float),
    ("Boolean", AttrType::Boolean),
    ("Date", AttrType::Date),
    ("DateTime", AttrType::DateTime),
];

/// An attribute of a compiled class.
#[derive(Debug, Clone)]
pub struct Attribute {
    name: String,
    id: AttrId,
    attr_type: AttrType,
}

impl Attribute {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> AttrId {
        self.id
    }

    pub fn attr_type(&self) -> AttrType {
        self.attr_type
    }
}

/// A containment reference of a compiled class.
#[derive(Debug, Clone)]
pub struct Reference {
    name: String,
    id: RefId,
    target: ClassId,
    many: bool,
}

impl Reference {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> RefId {
        self.id
    }

    /// The declared type of the children.
    pub fn target(&self) -> ClassId {
        self.target
    }

    /// `true` for a list, `false` for a single reference (zero or one child).
    pub fn is_many(&self) -> bool {
        self.many
    }
}

/// A compiled class, with its full layout: inherited attributes and references first.
#[derive(Debug, Clone)]
pub struct Class {
    name: String,
    id: ClassId,
    is_abstract: bool,
    /// The chain of ancestors, from the root class down to this class (included). The JVMs call
    /// this the *display* of a class: `display[depth(A)] == A` tells whether this class is an A.
    display: Vec<ClassId>,
    has_subclasses: bool,
    attributes: Vec<Attribute>,
    references: Vec<Reference>,
}

impl Class {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> ClassId {
        self.id
    }

    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// The super class, if any.
    pub fn super_class(&self) -> Option<ClassId> {
        self.ancestors().next()
    }

    /// The ancestors, nearest first (the super class, then its super class...).
    pub fn ancestors(&self) -> impl Iterator<Item = ClassId> + '_ {
        self.display[..self.depth()].iter().rev().copied()
    }

    /// Number of ancestors: 0 for a class without super class.
    pub fn depth(&self) -> usize {
        self.display.len() - 1
    }

    /// `true` if at least one class inherits from this one.
    pub fn has_subclasses(&self) -> bool {
        self.has_subclasses
    }

    /// All the attributes, inherited ones first, in the order of their positions.
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    /// All the containment references, inherited ones first, in the order of their positions.
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    /// Finds an attribute, declared or inherited, by its name. Call it once, not on every access.
    pub fn attribute(&self, name: &str) -> Option<AttrId> {
        // A linear search: classes have few attributes, and comparing a few short strings is
        // as fast as hashing one.
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id)
    }

    /// Finds a containment reference, declared or inherited, by its name.
    pub fn reference(&self, name: &str) -> Option<RefId> {
        self.references
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.id)
    }
}

/// A compiled enum.
#[derive(Debug, Clone)]
pub struct EnumType {
    name: String,
    literals: Vec<String>,
}

impl EnumType {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The literals, in their declaration order.
    pub fn literals(&self) -> &[String] {
        &self.literals
    }

    /// Finds the index of a literal by its name.
    pub fn literal(&self, name: &str) -> Option<u32> {
        // Enums are short: a linear search is enough.
        let index = self.literals.iter().position(|l| l == name)?;
        u32::try_from(index).ok()
    }
}

/// A compiled model. Immutable once compiled: it is shared with `Arc<Model>`.
#[derive(Debug, Clone)]
pub struct Model {
    classes: Vec<Class>,
    enums: Vec<EnumType>,
    class_by_name: HashMap<String, ClassId>,
}

impl Model {
    /// Compiles the model described by one or more model files.
    pub fn compile(files: &[ModelFile]) -> Result<Model, ModelError> {
        compile::compile(files)
    }

    /// Finds a class by its name.
    pub fn class_id(&self, name: &str) -> Option<ClassId> {
        self.class_by_name.get(name).copied()
    }

    pub fn class(&self, id: ClassId) -> &Class {
        &self.classes[id.index()]
    }

    pub fn classes(&self) -> &[Class] {
        &self.classes
    }

    pub fn enum_type(&self, id: EnumId) -> &EnumType {
        &self.enums[usize::from(id.0)]
    }

    pub fn attribute(&self, id: AttrId) -> &Attribute {
        // The declaring class has the attribute at its position, like all its subclasses.
        &self.class(id.owner).attributes[id.index()]
    }

    pub fn reference(&self, id: RefId) -> &Reference {
        &self.class(id.owner).references[id.index()]
    }

    /// `true` if `class` is `ancestor` or one of its subclasses, in constant time.
    ///
    /// In single inheritance, the ancestor of `class` at the depth of `ancestor` is unique: it is
    /// the JVM subtype test for primary super classes.
    pub fn is_subclass_of(&self, class: ClassId, ancestor: ClassId) -> bool {
        let depth = self.class(ancestor).depth();
        self.class(class).display.get(depth) == Some(&ancestor)
    }

    /// The name of a type, for messages: a base type name or the name of the enum.
    pub fn type_name(&self, attr_type: AttrType) -> &str {
        match attr_type {
            AttrType::Enum(id) => self.enum_type(id).name(),
            base => BASE_TYPES
                .iter()
                .find(|(_, t)| *t == base)
                .map(|(name, _)| *name)
                .unwrap_or("?"),
        }
    }
}

/// An error that prevents a model from compiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Two classes have the same name.
    DuplicateClass { name: String },
    /// Two enums or data types have the same name, or one has the name of a base type.
    DuplicateType { name: String },
    /// A super class is not in the model.
    UnknownSuperClass { class: String, super_class: String },
    /// A class inherits from itself, directly or not. `classes` lists the cycle.
    InheritanceCycle { classes: Vec<String> },
    /// The type of an attribute, or of a reference, is not in the model.
    UnknownType {
        type_name: String,
        class: String,
        feature: String,
    },
    /// The base types of a data type form a cycle.
    DataTypeCycle { name: String },
    /// A class declares an attribute or a reference whose name it already has, declared in
    /// `declared_in` (the class itself or one of its ancestors).
    DuplicateFeature {
        name: String,
        class: String,
        declared_in: String,
    },
    /// An attribute has an upper bound other than 1.
    MultiValuedAttribute { class: String, attribute: String },
    /// More classes, enums, attributes or references than the identifiers can number.
    TooLarge { what: &'static str },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::DuplicateClass { name } => write!(f, "class `{name}` is declared twice"),
            ModelError::DuplicateType { name } => write!(f, "type `{name}` is declared twice"),
            ModelError::UnknownSuperClass { class, super_class } => {
                write!(f, "class `{class}`: unknown super class `{super_class}`")
            }
            ModelError::InheritanceCycle { classes } => {
                write!(f, "inheritance cycle: {}", classes.join(" -> "))
            }
            ModelError::UnknownType {
                type_name,
                class,
                feature,
            } => {
                write!(f, "`{class}.{feature}`: unknown type `{type_name}`")
            }
            ModelError::DataTypeCycle { name } => {
                write!(f, "data type `{name}`: its base types form a cycle")
            }
            ModelError::DuplicateFeature {
                name,
                class,
                declared_in,
            } => write!(
                f,
                "class `{class}`: `{name}` is already declared in class `{declared_in}`"
            ),
            ModelError::MultiValuedAttribute { class, attribute } => write!(
                f,
                "`{class}.{attribute}`: multi-valued attributes are not supported yet"
            ),
            ModelError::TooLarge { what } => write!(f, "too many {what} in the model"),
        }
    }
}

impl std::error::Error for ModelError {}
