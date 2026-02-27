use std::path::PathBuf;

use crate::atoms::directory::Remove as RemoveDirAtom;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{actions::Action, steps::Step};

use super::DirectoryAction;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryRemove {
    pub target: String,
}

impl DirectoryRemove {}

impl DirectoryAction for DirectoryRemove {}

impl Action for DirectoryRemove {
    fn summarize(&self) -> String {
        format!("Removing directory {}", self.target)
    }

    fn plan(
        &self,
        _manifest: &crate::manifests::Manifest,
        _context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        let path = PathBuf::from(&self.target);

        let steps = vec![Step {
            atom: Box::new(RemoveDirAtom { target: path }),
            initializers: vec![],
            finalizers: vec![],
        }];

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
