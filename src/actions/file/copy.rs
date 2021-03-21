use super::FileAction;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifests::Manifest;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fs::create_dir_all, ops::Deref, path::PathBuf, u32};
use std::{fs::Permissions, io::Write};
use tera::Context;
use tracing::debug;

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

        match set_permissions(PathBuf::from(self.to.clone()), self.chmod) {
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

#[cfg(unix)]
fn set_permissions(to: PathBuf, chmod: u32) -> std::io::Result<()> {
    use std::os::unix::prelude::PermissionsExt;

    std::fs::set_permissions(to, Permissions::from_mode(chmod))
}

#[cfg(windows)]
fn set_permissions(to: PathBuf, chmod: u32) -> std::io::Result<()> {
    Ok(())
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
