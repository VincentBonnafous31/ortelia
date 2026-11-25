//! The JSON reader: reads a data file into a new fragment for a branch, as a stream, without an
//! intermediate tree, with the identity mapping: a key is the name of an attribute or of a
//! containment reference of the object's class, and `_type_` gives the concrete class of an
//! object.
//!
//! The value of the file is what the branch holds ([`Shape`]): an object for the root or a single
//! reference, an array of objects for a list.
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
//! A data error does not stop the reading: the builder records it, and the reader skips what was
//! refused (a value, or a whole object) with `IgnoredAny`, which reads a JSON value without
//! keeping anything. Only an error of syntax or of input-output stops it.

use std::fmt;
use std::io::{BufReader, Read};
use std::sync::Arc;

use ortelia_model::builder::{Builder, DataPath, LoadErrorKind, LoadFailure, Loaded};
use ortelia_model::fragment::Shape;
use ortelia_model::metamodel::{AttrId, ClassId, Model, RefId};
use serde::de::value::{MapAccessDeserializer, MapDeserializer};
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

/// The reserved key that gives the concrete class of an object.
pub const TYPE_KEY: &str = "_type_";

/// Size of the read buffer of a stream: serde_json does not buffer by itself.
const BUFFER_SIZE: usize = 64 * 1024;

/// What a JSON value must be to hold objects of a branch or a reference.
const AN_ARRAY: &str = "an array of objects";
const AN_OBJECT: &str = "an object";

/// Reads a JSON file from a stream into a new fragment, for a branch of shape `shape` whose
/// path in the document is `base` (it starts the path of every error).
///
/// The file is read as it comes, so it is never in memory as a whole.
pub fn read_json(
    reader: impl Read,
    model: Arc<Model>,
    shape: Shape,
    base: DataPath,
) -> Result<Loaded, LoadFailure> {
    let reader = BufReader::with_capacity(BUFFER_SIZE, reader);
    read(
        &mut serde_json::Deserializer::from_reader(reader),
        Builder::new(model, shape, base),
    )
}

/// Reads a JSON file already in memory into a new fragment, like [`read_json`].
pub fn read_json_slice(
    bytes: &[u8],
    model: Arc<Model>,
    shape: Shape,
    base: DataPath,
) -> Result<Loaded, LoadFailure> {
    read(
        &mut serde_json::Deserializer::from_slice(bytes),
        Builder::new(model, shape, base),
    )
}

fn read<'de, R>(
    de: &mut serde_json::Deserializer<R>,
    mut builder: Builder,
) -> Result<Loaded, LoadFailure>
where
    R: serde_json::de::Read<'de>,
{
    let shape = builder.shape();
    let seed = ObjectsSeed {
        builder: &mut builder,
        declared: shape.class,
        many: shape.many,
        is_head: true,
    };
    // `end` checks that nothing but white space follows the value.
    if let Err(error) = seed.deserialize(&mut *de).and_then(|()| de.end()) {
        // Only an error of syntax or of input-output reaches here: it ends the reading, and
        // joins the data errors found before it, at the position where the reading stopped.
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
        builder.report(kind);
    }
    builder.finish()
}

fn non_conforming(model: &Model, class: ClassId, expected: ClassId) -> LoadErrorKind {
    LoadErrorKind::NonConformingClass {
        class: model.class(class).name().to_owned(),
        expected: model.class(expected).name().to_owned(),
    }
}

/// Reads the rest of an object without keeping anything.
fn skip_map<'de, A: MapAccess<'de>>(mut map: A) -> Result<(), A::Error> {
    while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
    Ok(())
}

/// Reads the rest of an array without keeping anything.
fn skip_seq<'de, A: SeqAccess<'de>>(mut seq: A) -> Result<(), A::Error> {
    while seq.next_element::<IgnoredAny>()?.is_some() {}
    Ok(())
}

/// What a JSON value must be to hold one object at most, or a list of objects.
fn expected_for(many: bool) -> &'static str {
    if many { AN_ARRAY } else { AN_OBJECT }
}

/// Reads objects whose declared type is `declared`: an array for a list (`many`), an object for
/// a single branch or reference. Used for the value of the file (the heads of the fragment) and
/// for the value of a containment reference.
struct ObjectsSeed<'c> {
    builder: &'c mut Builder,
    declared: ClassId,
    many: bool,
    /// The objects are the heads of the fragment: the value of the file.
    is_head: bool,
}

