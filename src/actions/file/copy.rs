use super::FileAction;
use crate::actions::Action;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::Result;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{path::PathBuf, u32};
use tera::Context;
use tracing::error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileCopy {
    pub from: String,
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,
}

fn from_octal<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let chmod: u32 = Deserialize::deserialize(deserializer)?;
    u32::from_str_radix(&chmod.to_string(), 8).map_err(D::Error::custom)
}

fn default_chmod() -> u32 {
    0o644
}

fn default_template() -> bool {
    false
}

impl FileCopy {}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn plan(&self, manifest: &Manifest, context: &Context) -> Vec<Step> {
        let contents = match self.load(manifest, &self.from) {
            Ok(contents) => {
                if self.template {
                    match tera::Tera::one_off(contents.as_str(), context, false) {
                        Ok(rendered) => rendered,
                        Err(err) => {
                            error!(
                                "Failed to render contents for FileCopy action: {}",
                                err.to_string()
                            );
                            return vec![];
                        }
                    }
                } else {
                    contents
                }
            }
            Err(err) => {
                error!(
                    "Failed to get contents for FileCopy action: {}",
                    err.to_string()
                );
                return vec![];
            }
        };

        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::{Chmod, Create, SetContents};

        let path = PathBuf::from(&self.to);
        let parent = path.clone();

        vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent.parent().unwrap().into(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Create { path: path.clone() }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Chmod {
                    path: path.clone(),
                    mode: self.chmod,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(SetContents { path, contents }),
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
- action: file.copy
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(file_copy)) => {
                assert_eq!("a", file_copy.from);
                assert_eq!("b", file_copy.to);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }
}
