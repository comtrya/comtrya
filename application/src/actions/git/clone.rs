use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GitClone {
    pub repository: String,
    pub reference: Option<String>,
    pub directory: String,
}

impl Action for GitClone {
    fn plan(&self, _: &Manifest, _: &Contexts) -> Vec<Step> {
        vec![Step {
            atom: Box::new(crate::atoms::git::Clone {
                repository: self.repository.clone(),
                reference: self.reference.clone(),
                directory: PathBuf::from(self.directory.clone()),
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
