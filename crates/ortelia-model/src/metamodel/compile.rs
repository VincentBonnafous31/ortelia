//! Compilation of model files into a [`Model`], in four passes:
//! 1. index the classes, enums and data types by name;
//! 2. resolve the super classes and compute each chain of ancestors;
//! 3. build the layout of each class, ancestors first, resolving the types on the way;
//! 4. record which classes have subclasses.

use std::collections::HashMap;

use super::file::{ClassDef, DataTypeDef, EnumDef, ModelFile, PackageDef};
use super::{
    AttrId, AttrType, Attribute, BASE_TYPES, Class, ClassId, EnumId, EnumType, Model, ModelError,
    RefId, Reference,
};

/// What a type name designates, before data types are reduced to their base type.
#[derive(Clone, Copy)]
enum TypeEntry {
    Base(AttrType),
    Enum(EnumId),
    /// Index in the list of data types.
    Data(usize),
}

/// The declarations of all the files, flattened (nested packages included).
#[derive(Default)]
struct Declarations<'f> {
    classes: Vec<&'f ClassDef>,
    enums: Vec<&'f EnumDef>,
    data_types: Vec<&'f DataTypeDef>,
}

impl<'f> Declarations<'f> {
    fn collect(&mut self, package: &'f PackageDef) {
        self.classes.extend(&package.classes);
        self.enums.extend(&package.enums);
        self.data_types.extend(&package.data_types);
        for nested in &package.packages {
            self.collect(nested);
        }
    }
}

pub(super) fn compile(files: &[ModelFile]) -> Result<Model, ModelError> {
    let mut decls = Declarations::default();
    for package in files.iter().flat_map(|f| &f.packages) {
        decls.collect(package);
    }

    // Pass 1: names.
    let mut class_by_name = HashMap::new();
    for (index, class) in decls.classes.iter().enumerate() {
        let id = ClassId(small(index, "classes")?);
        if class_by_name.insert(class.name.clone(), id).is_some() {
            return Err(ModelError::DuplicateClass {
                name: class.name.clone(),
            });
        }
    }
    let mut types: HashMap<&str, TypeEntry> = HashMap::new();
    for (name, base) in BASE_TYPES {
        types.insert(name, TypeEntry::Base(base));
    }
    for (index, e) in decls.enums.iter().enumerate() {
        let entry = TypeEntry::Enum(EnumId(small(index, "enums")?));
        if types.insert(&e.name, entry).is_some() {
            return Err(ModelError::DuplicateType {
                name: e.name.clone(),
            });
        }
    }
    for (index, d) in decls.data_types.iter().enumerate() {
        if types.insert(&d.name, TypeEntry::Data(index)).is_some() {
            return Err(ModelError::DuplicateType {
                name: d.name.clone(),
            });
        }
    }

    // Pass 2: super classes and chains of ancestors.
    let mut super_of = Vec::with_capacity(decls.classes.len());
    for class in &decls.classes {
        let super_class = match &class.super_class {
            None => None,
            Some(name) => {
                Some(
                    *class_by_name
                        .get(name)
                        .ok_or_else(|| ModelError::UnknownSuperClass {
                            class: class.name.clone(),
                            super_class: name.clone(),
                        })?,
                )
            }
        };
        super_of.push(super_class);
    }
    let mut displays = Vec::with_capacity(decls.classes.len());
    for index in 0..decls.classes.len() {
        displays.push(display_of(index, &super_of, &decls.classes)?);
    }

    // Pass 3: layouts, super classes before their subclasses (sorted by depth).
    let mut order: Vec<usize> = (0..decls.classes.len()).collect();
    order.sort_by_key(|&index| displays[index].len());
    let mut layouts: Vec<Option<(Vec<Attribute>, Vec<Reference>)>> =
        vec![None; decls.classes.len()];
    for index in order {
        let def = decls.classes[index];
        let id = ClassId(small(index, "classes")?);
        let (mut attributes, mut references) = match super_of[index] {
            Some(parent) => layouts[parent.index()]
                .clone()
                .expect("parent laid out first"),
            None => (Vec::new(), Vec::new()),
        };
        let declared_in = |name: &str, attributes: &[Attribute], references: &[Reference]| {
            let owner = attributes
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.id.owner)
                .or_else(|| {
                    references
                        .iter()
                        .find(|r| r.name == name)
                        .map(|r| r.id.owner)
                })?;
            Some(decls.classes[owner.index()].name.clone())
        };

        for attr in &def.attributes {
            if let Some(declared_in) = declared_in(&attr.name, &attributes, &references) {
                return Err(duplicate(&attr.name, def, declared_in));
            }
            if attr.upper_bound != 1 {
                return Err(ModelError::MultiValuedAttribute {
                    class: def.name.clone(),
                    attribute: attr.name.clone(),
                });
            }
            let attr_type =
                resolve_type(&attr.type_name, &types, &decls.data_types)?.ok_or_else(|| {
                    ModelError::UnknownType {
                        type_name: attr.type_name.clone(),
                        class: def.name.clone(),
                        feature: attr.name.clone(),
                    }
                })?;
            let position = small(attributes.len(), "attributes")?;
            attributes.push(Attribute {
                name: attr.name.clone(),
                id: AttrId {
                    owner: id,
                    index: position,
                },
                attr_type,
            });
        }

        for reference in &def.references {
            if let Some(declared_in) = declared_in(&reference.name, &attributes, &references) {
                return Err(duplicate(&reference.name, def, declared_in));
            }
            let target = *class_by_name.get(&reference.type_name).ok_or_else(|| {
                ModelError::UnknownType {
                    type_name: reference.type_name.clone(),
                    class: def.name.clone(),
                    feature: reference.name.clone(),
                }
            })?;
            let position = small(references.len(), "references")?;
            references.push(Reference {
                name: reference.name.clone(),
                id: RefId {
                    owner: id,
                    index: position,
                },
                target,
                many: reference.upper_bound != 1,
            });
        }
        layouts[index] = Some((attributes, references));
    }

    // Pass 4: assemble the classes, and record which ones have subclasses.
    let mut classes = Vec::with_capacity(decls.classes.len());
    for (index, (def, layout)) in decls.classes.iter().zip(layouts).enumerate() {
        let (attributes, references) = layout.expect("every class laid out");
        classes.push(Class {
            name: def.name.clone(),
            id: ClassId(small(index, "classes")?),
            is_abstract: def.is_abstract,
            display: displays[index].clone(),
            has_subclasses: false,
            attributes,
            references,
        });
    }
    for parent in super_of.iter().flatten() {
        classes[parent.index()].has_subclasses = true;
    }

    let enums = decls
        .enums
        .iter()
        .map(|e| EnumType {
            name: e.name.clone(),
            literals: e.literals.clone(),
        })
        .collect();
    Ok(Model {
        classes,
        enums,
        class_by_name,
    })
}

