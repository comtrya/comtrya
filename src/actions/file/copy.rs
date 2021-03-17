use super::FileAction;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::{fs::create_dir_all, ops::Deref, path::PathBuf};
use std::{fs::Permissions, io::Write, os::unix::prelude::PermissionsExt};
use tera::Context;
use tracing::debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileCopy {
    pub from: String,
    pub to: String,

    #[serde(default = "default_chmod")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,
}

fn default_chmod() -> u32 {
    644
}

fn default_template() -> bool {
    false
}

impl FileCopy {}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult, ActionError> {
        let tera = self.init(manifest);

        let contents = match if self.template {
            tera.render(self.from.clone().deref(), context)
                .map_err(|e| ActionError {
                    message: e.to_string(),
                })
        } else {
            self.load(manifest, &self.from)
        } {
            Ok(contents) => contents,
            Err(error) => {
                return Err(error);
            }
        };

        let mut parent = PathBuf::from(&self.to);
        parent.pop();

        debug!(
            message = "Creating Prerequisite Directories",
            directories = &parent.to_str().unwrap()
        );

        match create_dir_all(parent) {
            Ok(_) => (),
            Err(_) => {
                return Err(ActionError {
                    message: String::from("Failed to create parent directory"),
                });
            }
        }

        let mut file = match std::fs::File::create(self.to.clone()) {
            Ok(f) => f,
            Err(_) => {
                return Err(ActionError {
                    message: String::from("Failed to create file"),
                });
            }
        };

        match file.write_all(contents.as_bytes()) {
            Ok(_) => {}
            Err(_) => {
                return Err(ActionError {
                    message: String::from("Failed to create file"),
                });
            }
        };

        match file.sync_all() {
            Ok(_) => {}
            Err(_) => {
                return Err(ActionError {
                    message: String::from("Failed to create file"),
                });
            }
        }

        match std::fs::set_permissions(self.to.clone(), Permissions::from_mode(self.chmod)) {
            Ok(_) => {}
            Err(e) => {
                return Err(ActionError {
                    message: format!("Failed to set permissions: {}", e.to_string()),
                })
            }
        }

        Ok(ActionResult {
            message: String::from("Copied"),
        })
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
