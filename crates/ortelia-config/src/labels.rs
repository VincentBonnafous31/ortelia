//! The labels of `app.json`: checked against the model, then gathered in a lookup table.
//!
//! A key names a class (`Personne`), an attribute of a class (`Personne.nom`, inherited
//! attributes included) or a literal of an enum (`SituationFamiliale.MARIE`); a key cited by a
//! form (title of a group, fixed text, label of a field) may be free (`Personne.identite`).
//! The lookup itself,
//! with the inheritance and the `#key#` of a missing label, belongs to the presentation layer.

use std::collections::{HashMap, HashSet};

use ortelia_model::metamodel::Model;

use crate::app::{ConfigError, ConfigWarning, LabelGroup};

/// The labels of an application, by language then by key.
//
// A `HashMap` in memory: the rule "no dictionaries" is about the file formats, which are lists
// of pairs; a lookup table inside the program is another matter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    by_lang: HashMap<String, HashMap<String, String>>,
}

impl Labels {
    /// The text of `key` in the language `lang`, if it has one.
    pub fn text(&self, lang: &str, key: &str) -> Option<&str> {
        self.by_lang.get(lang)?.get(key).map(String::as_str)
    }
}

/// Checks the groups of labels (errors: undeclared language, two groups of one language, a key
/// twice in a group; warning: a key that names nothing and that no form cites), and builds the
/// lookup table. `cited` are the keys cited by the forms, which may be free.
pub(crate) fn check_labels(
    groups: &[LabelGroup],
    languages: &[String],
    model: &Model,
    cited: &HashSet<&str>,
    errors: &mut Vec<ConfigError>,
    warnings: &mut Vec<ConfigWarning>,
) -> Labels {
    let mut labels = Labels::default();
    for (index, group) in groups.iter().enumerate() {
        if !languages.contains(&group.lang) {
            errors.push(ConfigError::UndeclaredLabelLanguage {
                lang: group.lang.clone(),
            });
        }
        // Reported once, at the second group of the language.
        let earlier = groups[..index]
            .iter()
            .filter(|g| g.lang == group.lang)
            .count();
        if earlier >= 1 {
            if earlier == 1 {
                errors.push(ConfigError::DuplicateLabelGroup {
                    lang: group.lang.clone(),
                });
            }
            // Its keys are not checked: they would all be reported again as duplicates.
            continue;
        }
        let table = labels.by_lang.entry(group.lang.clone()).or_default();
        for entry in &group.entries {
            if !names_something(model, &entry.key) && !cited.contains(&entry.key.as_str()) {
                warnings.push(ConfigWarning::UnknownLabelKey {
                    lang: group.lang.clone(),
                    key: entry.key.clone(),
                });
            }
            // `insert` returns the previous value: a key already there is a duplicate.
            if table
                .insert(entry.key.clone(), entry.text.clone())
                .is_some()
            {
                errors.push(ConfigError::DuplicateLabel {
                    lang: group.lang.clone(),
                    key: entry.key.clone(),
                });
            }
        }
    }
    labels
}

/// `true` if `key` names a class, an attribute of a class or a literal of an enum.
fn names_something(model: &Model, key: &str) -> bool {
    match key.split_once('.') {
        None => model.class_id(key).is_some(),
        Some((owner, member)) => {
            let attribute = model
                .class_id(owner)
                .is_some_and(|class| model.class(class).attribute(member).is_some());
            let literal = model
                .enum_id(owner)
                .is_some_and(|e| model.enum_type(e).literal(member).is_some());
            attribute || literal
        }
    }
}
