//! `file.remove`

use crate::actions::Result;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{actions::Action, steps::Step};

use super::FileAction;

/// Removes a file
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRemove {
    /// The file to remove
    pub target: String,
    /// Make the change as root, in the privileged helper that the privilege provider (like
    /// sudo) starts once per run, unless comtrya runs as root
    #[serde(
        default,
        alias = "sudo",
        skip_serializing_if = "crate::actions::is_false"
    )]
    #[schemars(extend("x-aliases" = ["sudo"]))]
    pub privileged: bool,
}

impl FileRemove {}

impl FileAction for FileRemove {}

impl Action for FileRemove {
    fn privileged(&self) -> bool {
        self.privileged
    }

    fn summarize(&self) -> String {
        format!("Removing file {}", self.target)
    }

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::actions::PlanCx,
    ) -> Result<Vec<crate::steps::Step>> {
        use crate::atoms::file::Remove as RemoveFile;

        let path = PathBuf::from(&self.target);

        let steps = vec![Step::new(RemoveFile { target: path })];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.remove
  target: a
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileRemove(action)) => {
                assert_eq!("a", action.action.target);
            }
            _ => {
                panic!("FileRemove didn't deserialize to the correct type");
            }
        };
    }
}
