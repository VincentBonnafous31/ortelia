//! The JSON reader: reads a data file into a new store, as a stream, without an intermediate
//! tree, with the identity mapping: a key is the name of an attribute or of a containment
//! reference of the object's class, and `_type_` gives the concrete class of an object.
//!
//! It is a chain of serde `DeserializeSeed`s: a `Deserialize` that carries state, here the
//! builder and the expected class. serde_json calls them as it reads, like a SAX parser calls
//! its handler, and they call the builder in file order. The reader only uses the public API of
//! the builder, as a reading plugin would.
//!
//! `_type_` (see [`ObjectSeed`]):
//! - if the declared type of the reference has no subclass, the class is known in advance;
//! - otherwise, if the first key is `_type_`, the object is created at once with that class
//!   (the files Ortelia will write);
//! - otherwise, the object is read whole into a `serde_json::Map`, then replayed by the same
//!   seed. Only these objects go through an intermediate tree.
//!
//! The first error stops the load.

use std::fmt;
use std::io::{BufReader, Read};
use std::sync::Arc;

use ortelia_model::builder::{Builder, LoadError, LoadErrorKind};
use ortelia_model::metamodel::{AttrId, ClassId, Model, RefId};
use ortelia_model::store::Store;
use serde::de::value::{MapAccessDeserializer, MapDeserializer};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Unexpected, Visitor};

/// The reserved key that gives the concrete class of an object.
pub const TYPE_KEY: &str = "_type_";

/// Size of the read buffer of a stream: serde_json does not buffer by itself.
const BUFFER_SIZE: usize = 64 * 1024;

/// Reads a JSON file from a stream into a new store. The file's value is the root object, of
/// class `root` (or one of its subclasses, given by `_type_`).
///
/// The file is read as it comes, so it is never in memory as a whole.
pub fn read_json(reader: impl Read, model: Arc<Model>, root: ClassId) -> Result<Store, LoadError> {
    let reader = BufReader::with_capacity(BUFFER_SIZE, reader);
    read(
        &mut serde_json::Deserializer::from_reader(reader),
        model,
        root,
    )
}

/// Reads a JSON file already in memory into a new store, like [`read_json`].
pub fn read_json_slice(bytes: &[u8], model: Arc<Model>, root: ClassId) -> Result<Store, LoadError> {
    read(
        &mut serde_json::Deserializer::from_slice(bytes),
        model,
        root,
    )
}

fn read<'de, R>(
    de: &mut serde_json::Deserializer<R>,
    model: Arc<Model>,
    root: ClassId,
) -> Result<Store, LoadError>
where
    R: serde_json::de::Read<'de>,
{
    let mut ctx = Context {
        builder: Builder::new(model.clone()),
        model,
        error: None,
    };
    let seed = ObjectSeed {
        ctx: &mut ctx,
        declared: root,
        is_root: true,
        class: None,
    };
    // `end` checks that nothing but white space follows the root object.
    match seed.deserialize(&mut *de).and_then(|()| de.end()) {
        Ok(()) => Ok(ctx.builder.finish()),
        Err(error) => Err(match ctx.error.take() {
            // The builder refused something: its error, saved in the context, is the real one.
            Some(load_error) => load_error,
            None => {
                let kind = if error.is_io() {
                    LoadErrorKind::Io {
                        message: error.to_string(),
                    }
                } else {
                    LoadErrorKind::Syntax {
                        line: error.line(),
                        column: error.column(),
                    }
                };
                ctx.builder.error(kind)
            }
        }),
    }
}

/// The state shared by all the seeds of a load.
struct Context {
    builder: Builder,
    /// The model, also held by the builder; kept here so that it can be read while the builder
    /// is borrowed mutably.
    model: Arc<Model>,
    /// The error of the builder, if any. serde only accepts its own error type: a visitor saves
    /// the builder's error here, then returns a serde error to stop everything.
    error: Option<LoadError>,
}

impl Context {
    /// Saves a load error and returns the serde error that stops the load.
    fn fail<E: de::Error>(&mut self, error: LoadError) -> E {
        self.error = Some(error);
        E::custom("load refused")
    }

    /// Begins an object of class `class`, declared as `declared`.
    fn begin<E: de::Error>(
        &mut self,
        class: ClassId,
        declared: ClassId,
        is_root: bool,
    ) -> Result<(), E> {
        let result = if !is_root {
            // The store checks that the class is concrete and conforms to the reference.
            self.builder.begin_object(class)
        } else if self.model.is_subclass_of(class, declared) {
            self.builder.begin_root(class)
        } else {
            Err(self
                .builder
                .error(self.non_conforming(self.model.class(class).name(), declared)))
        };
        result.map_err(|error| self.fail(error))
    }

