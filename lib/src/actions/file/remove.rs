use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{actions::Action, steps::Step};

use super::FileAction;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRemove {
    pub target: String,
}

impl FileRemove {}

impl FileAction for FileRemove {}

impl Action for FileRemove {
    fn plan(
        &self,
        manifest: &crate::manifests::Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        use crate::atoms::file::Remove as RemoveFile;

        let path = PathBuf::from(&self.target);
        dbg!(&manifest.root_dir);

        let mut steps = vec![Step {
            atom: Box::new(RemoveFile { target: path }),
            initializers: vec![],
            finalizers: vec![],
        }];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {}
