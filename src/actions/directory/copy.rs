use std::fs::create_dir_all;

use super::DirectoryAction;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifest::Manifest;
use fs_extra::dir::CopyOptions;
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectoryCopy {
    pub from: String,
    pub to: String,

    #[serde(default = "get_true")]
    pub template: bool,
}

fn get_true() -> bool {
    true
}

impl DirectoryCopy {}

impl DirectoryAction for DirectoryCopy {}

impl Action for DirectoryCopy {
    fn run(&self, manifest: &Manifest, _context: &Context) -> Result<ActionResult, ActionError> {
        let absolute_path = manifest
            .root_dir
            .clone()
            .unwrap()
            .join("files")
            .join(&self.from);

        match create_dir_all(&self.to) {
            Ok(_) => (),
            Err(_) => {
                return Err(ActionError {
                    message: String::from("Failed to create directory"),
                });
            }
        }

        match fs_extra::dir::copy(
            &absolute_path,
            &self.to,
            &CopyOptions {
                overwrite: true,
                content_only: true,
                ..Default::default()
            },
        ) {
            Ok(_) => Ok(ActionResult {
                message: String::from("Copied"),
            }),
            Err(e) => Err(ActionError {
                message: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: directory.copy
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::DirectoryCopy(dir_copy)) => {
                assert_eq!("a", dir_copy.from);
                assert_eq!("b", dir_copy.to);
            }
            _ => {
                panic!("DirectoryCopy didn't deserialize to the correct type");
            }
        };
    }
}