/// Chain of ancestors of a class, from the root class down to the class itself.
fn display_of(
    index: usize,
    super_of: &[Option<ClassId>],
    classes: &[&ClassDef],
) -> Result<Vec<ClassId>, ModelError> {
    let mut chain = vec![ClassId(small(index, "classes")?)];
    let mut current = super_of[index];
    while let Some(parent) = current {
        if let Some(start) = chain.iter().position(|&c| c == parent) {
            // Walking up reached a class already met: report the cycle, closed on its start.
            let mut names: Vec<String> = chain[start..]
                .iter()
                .map(|c| classes[c.index()].name.clone())
                .collect();
            names.push(classes[parent.index()].name.clone());
            return Err(ModelError::InheritanceCycle { classes: names });
        }
        chain.push(parent);
        current = super_of[parent.index()];
    }
    chain.reverse();
    Ok(chain)
}

/// Resolves a type name to the way its values are stored. `Ok(None)` if the name is unknown.
fn resolve_type(
    name: &str,
    types: &HashMap<&str, TypeEntry>,
    data_types: &[&DataTypeDef],
) -> Result<Option<AttrType>, ModelError> {
    let mut current = name;
    // Each step goes down one `baseType`: more steps than there are data types means a cycle.
    for _ in 0..=data_types.len() {
        match types.get(current) {
            None => return Ok(None),
            Some(TypeEntry::Base(base)) => return Ok(Some(*base)),
            Some(TypeEntry::Enum(id)) => return Ok(Some(AttrType::Enum(*id))),
            Some(TypeEntry::Data(index)) => current = &data_types[*index].base_type,
        }
    }
    Err(ModelError::DataTypeCycle {
        name: name.to_owned(),
    })
}

fn duplicate(name: &str, class: &ClassDef, declared_in: String) -> ModelError {
    ModelError::DuplicateFeature {
        name: name.to_owned(),
        class: class.name.clone(),
        declared_in,
    }
}

