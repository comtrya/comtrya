use super::FileAction;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::path::PathBuf;
use tera::Context;
use tracing::debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileLink {
    pub from: String,
    pub to: String,
}

impl FileLink {}

impl FileAction for FileLink {}

impl Action for FileLink {
    fn run(&self, manifest: &Manifest, _context: &Context) -> Result<ActionResult, ActionError> {
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

        let from = self.resolve(manifest, &self.from).unwrap();
        let to = PathBuf::from(&self.to);

        match to.read_link() {
            Ok(symlink) => {
                if from.eq(&symlink) {
                    Ok(ActionResult {
                        message: String::from("Already present"),
                    })
                } else {
                    Err(ActionError {
                        message: String::from("Symlink exists to another file"),
                    })
                }
            }
            Err(_) => create_link(from, to),
        }
    }
}

#[cfg(windows)]
fn create_link(from: PathBuf, to: PathBuf) -> Result<ActionResult, ActionError> {
    if from.is_dir() {
        match std::os::windows::fs::symlink_dir(from, to).map_err(|e| ActionError {
            message: e.to_string(),
        }) {
            Ok(_) => Ok(ActionResult {
                message: String::from("Symlink created"),
            }),
            Err(e) => Err(e),
        }
    } else {
        match std::os::windows::fs::symlink_file(from, to).map_err(|e| ActionError {
            message: e.to_string(),
        }) {
            Ok(_) => Ok(ActionResult {
                message: String::from("Symlink created"),
            }),
            Err(e) => Err(e),
        }
    }
}

#[cfg(unix)]
fn create_link(from: PathBuf, to: PathBuf) -> Result<ActionResult, ActionError> {
    match std::os::unix::fs::symlink(from.clone(), to.clone()).map_err(|e| ActionError {
        message: format!("A: {:?} - {:?} - {}", from, to, e.to_string()),
    }) {
        Ok(_) => Ok(ActionResult {
            message: String::from("Symlink created"),
        }),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.link
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(link)) => {
                assert_eq!("a", link.from);
                assert_eq!("b", link.to);
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };
    }
}
