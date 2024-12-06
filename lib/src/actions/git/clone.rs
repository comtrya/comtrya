use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitClone {
    pub repo_url: String,
    pub directory: String,
}

impl Action for GitClone {
    fn summarize(&self) -> String {
        format!("Cloning repository {} to {}", self.repo_url, self.directory)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let url = gix::url::parse(self.repo_url.as_str().into())?;
        Ok(vec![Step {
            atom: Box::new(crate::atoms::git::Clone {
                repository: url.clone(),
                directory: PathBuf::from(self.directory.clone()),
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