    /// The error of an unknown class name, at the object being read.
    fn unknown_class<E: de::Error>(&mut self, name: String, is_root: bool) -> E {
        let kind = LoadErrorKind::UnknownClass { name };
        let error = if is_root {
            self.builder.error(kind)
        } else {
            self.builder.fail_object(kind)
        };
        self.fail(error)
    }

    fn non_conforming(&self, class: &str, expected: ClassId) -> LoadErrorKind {
        LoadErrorKind::NonConformingClass {
            class: class.to_owned(),
            expected: self.model.class(expected).name().to_owned(),
        }
    }
}

/// Reads an object (a JSON object) whose declared type is `declared`.
struct ObjectSeed<'c> {
    ctx: &'c mut Context,
    declared: ClassId,
    is_root: bool,
    /// The class, when already known: the replay of an object whose `_type_` came late.
    class: Option<ClassId>,
}

impl<'de> DeserializeSeed<'de> for ObjectSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ObjectSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let ctx = self.ctx;
        let class = match self.class {
            Some(class) => class,
            None if !ctx.model.class(self.declared).has_subclasses() => self.declared,
            None => match map.next_key_seed(FirstKeySeed)? {
                // An empty object: the declared type, refused later if it is abstract.
                None => self.declared,
                Some(FirstKey::Type) => {
                    match map.next_value_seed(ClassNameSeed { model: &ctx.model })? {
                        Ok(class) => class,
                        Err(name) => return Err(ctx.unknown_class(name, self.is_root)),
                    }
                }
                Some(FirstKey::Other(key)) => {
                    // `_type_` may come later: read the whole object, then replay it. A list of
                    // entries rather than a map keeps the order and the duplicate keys, so that
                    // the replay gives the same result as an object read with `_type_` first.
                    let mut entries = vec![(key, map.next_value::<serde_json::Value>()?)];
                    while let Some(entry) = map.next_entry::<String, serde_json::Value>()? {
                        entries.push(entry);
                    }
                    let type_entry = entries.iter().position(|(key, _)| key == TYPE_KEY);
                    let class = match type_entry.map(|index| entries.remove(index).1) {
                        None => self.declared,
                        // The same seed as for a `_type_` read first.
                        Some(name) => {
                            let seed = ClassNameSeed { model: &ctx.model };
                            match seed.deserialize(name).map_err(de::Error::custom)? {
                                Ok(class) => class,
                                Err(name) => return Err(ctx.unknown_class(name, self.is_root)),
                            }
                        }
                    };
                    let replay = ObjectSeed {
                        ctx,
                        declared: self.declared,
                        is_root: self.is_root,
                        class: Some(class),
                    };
                    // A map deserializer over the entries, whose values are themselves
                    // deserializers (`serde_json::Value`): the same code reads both.
                    let entries = MapDeserializer::<_, serde_json::Error>::new(entries.into_iter());
                    return replay.deserialize(entries).map_err(de::Error::custom);
                }
            },
        };
        ctx.begin(class, self.declared, self.is_root)?;

        while let Some(key) = map.next_key_seed(KeySeed { ctx: &mut *ctx })? {
            match key {
                Key::Type => {
                    // The class is already known: `_type_` must name it.
                    let resolved = map.next_value_seed(ClassNameSeed { model: &ctx.model })?;
                    match resolved {
                        Ok(named) if named == class => {}
                        Ok(named) => {
                            // A second `_type_` that names another class than the first.
                            let kind = ctx.non_conforming(ctx.model.class(named).name(), class);
                            let error = ctx.builder.error(kind);
                            return Err(ctx.fail(error));
                        }
                        Err(name) => {
                            let error = ctx.builder.error(LoadErrorKind::UnknownClass { name });
                            return Err(ctx.fail(error));
                        }
                    }
                }
                Key::Attribute(attr) => map.next_value_seed(ValueSeed {
                    ctx: &mut *ctx,
                    attr,
                })?,
                Key::Reference(reference) => {
                    ctx.builder.begin_reference(reference);
                    map.next_value_seed(ReferenceSeed {
                        ctx: &mut *ctx,
                        reference,
                    })?;
                    ctx.builder.end_reference();
                }
            }
        }
        ctx.builder.end_object();
        Ok(())
    }
}

/// The first key of an object whose class is not known yet.
enum FirstKey {
    Type,
    Other(String),
}

struct FirstKeySeed;

impl<'de> DeserializeSeed<'de> for FirstKeySeed {
    type Value = FirstKey;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<FirstKey, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for FirstKeySeed {
    type Value = FirstKey;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a key")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<FirstKey, E> {
        Ok(if key == TYPE_KEY {
            FirstKey::Type
        } else {
            FirstKey::Other(key.to_owned())
        })
    }
}

