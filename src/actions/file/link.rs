use super::FileAction;
use crate::actions::{Action, ActionError, ActionResult, ActionResultExt};
use crate::manifests::Manifest;
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
    fn run(
        &self,
        manifest: &Manifest,
        _context: &Context,
        dry_run: bool,
    ) -> Result<ActionResult, ActionError> {
        let mut parent = PathBuf::from(&self.to);
        parent.pop();

        debug!(
            message = "Creating Prerequisite Directories",
            directories = &parent.to_str().unwrap()
        );

        if !dry_run {
            create_dir_all(parent).context("Failed to create parent directory")?;
        }

        let from = self.resolve(manifest, &self.from)?;
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
            Err(_) => {
                if dry_run {
                    Ok(ActionResult {
                        message: format!(
                            "symlink from {} to {}",
                            from.to_string_lossy(),
                            to.to_string_lossy()
                        ),
                    })
                } else {
                    create_link(from, to)
                }
            }
        }
    }
}

#[cfg(windows)]
fn create_link(from: PathBuf, to: PathBuf) -> Result<ActionResult, ActionError> {
    if from.is_dir() {
        std::os::windows::fs::symlink_dir(from, to)
            .context(format!("A: {:?} - {:?}", from, to))
            .map(|_| ActionResult {
                message: String::from("Symlink created"),
            })
    } else {
        std::os::windows::fs::symlink_file(from, to)
            .context(format!("A: {:?} - {:?}", from, to))
            .map(|_| ActionResult {
                message: String::from("Symlink created"),
            })
    }
}

#[cfg(unix)]
fn create_link(from: PathBuf, to: PathBuf) -> Result<ActionResult, ActionError> {
    std::os::unix::fs::symlink(from.clone(), to.clone())
        .context(format!("A: {:?} - {:?}", from, to))
        .map(|_| ActionResult {
            message: String::from("Symlink created"),
        })
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
