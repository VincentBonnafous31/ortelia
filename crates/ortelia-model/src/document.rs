//! The document: the objects of an application, and the only way to modify them.
//!
//! A [`Document`] owns the [`Store`] of the application. Everyone may read it ([`Document::store`]);
//! only a transaction ([`Document::edit`]) modifies it, and records what it changed. The document
//! numbers its transactions: its *revision* goes up by one with each transaction that changed
//! something, and its *journal* keeps them, oldest first.
//!
//! The presentation does not subscribe to changes: it keeps the last revision it saw, compares it
//! with [`Document::revision`] (an integer comparison, at each frame), and reads
//! [`Document::changes_since`] when it moved. No observer is called during a modification, so
//! nothing tries to read the store while it is borrowed for writing.
//!
//! A source feeds a branch by a [`Fragment`] read apart, then grafted with [`Edit::replace`]: the
//! fragment's store is absorbed, not copied, and only the objects of the replaced branch die.

use std::fmt;
use std::sync::Arc;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::fragment::{Fragment, Shape};
use crate::metamodel::{AttrId, ClassId, Model, RefId};
use crate::store::{ObjectId, Position, Store, StoreError};
use crate::value::Value;

/// The owned version of a [`Value`]: a text is a `String` rather than a `&str` borrowed from the
/// store, so that the journal outlives the following modifications of the store.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedValue {
    Unset,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Enum(u32),
    Date(Date),
    DateTime(Timestamp),
}

impl OwnedValue {
    /// The value borrowed from this one, to compare it with a value read from a store.
    pub fn as_value(&self) -> Value<'_> {
        match self {
            OwnedValue::Unset => Value::Unset,
            OwnedValue::Int(v) => Value::Int(*v),
            OwnedValue::Float(v) => Value::Float(*v),
            OwnedValue::Bool(v) => Value::Bool(*v),
            OwnedValue::Str(text) => Value::Str(text),
            OwnedValue::Enum(v) => Value::Enum(*v),
            OwnedValue::Date(date) => Value::Date(*date),
            OwnedValue::DateTime(instant) => Value::DateTime(*instant),
        }
    }
}

impl From<Value<'_>> for OwnedValue {
    fn from(value: Value<'_>) -> OwnedValue {
        match value {
            Value::Unset => OwnedValue::Unset,
            Value::Int(v) => OwnedValue::Int(v),
            Value::Float(v) => OwnedValue::Float(v),
            Value::Bool(v) => OwnedValue::Bool(v),
            Value::Str(text) => OwnedValue::Str(text.to_owned()),
            Value::Enum(v) => OwnedValue::Enum(v),
            Value::Date(date) => OwnedValue::Date(date),
            Value::DateTime(instant) => OwnedValue::DateTime(instant),
        }
    }
}

/// A branch of the tree of objects: the root itself, or a containment reference of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Root,
    Reference { parent: ObjectId, reference: RefId },
}

/// One change of a transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// An attribute changed value. `old` or `new` is [`OwnedValue::Unset`] for a value absent
    /// before or after.
    Set {
        obj: ObjectId,
        attr: AttrId,
        old: OwnedValue,
        new: OwnedValue,
    },
    /// An object was created, empty, at position `index` of the reference (0 in a single
    /// reference). Its values follow as [`Change::Set`]s.
    Created {
        obj: ObjectId,
        class: ClassId,
        parent: ObjectId,
        reference: RefId,
        index: usize,
    },
    /// An object was deleted with its descendants; `index` is its position before the
    /// deletion. In a batch deletion, every index is the position before the batch: to replay
    /// them, take them from the largest position to the smallest.
    Deleted {
        obj: ObjectId,
        parent: ObjectId,
        reference: RefId,
        index: usize,
    },
    /// The content of a branch was replaced. `objects` are its new objects, in order; the old
    /// ones are dead ([`Store::is_alive`]).
    Replaced {
        target: Target,
        objects: Vec<ObjectId>,
    },
}

/// The changes of a transaction, in the order of its operations.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// The revision of the document after this transaction.
    pub revision: u64,
    pub changes: Vec<Change>,
}

/// The objects of an application, under one root, with their revision and their journal.
#[derive(Debug)]
pub struct Document {
    // The root is the store's: `Store::root`.
    store: Store,
    /// The class given at creation: a fragment that replaces the root has its shape.
    root_class: ClassId,
    revision: u64,
    /// Oldest first; one transaction per revision since 0.
    journal: Vec<Transaction>,
}

