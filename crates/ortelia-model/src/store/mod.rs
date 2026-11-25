//! The object store: the objects of an application, whose model is only known at run time.
//!
//! One block of typed values per object, the same principle as EMF's `DynamicEObjectImpl`,
//! which keeps its values in an array of its own:
//!
//! ```text
//!  objects: Vec<Option<Entry>>       Entry { class, container, slots: Box<[Slot]> }
//!                                                                 |
//!                                    +-----------+-----------+----v------+-----------+
//!                                    | Str  16 B | Date 16 B | Float 16B | Many 16 B |
//!                                    +-----------+-----------+-----------+-----------+
//! ```
//!
//! A slot is an enum of 16 bytes (8 of value, plus the tag and the alignment): attributes first,
//! then references. One allocation per object; deleting an object frees its block. Texts live
//! in the string arena ([`StringArena`]), dates are encoded by [`value`](crate::value).
//!
//! This layout was chosen by the S1 trial (`docs/essais/s1-stockage.md`), against a heap of
//! contiguous records: it is faster to navigate, at the cost of more memory.

mod absorb;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::metamodel::{AttrId, AttrType, ClassId, Model, RefId};
use crate::strings::{StrRef, StringArena};
use crate::value::{
    Value, decode_date, decode_datetime, encode_date, encode_datetime, is_whole_microsecond,
};

/// Identifier of an object: its index in the object table. Stable during the session, and never
/// reused, even after the object is deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Position in the object table, in creation order.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Identifier of the object table entry `index`.
    ///
    /// # Panics
    /// If `index` does not fit in 32 bits.
    fn from_index(index: usize) -> ObjectId {
        ObjectId(u32::try_from(index).expect("too many objects"))
    }
}

/// Where a new child goes in a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// After the last child.
    End,
    /// At this index: the children from this index on move one place down.
    At(usize),
}

/// An error that the caller can handle: the store is left unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// An abstract class cannot be instantiated.
    AbstractClass { class: String },
    /// The class is neither the type of the reference nor one of its subclasses.
    NonConformingClass { class: String, expected: String },
    /// A single reference already holds a child.
    ReferenceAlreadySet { reference: String },
    /// The value does not have the type of the attribute, or is not representable (an enum
    /// literal out of range, an instant finer than the microsecond).
    WrongValueType { attribute: String, expected: String },
    /// The store already has a root object.
    RootAlreadyExists,
    /// The position is past the end of the list.
    PositionOutOfRange { position: usize, len: usize },
    /// The root of a document is replaced, never deleted.
    RootNotDeletable,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::AbstractClass { class } => {
                write!(f, "class `{class}` is abstract")
            }
            StoreError::NonConformingClass { class, expected } => {
                write!(f, "class `{class}` is not a `{expected}`")
            }
            StoreError::ReferenceAlreadySet { reference } => {
                write!(f, "single reference `{reference}` already holds an object")
            }
            StoreError::WrongValueType {
                attribute,
                expected,
            } => {
                write!(
                    f,
                    "attribute `{attribute}` expects a value of type `{expected}`"
                )
            }
            StoreError::RootAlreadyExists => write!(f, "the store already has a root object"),
            StoreError::PositionOutOfRange { position, len } => {
                write!(f, "position {position} is past the end of a list of {len}")
            }
            StoreError::RootNotDeletable => {
                write!(f, "the root cannot be deleted, only replaced")
            }
        }
    }
}

impl std::error::Error for StoreError {}

/// The value of an attribute, or the content of a reference, in an object's block.
#[derive(Debug, Clone, Copy)]
enum Slot {
    Unset,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(StrRef),
    Enum(u32),
    /// Encoded by [`encode_date`].
    Date(i64),
    /// Microseconds, encoded by [`encode_datetime`].
    DateTime(i64),
    /// A single reference: its child, if any.
    One(Option<ObjectId>),
    /// A list: its number in the list table, created with the first child.
    Many(Option<u32>),
}

