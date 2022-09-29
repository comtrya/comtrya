use super::providers::UserProviders;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// pub type UserAddGroup = User;
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserAddGroup {
    #[serde(default)]
    pub username: String,

    #[serde(default)]
    pub group: Vec<String>,

    #[serde(default)]
    pub provider: UserProviders,
}

impl Action for UserAddGroup {
    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let box_provider = self.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        atoms.append(&mut provider.add_to_group(&self));

        Ok(atoms)
    }
}
