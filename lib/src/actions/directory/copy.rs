//! `directory.copy`: copies the contents of a directory into another one

use super::DirectoryAction;
use crate::actions::{Action, PlanCx, Result};
use crate::steps::Step;
use crate::{atoms::directory::Copy, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Copies the contents of a directory from the manifest's `files` directory, every time
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryCopy {
    /// The directory to copy, below the manifest's `files` directory
    pub from: String,
    /// The directory the contents of `from` are copied into, on every platform. Files with the
    /// same path are overwritten, other files in it are kept. A trailing `/` makes no difference
    pub to: String,
}

impl DirectoryCopy {}

impl DirectoryAction for DirectoryCopy {}

impl Action for DirectoryCopy {
    fn summarize(&self) -> String {
        format!("Copying {} to {}", self.from, self.to)
    }

    fn plan(&self, manifest: &Manifest, _context: &PlanCx) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Copy {
            from: self.resolve(manifest, &self.from)?,
            to: PathBuf::from(&self.to),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::DirectoryCopy;
    use crate::actions::{Action, Actions, PlanCx};
    use crate::manifests::Manifest;
    use std::path::PathBuf;

    fn get_manifest_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("directory")
            .join("copy")
    }

    #[test]
    fn it_plans_one_step_with_the_copy_atom() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("files").join("mydir")).unwrap();
        let manifest = Manifest {
            root_dir: Some(dir.path().to_path_buf()),
            ..Default::default()
        };
        let action = DirectoryCopy {
            from: String::from("mydir"),
            to: String::from("/some/where/"),
        };

        let steps = action.plan(&manifest, &PlanCx::empty()).unwrap();

        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("need to be copied to /some/where/"),
            "{display}"
        );
        assert!(display.contains("mydir"), "{display}");
    }

    #[test]
    fn it_can_be_deserialized() {
        let example_yaml = std::fs::File::open(get_manifest_dir().join("dircopy.yaml")).unwrap();
        let mut manifest: Manifest = serde_yaml_ng::from_reader(example_yaml).unwrap();

        match manifest.actions.pop() {
            Some(Actions::DirectoryCopy(action)) => {
                assert_eq!("mydir", action.action.from);
                assert_eq!("/tmp/dircopy", action.action.to);
            }
            _ => {
                panic!("DirectoryCopy didn't deserialize to the correct type");
            }
        };
    }
}
