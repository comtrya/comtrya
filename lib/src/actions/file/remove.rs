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
    fn summarize(&self) -> String {
        format!("Removing file {}", self.target)
    }

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        use crate::atoms::file::Remove as RemoveFile;

        let path = PathBuf::from(&self.target);

        let steps = vec![Step {
            atom: Box::new(RemoveFile { target: path }),
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
