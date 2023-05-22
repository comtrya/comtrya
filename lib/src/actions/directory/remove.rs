use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actions::Action;

use super::DirectoryAction;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryRemove {
    pub target: String,
}

impl DirectoryRemove {}

impl DirectoryAction for DirectoryRemove {}

#[cfg(target_family = "unix")]
impl Action for DirectoryRemove {
    fn plan(
        &self,
        manifest: &crate::manifests::Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        todo!()
    }
}
