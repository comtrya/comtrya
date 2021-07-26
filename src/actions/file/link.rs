use super::FileAction;
use crate::actions::Action;
use crate::manifests::Manifest;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tera::Context;
use tracing::error;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileLink {
    #[serde(alias = "from")]
    pub source: String,

    #[serde(alias = "to")]
    pub target: String,
}

impl FileLink {}

impl FileAction for FileLink {}

impl Action for FileLink {
    fn plan(&self, manifest: &Manifest, _: &Context) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        let from: PathBuf = match self.resolve(manifest, &self.source) {
            Ok(from) => from,
            Err(_) => {
                error!("Failed to resolve path for file link");
                return vec![];
            }
        };

        let to = PathBuf::from(&self.target);
        let parent = to.clone();

        vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent.parent().unwrap().into(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Link {
                    source: from,
                    target: to,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.link
  source: a
  target: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source);
                assert_eq!("b", action.action.target);
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };

        // Old style format
        let yaml = r#"
- action: file.link
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source);
                assert_eq!("b", action.action.target);
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };
    }
}
