//! The model file format (`*.model.json`, format 0.1), as read by serde.
//!
//! These structs mirror the file one to one; `compile.rs` turns them into a [`Model`].
//! `deny_unknown_fields` rejects a misspelt key (serde_json's error then names the key, with its
//! line and column), and `rename_all = "camelCase"` maps `lowerBound` to `lower_bound`.
//!
//! Some keys are read but not used yet (`defaultValue`, `unit`, `identifier`, `refersTo`,
//! `constraints`, and the restrictions of data types): a file valid today stays valid when they
//! start to be used.
//!
//! [`Model`]: super::Model

use serde::Deserialize;

/// A whole model file.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModelFile {
    /// The JSON Schema of the file, for text editors: read and ignored.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    pub format_version: String,
    #[serde(default)]
    pub packages: Vec<PackageDef>,
}

impl ModelFile {
    /// Parses the text of a model file.
    pub fn parse(text: &str) -> Result<ModelFile, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// A package: classes, enums, data types, and nested packages.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackageDef {
    pub name: String,
    #[serde(default)]
    pub packages: Vec<PackageDef>,
    #[serde(default)]
    pub classes: Vec<ClassDef>,
    #[serde(default)]
    pub enums: Vec<EnumDef>,
    #[serde(default)]
    pub data_types: Vec<DataTypeDef>,
}

/// A class, with its own attributes and containment references (not the inherited ones).
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClassDef {
    pub name: String,
    #[serde(default)]
    pub super_class: Option<String>,
    #[serde(default, rename = "abstract")]
    pub is_abstract: bool,
    #[serde(default)]
    pub attributes: Vec<AttributeDef>,
    #[serde(default)]
    pub references: Vec<ReferenceDef>,
    /// Not used yet.
    #[serde(default)]
    pub constraints: Vec<ConstraintDef>,
}

/// An attribute: a value of a base type, an enum or a data type.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AttributeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub lower_bound: i32,
    #[serde(default = "one")]
    pub upper_bound: i32,
    /// Not used yet.
    #[serde(default)]
    pub default_value: Option<String>,
    /// Not used yet.
    #[serde(default)]
    pub unit: Option<String>,
    /// Not used yet.
    #[serde(default)]
    pub identifier: bool,
    /// Not used yet.
    #[serde(default)]
    pub refers_to: Option<String>,
}

/// A containment reference: the children belong to the object that holds the reference.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReferenceDef {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub lower_bound: i32,
    /// 1 for a single reference, -1 (unbounded) or more than 1 for a list.
    #[serde(default = "one")]
    pub upper_bound: i32,
}

/// An enum: an ordered list of literals.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EnumDef {
    pub name: String,
    /// Not used.
    // An enum value is stored as the index of its literal: no base type is needed.
    #[serde(default)]
    pub base_type: Option<String>,
    pub literals: Vec<String>,
}

/// A data type derived from another type (`baseType`), with restrictions not used yet.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DataTypeDef {
    pub name: String,
    pub base_type: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub min_length: Option<i64>,
    #[serde(default)]
    pub max_length: Option<i64>,
}

/// A constraint on a class. Not used yet.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstraintDef {
    pub name: String,
    #[serde(default)]
    pub severity: Option<String>,
    pub expression: String,
    #[serde(default)]
    pub message: Option<String>,
}

/// Default upper bound of attributes and references: a single value.
fn one() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(path: &str) -> String {
        let full = format!("{}/../../examples/{path}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
    }

    #[test]
    fn reads_the_directory_model() {
        let file = ModelFile::parse(&example("annuaire/annuaire.model.json")).expect("parse");
        let package = &file.packages[0];
        assert_eq!(package.classes.len(), 3);
        // `Racine.annuaire` is a single reference, `Annuaire.personnes` a list.
        assert_eq!(package.classes[0].references[0].upper_bound, 1);
        assert_eq!(package.classes[1].references[0].upper_bound, -1);
        assert_eq!(package.classes[2].attributes[4].unit.as_deref(), Some("m"));
        assert_eq!(package.enums[0].literals.len(), 5);
    }

    #[test]
    fn reads_the_metamodel_model() {
        let file = ModelFile::parse(&example("model-editor/metamodel.model.json")).expect("parse");
        let package = &file.packages[0];
        assert_eq!(package.enums[0].literals, ["ERROR", "WARNING", "INFO"]);
        assert_eq!(package.data_types[0].base_type, "String");
    }

    /// Scenario "Clé inconnue dans un fichier de modèle".
    #[test]
    fn misspelt_key_is_rejected_and_named() {
        let text = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p", "classes": [
            { "name": "A", "atributes": [] } ] } ] }"#;
        let error = ModelFile::parse(text).expect_err("unknown key");
        assert!(error.to_string().contains("atributes"), "{error}");
    }
}