/// Converts a position to the `u16` of identifiers, or fails if the model is too large.
fn small(index: usize, what: &'static str) -> Result<u16, ModelError> {
    u16::try_from(index).map_err(|_| ModelError::TooLarge { what })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(path: &str) -> ModelFile {
        let full = format!("{}/../../examples/{path}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"));
        ModelFile::parse(&text).expect("parse")
    }

    /// Builds a model file from the JSON text of its single package.
    fn package(json: &str) -> ModelFile {
        ModelFile::parse(&format!(
            r#"{{ "formatVersion": "0.1", "packages": [ {json} ] }}"#
        ))
        .expect("parse")
    }

    fn compile_one(json: &str) -> Result<Model, ModelError> {
        Model::compile(&[package(json)])
    }

    fn names(class: &Class) -> Vec<&str> {
        class.attributes().iter().map(Attribute::name).collect()
    }

    /// Scenario "Modèle de l'annuaire".
    #[test]
    fn directory_model() {
        let model = Model::compile(&[example("annuaire/annuaire.model.json")]).expect("compile");
        let person = model.class(model.class_id("Personne").expect("Personne"));
        assert_eq!(names(person), ["nom", "prenom", "date_naissance"]);
        assert_eq!(person.attributes()[2].attr_type(), AttrType::Date);
        let directory = model.class(model.class_id("Annuaire").expect("Annuaire"));
        let persons = model.reference(directory.reference("personnes").expect("personnes"));
        assert!(persons.is_many());
        assert_eq!(persons.target(), person.id());
    }

    /// The metamodel described with itself is read, but does not compile yet: `Enum.literals`
    /// is a multi-valued attribute.
    #[test]
    fn metamodel_model_needs_multi_valued_attributes() {
        let error = Model::compile(&[example("model-editor/metamodel.model.json")])
            .expect_err("multi-valued attribute");
        assert_eq!(
            error,
            ModelError::MultiValuedAttribute {
                class: "Enum".into(),
                attribute: "literals".into()
            }
        );
    }

    /// Scenario "Modèle réparti sur deux fichiers".
    #[test]
    fn model_split_over_two_files() {
        let first = package(
            r#"{ "name": "a", "classes": [ { "name": "Base",
                "attributes": [ { "name": "code", "type": "String" } ] } ] }"#,
        );
        let second = package(
            r#"{ "name": "b", "classes": [ { "name": "Derived", "superClass": "Base",
                "attributes": [ { "name": "size", "type": "Int" } ] } ] }"#,
        );
        let model = Model::compile(&[first, second]).expect("compile");
        let derived = model.class(model.class_id("Derived").expect("Derived"));
        assert_eq!(names(derived), ["code", "size"]);
    }

    /// Scenario "Attribut d'un type de base".
    #[test]
    fn attribute_of_a_base_type() {
        let model = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "Station",
                "attributes": [ { "name": "altitude", "type": "Int" } ] } ] }"#,
        )
        .expect("compile");
        let station = model.class(model.class_id("Station").expect("Station"));
        let altitude = station.attribute("altitude").expect("altitude");
        assert_eq!(model.attribute(altitude).attr_type(), AttrType::Int);
    }

    /// Scenario "Type inconnu".
    #[test]
    fn unknown_type() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "Station",
                "attributes": [ { "name": "altitude", "type": "Entier" } ] } ] }"#,
        )
        .expect_err("unknown type");
        assert_eq!(
            error,
            ModelError::UnknownType {
                type_name: "Entier".into(),
                class: "Station".into(),
                feature: "altitude".into()
            }
        );
        let message = error.to_string();
        assert!(
            message.contains("Entier") && message.contains("altitude"),
            "{message}"
        );
    }

    /// Scenario "Littéraux d'un énuméré".
    #[test]
    fn enum_literals() {
        let model = compile_one(
            r#"{ "name": "p", "enums": [ { "name": "Qualite", "literals": ["BONNE", "DOUTEUSE", "FAUSSE"] } ],
                "classes": [ { "name": "Releve", "attributes": [ { "name": "qualite", "type": "Qualite" } ] } ] }"#,
        )
        .expect("compile");
        let releve = model.class(model.class_id("Releve").expect("Releve"));
        let AttrType::Enum(id) = releve.attributes()[0].attr_type() else {
            panic!("not an enum");
        };
        assert_eq!(
            model.enum_type(id).literals(),
            ["BONNE", "DOUTEUSE", "FAUSSE"]
        );
        assert_eq!(model.enum_type(id).literal("FAUSSE"), Some(2));
    }

    /// Scenario "Type dérivé d'un type de base", through two levels of data types.
    #[test]
    fn derived_type_is_stored_as_its_base_type() {
        let model = compile_one(
            r#"{ "name": "p",
                "dataTypes": [ { "name": "Identifier", "baseType": "String" },
                               { "name": "Code", "baseType": "Identifier" } ],
                "classes": [ { "name": "A", "attributes": [
                    { "name": "id", "type": "Identifier" }, { "name": "code", "type": "Code" } ] } ] }"#,
        )
        .expect("compile");
        let a = model.class(model.class_id("A").expect("A"));
        assert_eq!(a.attributes()[0].attr_type(), AttrType::String);
        assert_eq!(a.attributes()[1].attr_type(), AttrType::String);
    }

    #[test]
    fn data_type_cycle() {
        let error = compile_one(
            r#"{ "name": "p",
                "dataTypes": [ { "name": "X", "baseType": "Y" }, { "name": "Y", "baseType": "X" } ],
                "classes": [ { "name": "A", "attributes": [ { "name": "x", "type": "X" } ] } ] }"#,
        )
        .expect_err("cycle");
        assert_eq!(error, ModelError::DataTypeCycle { name: "X".into() });
    }

    const EQUIPMENTS: &str = r#"{ "name": "p", "classes": [
        { "name": "Equipement", "abstract": true, "attributes": [
            { "name": "numero_serie", "type": "String" }, { "name": "modele", "type": "String" } ] },
        { "name": "Thermometre", "superClass": "Capteur", "attributes": [
            { "name": "plage_min", "type": "Float" } ] },
        { "name": "Capteur", "superClass": "Equipement", "abstract": true,
          "attributes": [ { "name": "frequence_s", "type": "Int" } ],
          "references": [ { "name": "mesures", "type": "Mesure", "upperBound": -1 } ] },
        { "name": "Camera", "superClass": "Equipement", "attributes": [
            { "name": "resolution", "type": "String" } ] },
        { "name": "Mesure", "attributes": [ { "name": "instant", "type": "DateTime" } ] },
        { "name": "Releve", "superClass": "Mesure" } ] }"#;

    /// Scenario "Héritage sur trois niveaux" (the subclass is declared before its super class).
    #[test]
    fn inheritance_over_three_levels() {
        let model = compile_one(EQUIPMENTS).expect("compile");
        let thermometer = model.class(model.class_id("Thermometre").expect("Thermometre"));
        assert_eq!(
            names(thermometer),
            ["numero_serie", "modele", "frequence_s", "plage_min"]
        );
        let mesures = thermometer.reference("mesures").expect("mesures");
        assert_eq!(mesures.owner(), model.class_id("Capteur").expect("Capteur"));
        let ancestors: Vec<&str> = thermometer
            .ancestors()
            .map(|c| model.class(c).name())
            .collect();
        assert_eq!(ancestors, ["Capteur", "Equipement"]);
    }

    #[test]
    fn subclass_test_and_subclass_presence() {
        let model = compile_one(EQUIPMENTS).expect("compile");
        let id = |name| model.class_id(name).expect(name);
        assert!(model.is_subclass_of(id("Thermometre"), id("Equipement")));
        assert!(model.is_subclass_of(id("Thermometre"), id("Thermometre")));
        assert!(!model.is_subclass_of(id("Camera"), id("Capteur")));
        assert!(!model.is_subclass_of(id("Equipement"), id("Capteur")));
        assert!(!model.is_subclass_of(id("Releve"), id("Equipement")));
        assert!(model.class(id("Equipement")).has_subclasses());
        assert!(!model.class(id("Camera")).has_subclasses());
    }

    /// Scenario "Super-classe introuvable".
    #[test]
    fn unknown_super_class() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "Camera", "superClass": "Equipment" } ] }"#,
        )
        .expect_err("unknown super class");
        assert!(error.to_string().contains("Equipment"), "{error}");
    }

    /// Scenario "Cycle d'héritage".
    #[test]
    fn inheritance_cycle() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "A", "superClass": "B" },
                                            { "name": "B", "superClass": "A" } ] }"#,
        )
        .expect_err("cycle");
        assert_eq!(
            error,
            ModelError::InheritanceCycle {
                classes: vec!["A".into(), "B".into(), "A".into()]
            }
        );
        assert_eq!(error.to_string(), "inheritance cycle: A -> B -> A");
    }

    /// Scenario "Attribut hérité dans deux sous-classes".
    #[test]
    fn inherited_attribute_has_the_same_id_in_two_subclasses() {
        let model = compile_one(EQUIPMENTS).expect("compile");
        let id = |name| model.class(model.class_id(name).expect(name));
        assert_eq!(
            id("Thermometre").attribute("numero_serie"),
            id("Camera").attribute("numero_serie")
        );
    }

    /// Scenarios "Attribut hérité trouvé depuis la sous-classe" and "Nom inconnu".
    #[test]
    fn name_lookup() {
        let model = compile_one(EQUIPMENTS).expect("compile");
        let releve = model.class(model.class_id("Releve").expect("Releve"));
        let mesure = model.class(model.class_id("Mesure").expect("Mesure"));
        assert_eq!(releve.attribute("instant"), mesure.attribute("instant"));
        assert!(releve.attribute("instant").is_some());
        assert_eq!(releve.attribute("temperature"), None);
        assert_eq!(model.class_id("Temperature"), None);
    }

    /// Scenario "Classe en double".
    #[test]
    fn duplicate_class() {
        let first = package(r#"{ "name": "a", "classes": [ { "name": "Point" } ] }"#);
        let second = package(r#"{ "name": "b", "classes": [ { "name": "Point" } ] }"#);
        let error = Model::compile(&[first, second]).expect_err("duplicate");
        assert_eq!(
            error,
            ModelError::DuplicateClass {
                name: "Point".into()
            }
        );
        assert!(error.to_string().contains("Point"));
    }

    /// Scenario "Attribut qui redéclare un attribut hérité".
    #[test]
    fn redeclared_inherited_attribute() {
        let json = EQUIPMENTS.replace(
            r#"{ "name": "plage_min", "type": "Float" }"#,
            r#"{ "name": "plage_min", "type": "Float" }, { "name": "modele", "type": "String" }"#,
        );
        let error = compile_one(&json).expect_err("redeclared");
        assert_eq!(
            error,
            ModelError::DuplicateFeature {
                name: "modele".into(),
                class: "Thermometre".into(),
                declared_in: "Equipement".into()
            }
        );
        let message = error.to_string();
        assert!(message.contains("modele") && message.contains("Thermometre"));
        assert!(message.contains("Equipement"));
    }

    #[test]
    fn reference_with_the_name_of_an_attribute() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "A",
                "attributes": [ { "name": "x", "type": "Int" } ],
                "references": [ { "name": "x", "type": "A" } ] } ] }"#,
        )
        .expect_err("duplicate");
        assert!(
            matches!(error, ModelError::DuplicateFeature { .. }),
            "{error}"
        );
    }

    /// Scenario "Attribut multivalué".
    #[test]
    fn multi_valued_attribute() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "Station", "attributes": [
                { "name": "mots_cles", "type": "String", "upperBound": -1 } ] } ] }"#,
        )
        .expect_err("multi-valued");
        let message = error.to_string();
        assert!(
            message.contains("mots_cles") && message.contains("not supported"),
            "{message}"
        );
    }

    /// The test model of the weather network (task 1.4).
    #[test]
    fn weather_network_model() {
        let model = Model::compile(&[example("reseau-meteo/reseau.model.json")]).expect("compile");
        let class = |name| model.class(model.class_id(name).expect(name));
        let ancestors: Vec<&str> = class("Thermometre")
            .ancestors()
            .map(|c| model.class(c).name())
            .collect();
        assert_eq!(ancestors, ["Capteur", "Equipement"]);
        assert_eq!(
            class("Thermometre").attribute("numero_serie"),
            class("Camera").attribute("numero_serie")
        );
        // `Code` is a data type derived from String.
        assert_eq!(
            class("Station").attributes()[0].attr_type(),
            AttrType::String
        );
        // `annotations` is declared in `Mesure`, so `Lacune` has it too.
        assert!(class("Lacune").reference("annotations").is_some());
        let emprise = model.reference(class("ZoneAlerte").reference("emprise").expect("emprise"));
        assert!(!emprise.is_many());
        assert!(model.class(emprise.target()).is_abstract());
    }

    #[test]
    fn unknown_reference_type() {
        let error = compile_one(
            r#"{ "name": "p", "classes": [ { "name": "A",
                "references": [ { "name": "items", "type": "Item", "upperBound": -1 } ] } ] }"#,
        )
        .expect_err("unknown type");
        assert!(error.to_string().contains("Item"));
    }
}
