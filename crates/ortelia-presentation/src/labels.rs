//! The lookup of the labels in the current language, with the inheritance of the attributes and
//! the `#key#` form of a missing label.
//!
//! The labels shown by a table (headers, enum literals) are looked up once, when the table is
//! created: the missing keys are gathered there, once each, and logged at the `WARN` level.
//! They are names of the configuration, not data of the user.

use ortelia_config::Labels;
use ortelia_model::metamodel::{AttrId, ClassId, EnumId, Model};
use tracing::warn;

/// Looks labels up in one language, and gathers the keys it did not find.
#[derive(Debug)]
pub struct LabelLookup<'a> {
    labels: &'a Labels,
    model: &'a Model,
    lang: &'a str,
    missing: Vec<String>,
}

impl<'a> LabelLookup<'a> {
    pub fn new(labels: &'a Labels, model: &'a Model, lang: &'a str) -> LabelLookup<'a> {
        LabelLookup {
            labels,
            model,
            lang,
            missing: Vec::new(),
        }
    }

    /// The label of an attribute shown with `class`: under the key of `class`, then of its
    /// super classes, up to the class that declares the attribute. Missing, `#Class.attr#`, with
    /// the key of `class`.
    pub fn attribute(&mut self, class: ClassId, attr: AttrId) -> String {
        let name = self.model.attribute(attr).name();
        let shown = self.model.class(class);
        // The class itself, then its ancestors, nearest first, until the declaring class.
        let owners = std::iter::once(class).chain(shown.ancestors());
        for owner in owners {
            let key = format!("{}.{name}", self.model.class(owner).name());
            if let Some(text) = self.labels.text(self.lang, &key) {
                return text.to_owned();
            }
            if owner == attr.owner() {
                break;
            }
        }
        self.missing_label(format!("{}.{name}", shown.name()))
    }

    /// The label of the literal `index` of an enum, or `#Enum.LITERAL#`.
    pub fn literal(&mut self, enum_id: EnumId, index: usize) -> String {
        let enum_type = self.model.enum_type(enum_id);
        let key = format!("{}.{}", enum_type.name(), enum_type.literals()[index]);
        match self.labels.text(self.lang, &key) {
            Some(text) => text.to_owned(),
            None => self.missing_label(key),
        }
    }

    /// The keys not found, in the order of the lookups, each once.
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// The language of the lookups.
    pub fn lang(&self) -> &str {
        self.lang
    }

    /// Records a missing key, the first time, and gives its `#key#` form.
    fn missing_label(&mut self, key: String) -> String {
        let shown = format!("#{key}#");
        if !self.missing.contains(&key) {
            warn!(key = %key, lang = %self.lang, "missing label");
            self.missing.push(key);
        }
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ortelia_config::AppFile;
    use ortelia_model::metamodel::ModelFile;

    /// `Employe` inherits from `Personne`.
    const MODEL: &str = r#"{ "formatVersion": "0.1", "packages": [ { "name": "p",
        "classes": [
          { "name": "Racine", "references": [
              { "name": "employes", "type": "Employe", "upperBound": -1 } ] },
          { "name": "Personne", "attributes": [
              { "name": "nom", "type": "String" }, { "name": "prenom", "type": "String" } ] },
          { "name": "Employe", "superClass": "Personne", "attributes": [
              { "name": "situation", "type": "Situation" } ] } ],
        "enums": [ { "name": "Situation", "literals": ["MARIE", "VEUF"] } ] } ] }"#;

    fn model() -> Model {
        Model::compile(&[ModelFile::parse(MODEL).expect("parse")]).expect("compile")
    }

    /// The labels of an `app.json` with the given groups.
    fn labels(model: &Model, groups: &str) -> Labels {
        let text = format!(
            r#"{{ "formatVersion": "0.5", "id": "a", "name": "A", "languages": ["fr", "en"],
                 "labels": {groups},
                 "model": {{ "files": [], "root": "Racine" }},
                 "window": {{ "content": {{ "_type_": "View", "id": "v",
                     "widget": {{ "_type_": "Table" }}, "data": "/employes" }} }} }}"#
        );
        AppFile::parse(&text)
            .expect("parse")
            .check(model)
            .expect("check")
            .labels
    }

    fn ids(model: &Model) -> (ClassId, ClassId, AttrId) {
        let personne = model.class_id("Personne").expect("Personne");
        let employe = model.class_id("Employe").expect("Employe");
        let nom = model.class(personne).attribute("nom").expect("nom");
        (personne, employe, nom)
    }

    /// Scenario "Sous-classe qui renomme".
    #[test]
    fn subclass_that_renames() {
        let model = model();
        let labels = labels(
            &model,
            r#"[ { "lang": "fr", "entries": [ { "key": "Personne.nom", "text": "Nom" },
                   { "key": "Employe.nom", "text": "Nom d'usage" } ] } ]"#,
        );
        let (personne, employe, nom) = ids(&model);
        let mut lookup = LabelLookup::new(&labels, &model, "fr");
        assert_eq!(lookup.attribute(employe, nom), "Nom d'usage");
        assert_eq!(lookup.attribute(personne, nom), "Nom");
        assert!(lookup.missing().is_empty());
    }

    /// Scenario "Libellé de la classe mère".
    #[test]
    fn label_of_the_super_class() {
        let model = model();
        let labels = labels(
            &model,
            r#"[ { "lang": "fr", "entries": [ { "key": "Personne.nom", "text": "Nom" } ] } ]"#,
        );
        let (_, employe, nom) = ids(&model);
        assert_eq!(
            LabelLookup::new(&labels, &model, "fr").attribute(employe, nom),
            "Nom"
        );
    }

    /// Scenarios "Aucun libellé" and "Libellé dans l'autre langue seulement": the key of the
    /// class shown, between `#`, even when another language has a label.
    #[test]
    fn missing_label() {
        // Reaches `warn!("missing label")`, which another test captures: see `lock_log_tests`.
        let _lock = crate::test_support::lock_log_tests();
        let model = model();
        let labels = labels(
            &model,
            r#"[ { "lang": "en", "entries": [ { "key": "Personne.prenom", "text": "First" } ] } ]"#,
        );
        let (personne, employe, _) = ids(&model);
        let prenom = model.class(personne).attribute("prenom").expect("prenom");
        let mut lookup = LabelLookup::new(&labels, &model, "fr");
        assert_eq!(lookup.attribute(personne, prenom), "#Personne.prenom#");
        assert_eq!(lookup.attribute(employe, prenom), "#Employe.prenom#");
        assert_eq!(lookup.missing(), ["Personne.prenom", "Employe.prenom"]);
        assert_eq!(
            LabelLookup::new(&labels, &model, "en").attribute(personne, prenom),
            "First"
        );
    }

    /// The literals of an enum, found or missing; each missing key is recorded once.
    #[test]
    fn literals() {
        let _lock = crate::test_support::lock_log_tests();
        let model = model();
        let labels = labels(
            &model,
            r#"[ { "lang": "fr", "entries": [ { "key": "Situation.MARIE", "text": "Marié(e)" } ] } ]"#,
        );
        let employe = model.class_id("Employe").expect("Employe");
        let situation = model.class(employe).attribute("situation").expect("attr");
        let ortelia_model::metamodel::AttrType::Enum(id) = model.attribute(situation).attr_type()
        else {
            panic!("enum")
        };
        let mut lookup = LabelLookup::new(&labels, &model, "fr");
        assert_eq!(lookup.literal(id, 0), "Marié(e)");
        assert_eq!(lookup.literal(id, 1), "#Situation.VEUF#");
        assert_eq!(lookup.literal(id, 1), "#Situation.VEUF#");
        assert_eq!(lookup.missing(), ["Situation.VEUF"]);
    }
}
