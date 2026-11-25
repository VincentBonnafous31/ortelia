//! Helpers shared by the unit tests of the crate: the example models, and names resolved once.

use std::sync::Arc;

use crate::metamodel::{AttrId, ClassId, Model, ModelFile, RefId};

/// The compiled test model of the weather network.
pub fn network_model() -> Arc<Model> {
    example_model("reseau-meteo/reseau.model.json")
}

/// The compiled model of the directory application.
pub fn directory_model() -> Arc<Model> {
    example_model("annuaire/annuaire.model.json")
}

fn example_model(path: &str) -> Arc<Model> {
    let path = format!("{}/../../examples/{path}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    Arc::new(Model::compile(&[ModelFile::parse(&text).expect("parse")]).expect("compile"))
}

/// Names resolved once, as the code that uses a store does.
#[derive(Clone)]
pub struct Names {
    pub model: Arc<Model>,
}

impl Names {
    pub fn new(model: &Arc<Model>) -> Names {
        Names {
            model: model.clone(),
        }
    }

    pub fn class(&self, name: &str) -> ClassId {
        self.model
            .class_id(name)
            .unwrap_or_else(|| panic!("class {name}"))
    }

    pub fn attr(&self, class: &str, name: &str) -> AttrId {
        self.model
            .class(self.class(class))
            .attribute(name)
            .unwrap_or_else(|| panic!("{class}.{name}"))
    }

    pub fn reference(&self, class: &str, name: &str) -> RefId {
        self.model
            .class(self.class(class))
            .reference(name)
            .unwrap_or_else(|| panic!("{class}.{name}"))
    }
}