/// Reads the value of `_type_` and resolves it to a class: `Err(name)` if the name is unknown.
struct ClassNameSeed<'m> {
    model: &'m Model,
}

impl<'de> DeserializeSeed<'de> for ClassNameSeed<'_> {
    type Value = Result<ClassId, String>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for ClassNameSeed<'_> {
    type Value = Result<ClassId, String>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a class name")
    }

    fn visit_str<E: de::Error>(self, name: &str) -> Result<Self::Value, E> {
        // Allocates only for an unknown name, which ends the load.
        Ok(self.model.class_id(name).ok_or_else(|| name.to_owned()))
    }
}

/// What a key designates in the class of the current object.
enum Key {
    Type,
    Attribute(AttrId),
    Reference(RefId),
}

/// Reads a key and resolves it in the class of the current object. The key arrives as a
/// short-lived `&str`, without allocation.
struct KeySeed<'c> {
    ctx: &'c mut Context,
}

impl<'de> DeserializeSeed<'de> for KeySeed<'_> {
    type Value = Key;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Key, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for KeySeed<'_> {
    type Value = Key;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a key")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<Key, E> {
        if key == TYPE_KEY {
            return Ok(Key::Type);
        }
        let class = self.ctx.model.class(self.ctx.builder.current_class());
        if let Some(attr) = class.attribute(key) {
            Ok(Key::Attribute(attr))
        } else if let Some(reference) = class.reference(key) {
            Ok(Key::Reference(reference))
        } else {
            let error = self.ctx.builder.error(LoadErrorKind::UnknownKey {
                key: key.to_owned(),
            });
            Err(self.ctx.fail(error))
        }
    }
}

/// Reads the value of an attribute, and hands it over to the builder according to what the file
/// holds: a string to convert, a number, a boolean, or null.
struct ValueSeed<'c> {
    ctx: &'c mut Context,
    attr: AttrId,
}

impl ValueSeed<'_> {
    fn done<E: de::Error>(self, result: Result<(), LoadError>) -> Result<(), E> {
        result.map_err(|error| self.ctx.fail(error))
    }

    fn wrong_type<E: de::Error>(self) -> Result<(), E> {
        let error = self.ctx.builder.wrong_type(self.attr);
        Err(self.ctx.fail(error))
    }
}

impl<'de> DeserializeSeed<'de> for ValueSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ValueSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an attribute value")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<(), E> {
        let result = self.ctx.builder.set_text(self.attr, text);
        self.done(result)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<(), E> {
        let result = self.ctx.builder.set_int(self.attr, value);
        self.done(result)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<(), E> {
        match i64::try_from(value) {
            Ok(value) => self.visit_i64(value),
            // Beyond `i64::MAX`: only a `Float` attribute can hold it, as JSON `1e19` would be.
            Err(_) => self.visit_f64(value as f64),
        }
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<(), E> {
        let result = self.ctx.builder.set_float(self.attr, value);
        self.done(result)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<(), E> {
        let result = self.ctx.builder.set_bool(self.attr, value);
        self.done(result)
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        let result = self.ctx.builder.set_null(self.attr);
        self.done(result)
    }

    fn visit_map<A: MapAccess<'de>>(self, _map: A) -> Result<(), A::Error> {
        self.wrong_type()
    }

    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<(), A::Error> {
        self.wrong_type()
    }
}

/// Reads the children of a containment reference: an array for a list, an object for a single
/// reference, or null for no child.
struct ReferenceSeed<'c> {
    ctx: &'c mut Context,
    reference: RefId,
}

impl<'de> DeserializeSeed<'de> for ReferenceSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ReferenceSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctx.model.reference(self.reference).is_many() {
            f.write_str("an array of objects")
        } else {
            f.write_str("an object")
        }
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let reference = self.ctx.model.reference(self.reference);
        if !reference.is_many() {
            return Err(de::Error::invalid_type(Unexpected::Seq, &self));
        }
        let declared = reference.target();
        loop {
            let seed = ObjectSeed {
                ctx: &mut *self.ctx,
                declared,
                is_root: false,
                class: None,
            };
            if seq.next_element_seed(seed)?.is_none() {
                return Ok(());
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<(), A::Error> {
        let reference = self.ctx.model.reference(self.reference);
        if reference.is_many() {
            return Err(de::Error::invalid_type(Unexpected::Map, &self));
        }
        let declared = reference.target();
        // The object is already open: `MapAccessDeserializer` turns its entries back into a
        // deserializer for the object seed.
        let seed = ObjectSeed {
            ctx: self.ctx,
            declared,
            is_root: false,
            class: None,
        };
        seed.deserialize(MapAccessDeserializer::new(map))
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
