use super::FileAction;
use super::{default_chmod, from_octal};
use crate::atoms::file::Decrypt;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::tera_functions::register_functions;
use crate::{actions::Action, contexts::to_tera};
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::{path::PathBuf, u32};
use tera::Tera;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCopy {
    pub from: String,
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,

    pub passphrase: Option<String>,
}

fn default_template() -> bool {
    false
}

impl FileCopy {}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn summarize(&self) -> String {
        format!("Copy file from {} to {}", self.from, self.to)
    }

    fn plan(
        &self,
        manifest: &Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let contents = match self.load(manifest, &self.from) {
            Ok(contents) => {
                if self.template {
                    let mut tera = Tera::default();
                    register_functions(&mut tera);

                    let content_as_str = std::str::from_utf8(&contents)?;

                    match tera.render_str(content_as_str, &to_tera(context)) {
                        Ok(rendered) => rendered,
                        Err(err) => match err.source() {
                            Some(source) => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {}",
                                    source
                                ));
                            }
                            None => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {}",
                                    err
                                ));
                            }
                        },
                    }
                    .as_bytes()
                    .to_vec()
                } else {
                    contents
                }
            }
            Err(err) => {
                return Err(anyhow!(
                    "Failed to get contents for FileCopy action: {}",
                    err.to_string()
                ));
            }
        };

        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::{Chmod, Create, SetContents};

        let mut path = PathBuf::from(&self.to);

        if path.is_dir() {
            if let Some(file_name) = PathBuf::from(self.from.clone()).file_name() {
                path = path.join(file_name);
            }
        }

        let parent = path.clone();
        let mut steps = vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent
                        .parent()
                        .ok_or_else(|| {
                            anyhow!("Failed to get parent directory for FileCopy action")
                        })?
                        .into(),
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
        ];

        if let Some(passphrase) = self.passphrase.to_owned() {
            steps.push(Step {
                atom: Box::new(Decrypt {
                    encrypted_content: contents,
                    path,
                    passphrase,
                }),
                initializers: vec![],
                finalizers: vec![],
            });

            Ok(steps)
        } else {
            steps.push(Step {
                atom: Box::new(SetContents { path, contents }),
                initializers: vec![],
                finalizers: vec![],
            });

            Ok(steps)
        }
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
  chmod: "0777"
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!(0o777, action.action.chmod);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }
}