impl ObjectsSeed<'_> {
    /// Records that the value has another kind than expected; the reader then skips it.
    fn wrong_structure(self) {
        let expected = expected_for(self.many).to_owned();
        self.builder
            .report(LoadErrorKind::WrongStructure { expected });
    }

    /// A value that cannot hold objects (a string, a number...). Its content is not kept: it must
    /// not reach the log.
    fn scalar<E: de::Error>(self) -> Result<(), E> {
        self.wrong_structure();
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for ObjectsSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ObjectsSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(expected_for(self.many))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        if !self.many {
            self.wrong_structure();
            return skip_seq(seq);
        }
        loop {
            let seed = ObjectSeed {
                builder: &mut *self.builder,
                declared: self.declared,
                is_head: self.is_head,
                class: None,
            };
            if seq.next_element_seed(seed)?.is_none() {
                return Ok(());
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<(), A::Error> {
        if self.many {
            self.wrong_structure();
            return skip_map(map);
        }
        // The object is already open: `MapAccessDeserializer` turns its entries back into a
        // deserializer for the object seed.
        let seed = ObjectSeed {
            builder: self.builder,
            declared: self.declared,
            is_head: self.is_head,
            class: None,
        };
        seed.deserialize(MapAccessDeserializer::new(map))
    }

    /// `null`: no child for a reference. The file itself must hold objects.
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        if self.is_head { self.scalar() } else { Ok(()) }
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        self.scalar()
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        self.scalar()
    }
}

/// Reads an object (a JSON object) whose declared type is `declared`.
struct ObjectSeed<'c> {
    builder: &'c mut Builder,
    declared: ClassId,
    is_head: bool,
    /// The class, when already known: the replay of an object whose `_type_` came late.
    class: Option<ClassId>,
}

