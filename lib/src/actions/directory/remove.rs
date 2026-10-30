//! `directory.remove`

use crate::actions::Result;
use std::path::PathBuf;

use crate::atoms::directory::Remove as RemoveDirAtom;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{actions::Action, steps::Step};

use super::DirectoryAction;

/// Removes a directory
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryRemove {
    /// The directory to remove
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

impl DirectoryRemove {}

impl DirectoryAction for DirectoryRemove {}

impl Action for DirectoryRemove {
    fn privileged(&self) -> bool {
        self.privileged
    }

    fn summarize(&self) -> String {
        format!("Removing directory {}", self.target)
    }

    fn plan(
        &self,
        _manifest: &crate::manifests::Manifest,
        _context: &crate::actions::PlanCx,
    ) -> Result<Vec<crate::steps::Step>> {
        let path = PathBuf::from(&self.target);

        let steps = vec![Step::new(RemoveDirAtom { target: path })];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: directory.remove
  target: a
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::DirectoryRemove(action)) => {
                assert_eq!("a", action.action.target);
            }
            _ => {
                panic!("Dir Remove didn't deserialize to the correct type");
            }
        };
    }
}