impl Document {
    /// A document whose root is a new, empty object of class `root`. Its revision is 0 and its
    /// journal is empty.
    pub fn new(model: Arc<Model>, root: ClassId) -> Result<Document, StoreError> {
        let mut store = Store::new(model);
        store.create_root(root)?;
        Ok(Document {
            store,
            root_class: root,
            revision: 0,
            journal: Vec::new(),
        })
    }

    /// The shape of a branch of this document: what a fragment must have to replace its
    /// content.
    pub fn shape_of(&self, target: Target) -> Shape {
        match target {
            Target::Root => Shape::root(self.root_class),
            Target::Reference { reference, .. } => Shape::of(self.model(), reference),
        }
    }

    /// The objects, to read them. A modification goes through a transaction
    /// ([`Document::edit`]).
    ///
    /// Outside this crate, the methods that modify a [`Store`] are not visible: even with a
    /// `&mut Store`, the code below does not compile (error E0624, private method), so that no
    /// modification escapes the journal.
    ///
    /// ```compile_fail,E0624
    /// use ortelia_model::metamodel::AttrId;
    /// use ortelia_model::store::{ObjectId, Store};
    /// use ortelia_model::value::Value;
    ///
    /// fn modify(store: &mut Store, obj: ObjectId, attr: AttrId) {
    ///     let _ = store.set(obj, attr, Value::Int(1));
    /// }
    /// ```
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn model(&self) -> &Model {
        self.store.model()
    }

    /// The root object. It changes only when a fragment replaces it ([`Edit::replace`]).
    pub fn root(&self) -> ObjectId {
        self.store.root().expect("a document always has a root")
    }

    /// The current revision: 0 at creation, then one more for each transaction that changed
    /// something.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Every transaction since the creation of the document, oldest first.
    pub fn journal(&self) -> &[Transaction] {
        &self.journal
    }

    /// The transactions after revision `revision`, oldest first. Empty when `revision` is the
    /// current revision.
    pub fn changes_since(&self, revision: u64) -> &[Transaction] {
        // The journal is sorted by revision: a binary search finds the first one after it.
        let start = self.journal.partition_point(|t| t.revision <= revision);
        &self.journal[start..]
    }

    /// Begins a transaction. Its operations apply at once; the transaction is recorded when the
    /// returned [`Edit`] is dropped, at the end of its scope or by `drop(edit)`.
    ///
    /// The `Edit` borrows the document mutably, like a `std::lock_guard` holds its mutex: while
    /// it lives, the document cannot be read (the compiler refuses `document.store()`: *cannot
    /// borrow `document` as immutable because it is also borrowed as mutable*). Read through
    /// [`Edit::store`] during the transaction, or drop the `Edit` first.
    pub fn edit(&mut self) -> Edit<'_> {
        Edit {
            document: self,
            changes: Vec::new(),
        }
    }
}

/// `true` if two values are the same. Floats are compared by their bits rather than by `==`, which
/// says `0.0 == -0.0` (a change would be lost) and `NaN != NaN` (a set without effect would be
/// recorded).
fn same_value(a: &Value<'_>, b: &Value<'_>) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        _ => a == b,
    }
}

/// A transaction in progress: a guard that modifies the document and records its changes.
///
/// An operation refused with a [`StoreError`] changes nothing and records nothing; the operations
/// before it in the same transaction stay applied (there is no rollback).
#[derive(Debug)]
pub struct Edit<'a> {
    document: &'a mut Document,
    changes: Vec<Change>,
}

