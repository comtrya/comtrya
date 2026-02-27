use super::FileAction;
use crate::atoms::file::Unarchive;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUnarchive {
    #[serde(alias = "source")]
    pub from: String,

    #[serde(alias = "target")]
    pub to: String,

    pub force: Option<bool>,
}

impl FileUnarchive {}

impl FileAction for FileUnarchive {}

impl Action for FileUnarchive {
    fn summarize(&self) -> String {
        format!("Unarchiving file {} to {}", self.from, self.to)
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let steps = vec![Step {
            atom: Box::new(Unarchive {
                origin: self.from.clone().into(),
                dest: self.to.clone().into(),
                force: self.force.unwrap_or(true),
            }),
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
