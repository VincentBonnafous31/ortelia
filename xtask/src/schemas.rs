//! `cargo xtask gen-schemas [--check]`: the JSON Schemas of the configuration files written by
//! hand (`app.json`, `*.model.json`), generated from the types that read them, so that they
//! always describe what Ortelia accepts.
//!
//! schemars derives a schema from the serde definitions (`deny_unknown_fields` gives
//! `additionalProperties: false`, and the renames and defaults are taken into account). Draft 7,
//! the version that text editors recognize best. The schemas are written as indented JSON with a
//! final line break: the same code always gives the same bytes.

use std::io;
use std::path::{Path, PathBuf};

use ortelia_config::AppFile;
use ortelia_model::metamodel::ModelFile;
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;

/// The schemas, relative to the root of the repository, with their text.
pub fn generate() -> Vec<(&'static str, String)> {
    vec![
        ("schemas/app.schema.json", schema_text::<AppFile>()),
        ("schemas/model.schema.json", schema_text::<ModelFile>()),
    ]
}

fn schema_text<T: JsonSchema>() -> String {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<T>();
    let mut text = serde_json::to_string_pretty(&schema).expect("a schema is plain JSON");
    text.push('\n');
    text
}

/// The root of the repository: the parent of the `xtask` folder.
pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is in the repository")
        .to_owned()
}

/// Writes the schemas under `root`, and returns their paths.
pub fn write(root: &Path) -> io::Result<Vec<&'static str>> {
    let schemas = generate();
    std::fs::create_dir_all(root.join("schemas"))?;
    for (path, text) in &schemas {
        std::fs::write(root.join(path), text)?;
    }
    Ok(schemas.into_iter().map(|(path, _)| path).collect())
}