impl Edit<'_> {
    /// The objects, as modified so far by this transaction.
    pub fn store(&self) -> &Store {
        &self.document.store
    }

    /// The root object.
    pub fn root(&self) -> ObjectId {
        self.document.root()
    }

    /// Creates an empty object of class `class` in the containment reference `reference` of
    /// `parent`, as [`Store`] does, and records [`Change::Created`].
    pub fn create_child(
        &mut self,
        parent: ObjectId,
        reference: RefId,
        position: Position,
        class: ClassId,
    ) -> Result<ObjectId, StoreError> {
        let store = &mut self.document.store;
        let index = match position {
            Position::At(index) => index,
            Position::End => store.children(parent, reference).len(),
        };
        let obj = store.create_child(parent, reference, position, class)?;
        self.changes.push(Change::Created {
            obj,
            class,
            parent,
            reference,
            index,
        });
        Ok(obj)
    }

    /// Gives a value to an attribute, or removes it with [`Value::Unset`], and records
    /// [`Change::Set`] with the old and the new value. Giving the value the attribute already
    /// has records nothing.
    pub fn set(&mut self, obj: ObjectId, attr: AttrId, value: Value<'_>) -> Result<(), StoreError> {
        let store = &mut self.document.store;
        let current = store.get(obj, attr);
        if same_value(&current, &value) {
            return Ok(());
        }
        // Copied before the store changes: `current` borrows it.
        let old = OwnedValue::from(current);
        store.set(obj, attr, value)?;
        self.changes.push(Change::Set {
            obj,
            attr,
            old,
            new: OwnedValue::from(value),
        });
        Ok(())
    }

    /// Deletes an object and its descendants, and records [`Change::Deleted`]. The root cannot
    /// be deleted ([`StoreError::RootNotDeletable`]).
    pub fn delete(&mut self, obj: ObjectId) -> Result<(), StoreError> {
        if obj == self.document.root() {
            return Err(StoreError::RootNotDeletable);
        }
        // The path of a single object: no set to build, unlike `delete_many`.
        let (parent, reference, index) = self.document.store.delete(obj);
        self.changes.push(Change::Deleted {
            obj,
            parent,
            reference,
            index,
        });
        Ok(())
    }

    /// Deletes several objects and their descendants, filtering each list once, and records one
    /// [`Change::Deleted`] per object removed from its container (see [`Store::delete_many`]).
    /// Nothing is deleted if one of them is the root.
    pub fn delete_many(&mut self, objs: &[ObjectId]) -> Result<(), StoreError> {
        if objs.contains(&self.document.root()) {
            return Err(StoreError::RootNotDeletable);
        }
        let deleted = self.document.store.delete_many(objs);
        self.changes
            .extend(
                deleted
                    .into_iter()
                    .map(|(obj, parent, reference, index)| Change::Deleted {
                        obj,
                        parent,
                        reference,
                        index,
                    }),
            );
        Ok(())
    }

    /// Replaces the content of a branch with the objects of a fragment, in their order, and
    /// records a single [`Change::Replaced`].
    ///
    /// The fragment must have the shape of the branch ([`Document::shape_of`]), hold an object
    /// for the root, and have been built with the model of the document; otherwise nothing
    /// changes. The old objects of the branch and their descendants are deleted. The fragment's
    /// objects are moved into the document's store, not copied, and get new identifiers, never
    /// given before; the identifiers of the other branches stay valid.
    pub fn replace(&mut self, target: Target, fragment: Fragment) -> Result<(), IntegrateError> {
        let document = &mut *self.document;
        if !Arc::ptr_eq(fragment.store().model_arc(), document.store.model_arc()) {
            return Err(IntegrateError::OtherModel);
        }
        let expected = document.shape_of(target);
        if fragment.shape() != expected {
            let model = document.store.model();
            return Err(IntegrateError::WrongShape {
                expected: expected.describe(model),
                found: fragment.shape().describe(model),
            });
        }
        if target == Target::Root && fragment.heads().is_empty() {
            return Err(IntegrateError::EmptyRoot);
        }

        let (store, heads) = fragment.into_parts();
        if let Target::Reference { parent, reference } = target {
            document.store.clear_reference(parent, reference);
        }
        let heads = document.store.absorb(store, heads);
        match target {
            // A single shape: one head, checked above.
            Target::Root => document.store.replace_root(heads[0]),
            Target::Reference { parent, reference } => {
                document.store.attach(parent, reference, heads.clone());
            }
        }
        self.changes.push(Change::Replaced {
            target,
            objects: heads,
        });
        Ok(())
    }
}

/// Why a fragment cannot replace the content of a branch. The document is left unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrateError {
    /// The fragment was built for another type or multiplicity than those of the branch.
    WrongShape { expected: String, found: String },
    /// A fragment for the root holds no object: a document always has a root.
    EmptyRoot,
    /// The fragment was built with another model than the document's.
    OtherModel,
}

impl fmt::Display for IntegrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrateError::WrongShape { expected, found } => write!(
                f,
                "the branch holds {expected}, but the fragment was read for {found}"
            ),
            IntegrateError::EmptyRoot => write!(f, "the fragment for the root holds no object"),
            IntegrateError::OtherModel => {
                write!(f, "the fragment was built with another model")
            }
        }
    }
}

impl std::error::Error for IntegrateError {}

/// Records the transaction, if it changed something.
impl Drop for Edit<'_> {
    fn drop(&mut self) {
        if self.changes.is_empty() {
            return;
        }
        let document = &mut *self.document;
        document.revision += 1;
        document.journal.push(Transaction {
            revision: document.revision,
            changes: std::mem::take(&mut self.changes),
        });
    }
}

#[cfg(test)]
mod tests;
