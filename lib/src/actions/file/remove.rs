use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actions::Action;

use super::FileAction;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRemove {
    pub target: Option<String>,
}

impl FileRemove {}

impl FileAction for FileRemove {}

impl Action for FileRemove {
    fn plan(
        &self,
        manifest: &crate::manifests::Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        dbg!(&self.target);

        todo!()
    }
}