/// The schemas under `root` that are missing or differ from what the code generates.
pub fn stale(root: &Path) -> Vec<&'static str> {
    generate()
        .into_iter()
        .filter(|(path, text)| std::fs::read_to_string(root.join(path)).ok().as_ref() != Some(text))
        .map(|(path, _)| path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn schema(name: &str) -> Value {
        let text = generate()
            .into_iter()
            .find(|(path, _)| path.ends_with(name))
            .expect("schema")
            .1;
        serde_json::from_str(&text).expect("JSON")
    }

    /// The names of the properties of an object schema.
    fn keys(object: &Value) -> Vec<&str> {
        object["properties"]
            .as_object()
            .expect("properties")
            .keys()
            .map(String::as_str)
            .collect()
    }

    /// Scenario "Génération des deux schémas".
    #[test]
    fn both_schemas_are_written_and_refuse_unknown_keys() {
        let dir = tempfile::tempdir().expect("temporary folder");
        let written = write(dir.path()).expect("write");
        assert_eq!(
            written,
            ["schemas/app.schema.json", "schemas/model.schema.json"]
        );
        for path in written {
            let text = std::fs::read_to_string(dir.path().join(path)).expect("read");
            let schema: Value = serde_json::from_str(&text).expect("JSON");
            assert_eq!(
                schema["$schema"], "http://json-schema.org/draft-07/schema#",
                "{path}"
            );
            assert_eq!(schema["type"], "object", "{path}");
            assert_eq!(schema["additionalProperties"], false, "{path}");
        }
    }

    /// Scenario "Clés du schéma de `app.json`".
    #[test]
    fn keys_of_the_app_schema() {
        let app = schema("app.schema.json");
        assert_eq!(
            keys(&app),
            [
                "$schema",
                "formatVersion",
                "id",
                "name",
                "languages",
                "labels",
                "model",
                "sources",
                "presentations",
                "window"
            ]
        );
        // Draft 7 puts the definitions under `definitions`.
        let source = &app["definitions"]["SourceDef"];
        assert_eq!(keys(source), ["id", "format", "file", "target"]);
        assert_eq!(
            source["required"],
            serde_json::json!(["id", "format", "file", "target"])
        );
        assert_eq!(source["additionalProperties"], false);
        assert_eq!(
            app["definitions"]["SourceFormat"]["enum"],
            serde_json::json!(["json"])
        );
    }

    /// Scenario "Contenu polymorphe de la fenêtre": each variant is an object whose `_type_` is
    /// a constant, without other keys than its own.
    #[test]
    fn polymorphic_content_of_the_window() {
        let app = schema("app.schema.json");
        let definitions = &app["definitions"];
        assert_eq!(
            definitions["WindowDef"]["properties"]["content"]["$ref"],
            "#/definitions/Content"
        );
        let variants = |name: &str| definitions[name]["oneOf"].as_array().expect(name).clone();
        let [view] = &variants("Content")[..] else {
            panic!("one variant of Content")
        };
        assert_eq!(view["properties"]["_type_"]["const"], "View");
        assert_eq!(keys(view), ["_type_", "id", "widget", "data"]);
        assert_eq!(view["additionalProperties"], false);
        assert_eq!(view["properties"]["widget"]["$ref"], "#/definitions/Widget");
        let [table] = &variants("Widget")[..] else {
            panic!("one variant of Widget")
        };
        assert_eq!(table["properties"]["_type_"]["const"], "Table");
        assert_eq!(table["additionalProperties"], false);
        assert_eq!(table["required"], serde_json::json!(["_type_"]));
    }

    /// The model schema follows the keys of the model file.
    #[test]
    fn keys_of_the_model_schema() {
        let model = schema("model.schema.json");
        assert_eq!(keys(&model), ["$schema", "formatVersion", "packages"]);
        assert_eq!(model["required"], serde_json::json!(["formatVersion"]));
    }

    /// Scenario "Génération reproductible".
    #[test]
    fn generation_is_reproducible() {
        let first = tempfile::tempdir().expect("temporary folder");
        let second = tempfile::tempdir().expect("temporary folder");
        write(first.path()).expect("write");
        write(second.path()).expect("write");
        for (path, _) in generate() {
            let a = std::fs::read(first.path().join(path)).expect("read");
            let b = std::fs::read(second.path().join(path)).expect("read");
            assert_eq!(a, b, "{path}");
            assert!(a.ends_with(b"}\n"), "{path}");
        }
    }

    /// Scenarios "Schéma périmé" and "Schémas à jour": missing, then up to date, then changed.
    #[test]
    fn check_of_the_schemas() {
        let dir = tempfile::tempdir().expect("temporary folder");
        assert_eq!(
            stale(dir.path()),
            ["schemas/app.schema.json", "schemas/model.schema.json"]
        );
        write(dir.path()).expect("write");
        assert!(stale(dir.path()).is_empty());
        let app = dir.path().join("schemas/app.schema.json");
        let text = std::fs::read_to_string(&app).expect("read");
        std::fs::write(&app, text.replace("\"formatVersion\"", "\"version\"")).expect("write");
        assert_eq!(stale(dir.path()), ["schemas/app.schema.json"]);
    }

    /// The schemas of the repository are those the code generates.
    #[test]
    fn repository_schemas_are_up_to_date() {
        assert!(
            stale(&repository_root()).is_empty(),
            "run `cargo xtask gen-schemas`"
        );
    }

    /// Scenario "Exemple de l'annuaire", and the other example files: their `$schema` key names
    /// the schema of their kind, which exists.
    #[test]
    fn examples_name_their_schema() {
        let root = repository_root();
        for (file, schema) in [
            ("examples/annuaire/app.json", "app.schema.json"),
            ("examples/annuaire/annuaire.model.json", "model.schema.json"),
            (
                "examples/reseau-meteo/reseau.model.json",
                "model.schema.json",
            ),
            (
                "examples/model-editor/metamodel.model.json",
                "model.schema.json",
            ),
        ] {
            let path = root.join(file);
            let text = std::fs::read_to_string(&path).expect("example");
            let json: Value = serde_json::from_str(&text).expect("JSON");
            let declared = json["$schema"].as_str().unwrap_or_else(|| panic!("{file}"));
            assert_eq!(declared, format!("../../schemas/{schema}"), "{file}");
            let target = path.parent().expect("folder").join(declared);
            assert!(target.is_file(), "{file}: {}", target.display());
        }
    }
}
