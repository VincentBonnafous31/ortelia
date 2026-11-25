//! What a fragment needs from the store: objects without a container (the heads of a
//! fragment), and the absorption of a fragment's store into a document's.
//!
//! A child module of `store`, so it sees the private fields of [`Store`].

use std::sync::Arc;

use super::{ObjectId, Slot, Store, StoreError, check_class};
use crate::metamodel::{ClassId, Model, RefId};

impl Store {
    /// The model, shared: lets the document check that a fragment was built with its model.
    pub(crate) fn model_arc(&self) -> &Arc<Model> {
        &self.model
    }

    /// Creates an object without a container, of class `class`, which must be concrete and be
    /// `declared` or one of its subclasses: a head of a fragment.
    pub(crate) fn create_head(
        &mut self,
        class: ClassId,
        declared: ClassId,
    ) -> Result<ObjectId, StoreError> {
        check_class(&self.model, class, Some(declared))?;
        Ok(self.new_entry(class, None))
    }

    /// Moves all the objects of `other`, built with the same model, into this store. Their
    /// identifiers are shifted by the number of objects of this store: returns `heads`, objects
    /// of `other`, with their new identifiers.
    ///
    /// Nothing is copied but the table entries: each object keeps its block of values, each
    /// list its vector, each text block its `String`. One pass rewrites the identifiers inside
    /// the moved entries (containers, children, list numbers, text blocks). `other`'s root, if
    /// any, is not this store's root.
    ///
    /// # Panics
    /// If the objects would not fit in 32-bit identifiers.
    pub(crate) fn absorb(&mut self, other: Store, heads: Vec<ObjectId>) -> Vec<ObjectId> {
        let objects = self.objects.len();
        let lists = u32::try_from(self.lists.len()).expect("too many lists");
        assert!(
            u32::try_from(objects + other.objects.len()).is_ok(),
            "too many objects"
        );
        let shift = |id: ObjectId| ObjectId::from_index(id.index() + objects);
        let blocks = self.strings.absorb(other.strings);

        self.objects.reserve(other.objects.len());
        for entry in other.objects {
            // A deleted object stays a free cell, so that the offset is the same for all.
            self.objects.push(entry.map(|mut entry| {
                entry.container = entry
                    .container
                    .map(|(parent, reference)| (shift(parent), reference));
                for slot in entry.slots.iter_mut() {
                    match slot {
                        Slot::One(Some(child)) => *child = shift(*child),
                        Slot::Many(Some(list)) => *list += lists,
                        Slot::Str(text) => *text = text.shifted(blocks),
                        _ => {}
                    }
                }
                entry
            }));
        }
        self.lists.reserve(other.lists.len());
        for mut list in other.lists {
            for child in list.iter_mut() {
                *child = shift(*child);
            }
            self.lists.push(list);
        }
        heads.into_iter().map(shift).collect()
    }

    /// Deletes all the children of a containment reference of `parent`, with their
    /// descendants.
    pub(crate) fn clear_reference(&mut self, parent: ObjectId, reference: RefId) {
        let slot_index = self.ref_slot(self.entry(parent).class, reference);
        let children = match self.entry(parent).slots[slot_index] {
            Slot::One(child) => {
                self.entry_mut(parent).slots[slot_index] = Slot::One(None);
                child.into_iter().collect()
            }
            // The list keeps its number, empty.
            Slot::Many(Some(list)) => std::mem::take(&mut self.lists[list as usize]),
            Slot::Many(None) => Vec::new(),
            _ => unreachable!("reference slot holds an attribute value"),
        };
        for child in children {
            self.free_subtree(child);
        }
    }

    /// Puts objects without a container, in order, in an empty containment reference of
    /// `parent`: at most one for a single reference.
    ///
    /// # Panics
    /// If the reference is not empty, or if `children` holds more than one object for a single
    /// reference.
    pub(crate) fn attach(&mut self, parent: ObjectId, reference: RefId, children: Vec<ObjectId>) {
        for &child in &children {
            let entry = self.entry_mut(child);
            assert!(entry.container.is_none(), "child already attached");
            entry.container = Some((parent, reference));
        }
        let slot_index = self.ref_slot(self.entry(parent).class, reference);
        match self.entry(parent).slots[slot_index] {
            Slot::One(None) => {
                assert!(children.len() <= 1, "two children in a single reference");
                self.entry_mut(parent).slots[slot_index] = Slot::One(children.first().copied());
            }
            Slot::Many(Some(list)) => {
                let list = &mut self.lists[list as usize];
                assert!(list.is_empty(), "list not empty");
                *list = children;
            }
            Slot::Many(None) if children.is_empty() => {}
            Slot::Many(None) => self.new_list(parent, slot_index, children),
            _ => panic!("reference not empty"),
        }
    }

    /// Replaces the root: deletes the old one with all its descendants, and makes `new_root`,
    /// an object without a container, the root.
    pub(crate) fn replace_root(&mut self, new_root: ObjectId) {
        assert!(
            self.entry(new_root).container.is_none(),
            "the new root has a container"
        );
        if let Some(old) = self.root.take() {
            self.free_subtree(old);
        }
        self.root = Some(new_root);
    }

    /// Checks that the store is a well-formed forest under `roots`, for the tests: every child
    /// has its parent for container, every living object is reached exactly once from `roots`,
    /// every list belongs to one object, and every text can be read.
    #[cfg(test)]
    pub(crate) fn assert_consistent(&self, roots: &[ObjectId]) {
        let mut reached = vec![false; self.objects.len()];
        let mut list_owners = vec![0usize; self.lists.len()];
        let mut stack: Vec<ObjectId> = roots.to_vec();
        for &root in roots {
            assert_eq!(self.entry(root).container, None, "root {root:?}");
        }
        while let Some(obj) = stack.pop() {
            assert!(!reached[obj.index()], "{obj:?} reached twice");
            reached[obj.index()] = true;
            let entry = self.entry(obj);
            let class = self.model.class(entry.class);
            for attribute in class.attributes() {
                // Reads the value, texts included: panics if a reference is wrong.
                let _ = self.get(obj, attribute.id());
            }
            for reference in class.references() {
                let slot = entry.slots[self.ref_slot(entry.class, reference.id())];
                if let Slot::Many(Some(list)) = slot {
                    list_owners[list as usize] += 1;
                }
                for &child in self.children(obj, reference.id()) {
                    assert_eq!(
                        self.entry(child).container,
                        Some((obj, reference.id())),
                        "container of {child:?}"
                    );
                    stack.push(child);
                }
            }
        }
        for (index, entry) in self.objects.iter().enumerate() {
            assert_eq!(
                entry.is_some(),
                reached[index],
                "object {index}: alive but not reached, or reached but dead"
            );
        }
        assert!(
            list_owners.iter().all(|&owners| owners <= 1),
            "a list shared by two objects"
        );
    }
}
