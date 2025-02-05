use super::providers::UserProviders;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

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
    fn summarize(&self) -> String {
        format!(
            "Adding user {} to group(s) {}",
            self.username,
            self.group.join(",")
        )
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let box_provider = self.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        atoms.append(&mut provider.add_to_group(self, context)?);

        Ok(atoms)
    }
}