/// An object.
#[derive(Debug)]
struct Entry {
    class: ClassId,
    container: Option<(ObjectId, RefId)>,
    slots: Box<[Slot]>,
}

/// The objects of an application.
///
/// Programming errors panic, like a `ClassCastException` in Java: using a deleted object (see
/// [`is_alive`](Store::is_alive)), or an attribute or reference of another class. Errors that
/// the caller can handle are [`StoreError`]s, and leave the store unchanged.
#[derive(Debug)]
pub struct Store {
    model: Arc<Model>,
    /// The initial block of each class (attributes unset, references empty), copied at creation.
    templates: Vec<Box<[Slot]>>,
    /// Indexed by [`ObjectId`]; `None` once deleted.
    objects: Vec<Option<Entry>>,
    lists: Vec<Vec<ObjectId>>,
    strings: StringArena,
    root: Option<ObjectId>,
}

impl Store {
    /// An empty store for the objects of `model`.
    pub fn new(model: Arc<Model>) -> Store {
        let templates = model
            .classes()
            .iter()
            .map(|class| {
                let attributes = class.attributes().iter().map(|_| Slot::Unset);
                let references = class.references().iter().map(|r| {
                    if r.is_many() {
                        Slot::Many(None)
                    } else {
                        Slot::One(None)
                    }
                });
                attributes.chain(references).collect()
            })
            .collect();
        Store {
            model,
            templates,
            objects: Vec::new(),
            lists: Vec::new(),
            strings: StringArena::new(),
            root: None,
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Creates the root object.
    pub(crate) fn create_root(&mut self, class: ClassId) -> Result<ObjectId, StoreError> {
        if self.root.is_some() {
            return Err(StoreError::RootAlreadyExists);
        }
        check_class(&self.model, class, None)?;
        let id = self.new_entry(class, None);
        self.root = Some(id);
        Ok(id)
    }

    pub fn root(&self) -> Option<ObjectId> {
        self.root
    }

    /// How many identifiers were given so far, deleted objects included.
    pub(crate) fn ids_given(&self) -> usize {
        self.objects.len()
    }

    /// Creates an object of class `class` in the containment reference `reference` of `parent`.
    pub(crate) fn create_child(
        &mut self,
        parent: ObjectId,
        reference: RefId,
        position: Position,
        class: ClassId,
    ) -> Result<ObjectId, StoreError> {
        let slot_index = self.ref_slot(self.entry(parent).class, reference);
        check_class(
            &self.model,
            class,
            Some(self.model.reference(reference).target()),
        )?;
        // Check everything before creating anything: on error, the store is unchanged.
        let insert_at = match self.entry(parent).slots[slot_index] {
            Slot::One(Some(_)) => {
                return Err(StoreError::ReferenceAlreadySet {
                    reference: self.model.reference(reference).name().to_owned(),
                });
            }
            Slot::One(None) => check_position(position, 0)?,
            Slot::Many(None) => check_position(position, 0)?,
            Slot::Many(Some(list)) => check_position(position, self.lists[list as usize].len())?,
            _ => unreachable!("reference slot holds an attribute value"),
        };

        let child = self.new_entry(class, Some((parent, reference)));
        let slot = self.entry(parent).slots[slot_index];
        match slot {
            Slot::One(_) => self.entry_mut(parent).slots[slot_index] = Slot::One(Some(child)),
            Slot::Many(Some(list)) => self.lists[list as usize].insert(insert_at, child),
            Slot::Many(None) => self.new_list(parent, slot_index, vec![child]),
            _ => unreachable!("reference slot holds an attribute value"),
        }
        Ok(child)
    }

    /// Gives a list, holding `children`, to the empty list slot `slot_index` of `parent`: lists
    /// are allocated on the first child only.
    fn new_list(&mut self, parent: ObjectId, slot_index: usize, children: Vec<ObjectId>) {
        let list = u32::try_from(self.lists.len()).expect("too many lists");
        self.lists.push(children);
        self.entry_mut(parent).slots[slot_index] = Slot::Many(Some(list));
    }

    /// `true` if `obj` designates an object that exists (not deleted).
    pub fn is_alive(&self, obj: ObjectId) -> bool {
        matches!(self.objects.get(obj.index()), Some(Some(_)))
    }

    pub fn class_of(&self, obj: ObjectId) -> ClassId {
        self.entry(obj).class
    }

    /// The container of `obj` and the reference that holds it; `None` for the root.
    pub fn container(&self, obj: ObjectId) -> Option<(ObjectId, RefId)> {
        self.entry(obj).container
    }

    /// The children of a containment reference, in order. A single reference gives zero or one.
    pub fn children(&self, obj: ObjectId, reference: RefId) -> &[ObjectId] {
        let entry = self.entry(obj);
        match &entry.slots[self.ref_slot(entry.class, reference)] {
            Slot::One(Some(child)) => std::slice::from_ref(child),
            Slot::Many(Some(list)) => &self.lists[*list as usize],
            Slot::One(None) | Slot::Many(None) => &[],
            _ => unreachable!("reference slot holds an attribute value"),
        }
    }

    pub fn get(&self, obj: ObjectId, attr: AttrId) -> Value<'_> {
        let entry = self.entry(obj);
        check_owner(&self.model, entry.class, attr.owner());
        match entry.slots[attr.index()] {
            Slot::Unset => Value::Unset,
            Slot::Int(v) => Value::Int(v),
            Slot::Float(v) => Value::Float(v),
            Slot::Bool(v) => Value::Bool(v),
            Slot::Str(r) => Value::Str(self.strings.get(r)),
            Slot::Enum(v) => Value::Enum(v),
            Slot::Date(code) => Value::Date(decode_date(code)),
            Slot::DateTime(code) => Value::DateTime(decode_datetime(code)),
            Slot::One(_) | Slot::Many(_) => unreachable!("attribute slot holds a reference"),
        }
    }

    /// Gives a value to an attribute, or removes it with [`Value::Unset`].
    pub(crate) fn set(
        &mut self,
        obj: ObjectId,
        attr: AttrId,
        value: Value<'_>,
    ) -> Result<(), StoreError> {
        let class = self.entry(obj).class;
        check_owner(&self.model, class, attr.owner());
        check_value(&self.model, attr, &value)?;
        let slot = match value {
            Value::Unset => Slot::Unset,
            Value::Int(v) => Slot::Int(v),
            Value::Float(v) => Slot::Float(v),
            Value::Bool(v) => Slot::Bool(v),
            Value::Str(text) => Slot::Str(self.strings.push(text)),
            Value::Enum(v) => Slot::Enum(v),
            Value::Date(date) => Slot::Date(encode_date(date)),
            Value::DateTime(instant) => Slot::DateTime(encode_datetime(instant)),
        };
        self.entry_mut(obj).slots[attr.index()] = slot;
        Ok(())
    }

    /// Deletes an object and all its descendants, and removes it from its container.
    ///
    /// Returns the container, the reference and the position the object had (0 in a single
    /// reference).
    ///
    /// # Panics
    /// If the object does not exist, or is the root: the [`Document`](crate::Document) refuses
    /// the root before calling.
    pub(crate) fn delete(&mut self, obj: ObjectId) -> (ObjectId, RefId, usize) {
        // Remove the object from its container.
        let (parent, reference) = self.entry(obj).container.expect("cannot delete the root");
        let slot_index = self.ref_slot(self.entry(parent).class, reference);
        let index = match self.entry(parent).slots[slot_index] {
            Slot::One(_) => {
                self.entry_mut(parent).slots[slot_index] = Slot::One(None);
                0
            }
            Slot::Many(Some(list)) => {
                let list = &mut self.lists[list as usize];
                let index = list.iter().position(|&c| c == obj).expect("child in list");
                // Shifts the following children, which keep their order.
                list.remove(index);
                index
            }
            _ => unreachable!("object missing from its container"),
        };

        self.free_subtree(obj);
        (parent, reference, index)
    }

    /// Deletes several objects and all their descendants, and removes them from their
    /// containers. Each list is filtered once, whatever the number of its children deleted; the
    /// remaining children keep their order. An object whose ancestor is also in `objs` is
    /// deleted with the ancestor, and an object given twice is deleted once.
    ///
    /// Returns, for each object removed from its container, in the order of `objs`: the object,
    /// its container, the reference, and its position before the deletion (0 in a single
    /// reference).
    ///
    /// # Panics
    /// If an object does not exist, or is the root.
    pub(crate) fn delete_many(
        &mut self,
        objs: &[ObjectId],
    ) -> Vec<(ObjectId, ObjectId, RefId, usize)> {
        let batch: HashSet<ObjectId> = objs.iter().copied().collect();
        // The objects to remove from their containers: those without an ancestor in the batch,
        // each once, in the order of `objs`.
        let mut seen = HashSet::new();
        let mut heads = Vec::new();
        for &obj in objs {
            let (parent, reference) = self.entry(obj).container.expect("cannot delete the root");
            if !seen.insert(obj) || self.has_ancestor_in(parent, &batch) {
                continue;
            }
            heads.push((obj, parent, reference));
        }

        // Group the heads by list: one pass per list, whatever the number of its heads.
        let mut by_list: HashMap<u32, HashSet<ObjectId>> = HashMap::new();
        let mut positions: HashMap<ObjectId, usize> = HashMap::new();
        for &(obj, parent, reference) in &heads {
            let slot_index = self.ref_slot(self.entry(parent).class, reference);
            match self.entry(parent).slots[slot_index] {
                Slot::One(_) => {
                    self.entry_mut(parent).slots[slot_index] = Slot::One(None);
                    positions.insert(obj, 0);
                }
                Slot::Many(Some(list)) => {
                    by_list.entry(list).or_default().insert(obj);
                }
                _ => unreachable!("object missing from its container"),
            }
        }
        for (list, doomed) in by_list {
            // `retain` visits the children once, in order: their positions are counted on the
            // way, before anything moves.
            let mut index = 0;
            self.lists[list as usize].retain(|child| {
                let keep = !doomed.contains(child);
                if !keep {
                    positions.insert(*child, index);
                }
                index += 1;
                keep
            });
        }

        let mut deleted = Vec::with_capacity(heads.len());
        for (obj, parent, reference) in heads {
            self.free_subtree(obj);
            deleted.push((obj, parent, reference, positions[&obj]));
        }
        deleted
    }

    /// All the descendants of `obj`, `obj` included: each once, a parent before its children,
    /// and the children of a reference in their order (a depth-first, pre-order walk).
    pub fn descendants(&self, obj: ObjectId) -> Vec<ObjectId> {
        let mut result = Vec::new();
        // An explicit stack rather than a recursion that a deep tree could overflow.
        let mut stack = vec![obj];
        while let Some(current) = stack.pop() {
            result.push(current);
            // Pushed in reverse, so that they are popped in order.
            for slot in self.entry(current).slots.iter().rev() {
                match *slot {
                    Slot::One(Some(child)) => stack.push(child),
                    Slot::Many(Some(list)) => {
                        stack.extend(self.lists[list as usize].iter().rev());
                    }
                    _ => {}
                }
            }
        }
        result
    }

    /// `true` if `obj` or one of its ancestors is in `set`.
    fn has_ancestor_in(&self, obj: ObjectId, set: &HashSet<ObjectId>) -> bool {
        let mut current = Some(obj);
        while let Some(obj) = current {
            if set.contains(&obj) {
                return true;
            }
            current = self.entry(obj).container.map(|(parent, _)| parent);
        }
        false
    }

    /// Marks `obj` and its descendants dead, and frees their blocks. `obj` must already be
    /// removed from its container.
    fn free_subtree(&mut self, obj: ObjectId) {
        // An explicit stack rather than a recursion that a deep tree could overflow.
        let mut stack = vec![obj];
        while let Some(current) = stack.pop() {
            let entry = self.objects[current.index()]
                .take()
                .expect("alive descendant");
            for slot in &entry.slots {
                match *slot {
                    Slot::One(Some(child)) => stack.push(child),
                    Slot::Many(Some(list)) => {
                        stack.extend(std::mem::take(&mut self.lists[list as usize]));
                    }
                    _ => {}
                }
            }
            // `entry` goes out of scope here: its block is freed.
        }
    }

    fn entry(&self, obj: ObjectId) -> &Entry {
        match self.objects.get(obj.index()) {
            Some(Some(entry)) => entry,
            _ => panic!("object {obj:?} does not exist"),
        }
    }

    fn entry_mut(&mut self, obj: ObjectId) -> &mut Entry {
        match self.objects.get_mut(obj.index()) {
            Some(Some(entry)) => entry,
            _ => panic!("object {obj:?} does not exist"),
        }
    }

    /// Index of the slot of `reference` in the block of an object of class `class`.
    fn ref_slot(&self, class: ClassId, reference: RefId) -> usize {
        check_owner(&self.model, class, reference.owner());
        self.model.class(class).attributes().len() + reference.index()
    }

    fn new_entry(&mut self, class: ClassId, container: Option<(ObjectId, RefId)>) -> ObjectId {
        let id = ObjectId::from_index(self.objects.len());
        let slots = self.templates[class.index()].clone();
        self.objects.push(Some(Entry {
            class,
            container,
            slots,
        }));
        id
    }
}

/// Checks that an object of class `class` may be created in a reference of type `target`.
fn check_class(model: &Model, class: ClassId, target: Option<ClassId>) -> Result<(), StoreError> {
    let compiled = model.class(class);
    if compiled.is_abstract() {
        return Err(StoreError::AbstractClass {
            class: compiled.name().to_owned(),
        });
    }
    if let Some(target) = target
        && !model.is_subclass_of(class, target)
    {
        return Err(StoreError::NonConformingClass {
            class: compiled.name().to_owned(),
            expected: model.class(target).name().to_owned(),
        });
    }
    Ok(())
}

/// Checks that `value` fits attribute `attr`.
fn check_value(model: &Model, attr: AttrId, value: &Value<'_>) -> Result<(), StoreError> {
    let attribute = model.attribute(attr);
    let fits = match (attribute.attr_type(), value) {
        (_, Value::Unset)
        | (AttrType::String, Value::Str(_))
        | (AttrType::Int, Value::Int(_))
        | (AttrType::Float, Value::Float(_))
        | (AttrType::Boolean, Value::Bool(_))
        | (AttrType::Date, Value::Date(_)) => true,
        (AttrType::DateTime, Value::DateTime(instant)) => is_whole_microsecond(*instant),
        (AttrType::Enum(id), Value::Enum(literal)) => {
            (*literal as usize) < model.enum_type(id).literals().len()
        }
        _ => false,
    };
    if fits {
        Ok(())
    } else {
        Err(StoreError::WrongValueType {
            attribute: attribute.name().to_owned(),
            expected: model.type_name(attribute.attr_type()).to_owned(),
        })
    }
}

/// Panics if `class` does not have the feature declared in `owner`: a programming error.
#[inline]
fn check_owner(model: &Model, class: ClassId, owner: ClassId) {
    if !model.is_subclass_of(class, owner) {
        panic!(
            "class `{}` has no feature of class `{}`",
            model.class(class).name(),
            model.class(owner).name()
        );
    }
}

/// Index where a new child goes in a list of `len` children, or an error if `position` is past
/// the end.
fn check_position(position: Position, len: usize) -> Result<usize, StoreError> {
    match position {
        Position::End => Ok(len),
        Position::At(index) if index <= len => Ok(index),
        Position::At(index) => Err(StoreError::PositionOutOfRange {
            position: index,
            len,
        }),
    }
}

#[cfg(test)]
mod tests;
