//! A fragment: objects read for one branch of a document, not attached to it yet.
//!
//! A reader always fills a new fragment ([`Builder`](crate::builder::Builder)), then the document
//! grafts it onto its branch ([`Edit::replace`](crate::document::Edit::replace)). The fragment
//! owns a store of its own, so it can be read in another thread while the document is displayed,
//! then handed over to the main thread: it is `Send`.

use crate::metamodel::{ClassId, Model, RefId};
use crate::store::{ObjectId, Store};

/// What a branch holds: objects of a type (the class or one of its subclasses), one at most or a
/// list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub class: ClassId,
    pub many: bool,
}

impl Shape {
    /// The shape of the root of a document of class `class`: one object.
    pub fn root(class: ClassId) -> Shape {
        Shape { class, many: false }
    }

    /// The shape of a containment reference: its declared type and its multiplicity.
    pub fn of(model: &Model, reference: RefId) -> Shape {
        let reference = model.reference(reference);
        Shape {
            class: reference.target(),
            many: reference.is_many(),
        }
    }

    /// The shape in words, for messages: "a `Station`" or "a list of `Station`".
    pub fn describe(self, model: &Model) -> String {
        let name = model.class(self.class).name();
        if self.many {
            format!("a list of `{name}`")
        } else {
            format!("a `{name}`")
        }
    }
}

/// Objects read for a branch of a given [`Shape`], in a store of their own.
///
/// Its *heads* are the objects of the branch itself, in order: one at most for a single shape,
/// any number for a list. They have no container, and the store has no root.
#[derive(Debug)]
pub struct Fragment {
    store: Store,
    heads: Vec<ObjectId>,
    shape: Shape,
}

impl Fragment {
    pub(crate) fn new(store: Store, heads: Vec<ObjectId>, shape: Shape) -> Fragment {
        Fragment {
            store,
            heads,
            shape,
        }
    }

    /// The objects of the fragment, to read them.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// The objects of the branch, in order.
    pub fn heads(&self) -> &[ObjectId] {
        &self.heads
    }

    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// The number of objects of the fragment, heads and descendants. A builder never deletes an
    /// object: all those it created are there.
    pub fn object_count(&self) -> usize {
        self.store.ids_given()
    }

    pub(crate) fn into_parts(self) -> (Store, Vec<ObjectId>) {
        (self.store, self.heads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compiles only if `T` can be sent to another thread.
    fn assert_send<T: Send>() {}

    #[test]
    fn fragment_is_send() {
        assert_send::<Fragment>();
    }
}
