use crate::actions::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::FileAction;
#[cfg(unix)]
use crate::atoms::file::Chown;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChown {
    pub path: String,
    pub user: Option<String>,
    pub group: Option<String>,
}

impl FileChown {}

impl FileAction for FileChown {}

impl Action for FileChown {
    fn summarize(&self) -> String {
        format!("Changing ownership for file {}", self.path)
    }

    #[cfg(not(unix))]
    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        tracing::warn!("This action is not supported on windows.");
        Ok(vec![])
    }

    #[cfg(unix)]
    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        let steps = vec![crate::steps::Step {
            atom: Box::new(Chown {
                path: self.path.clone().parse()?,
                owner: self.user.clone().unwrap_or("".to_string()),
                group: self.group.clone().unwrap_or("".to_string()),
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
    fn it_can_be_deserialized_user() {
        let yaml = r#"
- action: file.chown
  path: /home/test/one
  user: test
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert_eq!("/home/test/one", action.action.path);
                assert_eq!("test", action.action.user.unwrap());
                assert_eq!(None, action.action.group);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_be_deserialized_group() {
        let yaml = r#"
- action: file.chown
  path: /home/test/one
  group: test
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert_eq!("/home/test/one", action.action.path);
                assert_eq!(None, action.action.user);
                assert_eq!("test", action.action.group.unwrap());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }
}