impl ObjectSeed<'_> {
    /// Reads an object whose first key, `first_key`, is not `_type_`: `_type_` may come later.
    /// The whole object is read, then replayed with its class known. A list of entries rather
    /// than a map keeps the order and the duplicate keys, so that the replay gives the same
    /// result as an object read with `_type_` first.
    fn read_with_late_type<'de, A: MapAccess<'de>>(
        self,
        first_key: String,
        mut map: A,
    ) -> Result<(), A::Error> {
        let mut entries = vec![(first_key, map.next_value::<serde_json::Value>()?)];
        while let Some(entry) = map.next_entry::<String, serde_json::Value>()? {
            entries.push(entry);
        }
        let type_entry = entries.iter().position(|(key, _)| key == TYPE_KEY);
        let class = match type_entry.map(|index| entries.remove(index).1) {
            None => self.declared,
            // The same seed as for a `_type_` read first.
            Some(name) => {
                let seed = ClassNameSeed {
                    model: self.builder.model(),
                };
                match seed.deserialize(name).map_err(de::Error::custom)? {
                    Ok(class) => class,
                    Err(kind) => {
                        // The entries read are dropped instead of replayed.
                        self.builder.fail_object(kind);
                        return Ok(());
                    }
                }
            }
        };
        let replay = ObjectSeed {
            class: Some(class),
            ..self
        };
        // A map deserializer over the entries, whose values are themselves deserializers
        // (`serde_json::Value`): the same code reads both.
        let entries = MapDeserializer::<_, serde_json::Error>::new(entries.into_iter());
        replay.deserialize(entries).map_err(de::Error::custom)
    }

    /// A value that is not an object where an object is expected, in a list: it counts as an
    /// object refused, so that the following positions stay those of the file.
    fn not_an_object(self) {
        self.builder.fail_object(LoadErrorKind::WrongStructure {
            expected: AN_OBJECT.to_owned(),
        });
    }

    fn scalar<E: de::Error>(self) -> Result<(), E> {
        self.not_an_object();
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for ObjectSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        // `deserialize_any` rather than `deserialize_map`: a value of another kind reaches the
        // visitor, which records it, instead of a serde error that would stop the reading.
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ObjectSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(AN_OBJECT)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<(), A::Error> {
        self.not_an_object();
        skip_seq(seq)
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.scalar()
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        self.scalar()
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        self.scalar()
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        self.scalar()
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let builder = self.builder;
        let class = match self.class {
            Some(class) => class,
            None if !builder.model().class(self.declared).has_subclasses() => self.declared,
            None => match map.next_key_seed(FirstKeySeed)? {
                // An empty object: the declared type, refused later if it is abstract.
                None => self.declared,
                Some(FirstKey::Type) => {
                    match map.next_value_seed(ClassNameSeed {
                        model: builder.model(),
                    })? {
                        Ok(class) => class,
                        Err(kind) => {
                            builder.fail_object(kind);
                            return skip_map(map);
                        }
                    }
                }
                Some(FirstKey::Other(key)) => {
                    let seed = ObjectSeed {
                        builder,
                        declared: self.declared,
                        is_head: self.is_head,
                        class: None,
                    };
                    return seed.read_with_late_type(key, map);
                }
            },
        };
        // The builder checks that the class is concrete and conforms.
        let begun = if self.is_head {
            builder.begin_head(class)
        } else {
            builder.begin_object(class)
        };
        if !begun {
            // The object is refused (abstract or non-conforming class): its content is skipped.
            return skip_map(map);
        }

        while let Some(key) = map.next_key_seed(KeySeed {
            builder: &mut *builder,
        })? {
            match key {
                Key::Unknown => {
                    map.next_value::<IgnoredAny>()?;
                }
                Key::Type => {
                    // The class is already known: `_type_` must name it.
                    match map.next_value_seed(ClassNameSeed {
                        model: builder.model(),
                    })? {
                        Ok(named) if named == class => {}
                        Ok(named) => {
                            // A second `_type_` that names another class than the first.
                            let kind = non_conforming(builder.model(), named, class);
                            builder.report(kind);
                        }
                        Err(kind) => builder.report(kind),
                    }
                }
                Key::Attribute(attr) => map.next_value_seed(ValueSeed {
                    builder: &mut *builder,
                    attr,
                })?,
                Key::Reference(reference) => {
                    builder.begin_reference(reference);
                    let compiled = builder.model().reference(reference);
                    let seed = ObjectsSeed {
                        declared: compiled.target(),
                        many: compiled.is_many(),
                        builder: &mut *builder,
                        is_head: false,
                    };
                    map.next_value_seed(seed)?;
                    builder.end_reference();
                }
            }
        }
        builder.end_object();
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

/// Reads the value of `_type_` and resolves it to a class, or gives the error to record: an
/// unknown name, or a value that is not a text. The value is consumed in every case.
struct ClassNameSeed<'m> {
    model: &'m Model,
}

/// A class, or the error of a `_type_` that names none.
type ClassName = Result<ClassId, LoadErrorKind>;

fn not_a_class_name() -> ClassName {
    Err(LoadErrorKind::WrongStructure {
        expected: "a class name".to_owned(),
    })
}

impl<'de> DeserializeSeed<'de> for ClassNameSeed<'_> {
    type Value = ClassName;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<ClassName, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ClassNameSeed<'_> {
    type Value = ClassName;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a class name")
    }

    fn visit_str<E: de::Error>(self, name: &str) -> Result<ClassName, E> {
        // Allocates only for an unknown name, which is an error.
        Ok(self
            .model
            .class_id(name)
            .ok_or_else(|| LoadErrorKind::UnknownClass {
                name: name.to_owned(),
            }))
    }

    fn visit_unit<E: de::Error>(self) -> Result<ClassName, E> {
        Ok(not_a_class_name())
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<ClassName, E> {
        Ok(not_a_class_name())
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<ClassName, E> {
        Ok(not_a_class_name())
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<ClassName, E> {
        Ok(not_a_class_name())
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<ClassName, E> {
        Ok(not_a_class_name())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<ClassName, A::Error> {
        skip_seq(seq)?;
        Ok(not_a_class_name())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<ClassName, A::Error> {
        skip_map(map)?;
        Ok(not_a_class_name())
    }
}

/// What a key designates in the class of the current object.
enum Key {
    Type,
    Attribute(AttrId),
    Reference(RefId),
    /// A key that designates nothing: the error is recorded, the value is to be skipped.
    Unknown,
}

/// Reads a key and resolves it in the class of the current object. The key arrives as a
/// short-lived `&str`, without allocation.
struct KeySeed<'c> {
    builder: &'c mut Builder,
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
        let class = self.builder.model().class(self.builder.current_class());
        Ok(if let Some(attr) = class.attribute(key) {
            Key::Attribute(attr)
        } else if let Some(reference) = class.reference(key) {
            Key::Reference(reference)
        } else {
            self.builder.report(LoadErrorKind::UnknownKey {
                key: key.to_owned(),
            });
            Key::Unknown
        })
    }
}

/// Reads the value of an attribute, and hands it over to the builder according to what the file
/// holds: a string to convert, a number, a boolean, or null. The builder records a value it
/// refuses.
struct ValueSeed<'c> {
    builder: &'c mut Builder,
    attr: AttrId,
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
        self.builder.set_text(self.attr, text);
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<(), E> {
        self.builder.set_int(self.attr, value);
        Ok(())
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<(), E> {
        match i64::try_from(value) {
            Ok(value) => self.visit_i64(value),
            // Beyond `i64::MAX`: only a `Float` attribute can hold it, as JSON `1e19` would be.
            Err(_) => self.visit_f64(value as f64),
        }
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<(), E> {
        self.builder.set_float(self.attr, value);
        Ok(())
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<(), E> {
        self.builder.set_bool(self.attr, value);
        Ok(())
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.builder.set_null(self.attr);
        Ok(())
    }

    // An object or an array for an attribute: an error of type, without a value to show.
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<(), A::Error> {
        self.builder.report_wrong_type(self.attr, None);
        skip_map(map)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<(), A::Error> {
        self.builder.report_wrong_type(self.attr, None);
        skip_seq(seq)
    }
}

#[cfg(test)]
mod tests;
