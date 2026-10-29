//! `file.unarchive`: unpacks `.tar.gz` archives

use super::FileAction;
use crate::actions::{Action, PlanCx};
use crate::atoms::file::Unarchive;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Unpacks an archive into a directory
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUnarchive {
    /// The archive. A path as it is, not below the manifest's `files` directory
    #[serde(alias = "source")]
    #[schemars(extend("x-aliases" = ["source"]))]
    pub from: String,

    /// The directory to unpack into
    #[serde(alias = "target")]
    #[schemars(extend("x-aliases" = ["target"]))]
    pub to: String,

    /// Unpack again when `to` exists already. On by default; when off, an existing `to` is left
    /// alone
    pub force: Option<bool>,
}

impl FileUnarchive {}

impl FileAction for FileUnarchive {}

impl Action for FileUnarchive {
    fn summarize(&self) -> String {
        format!("Unarchiving file {} to {}", self.from, self.to)
    }

    fn plan(&self, _manifest: &Manifest, _context: &PlanCx) -> anyhow::Result<Vec<Step>> {
        let steps = vec![Step::new(Unarchive {
            origin: self.from.clone().into(),
            dest: self.to.clone().into(),
            force: self.force.unwrap_or(true),
        })];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.unarchive
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileUnarchive(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }
}
