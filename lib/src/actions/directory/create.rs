//! `directory.create`

use crate::actions::{Action, PlanCx};
use crate::atoms::directory::Create as DirectoryCreateAtom;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Creates a directory and its parents
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryCreate {
    /// The directory to create
    pub path: String,
}

impl Action for DirectoryCreate {
    fn summarize(&self) -> String {
        format!("Creating directory {}", self.path)
    }

    fn plan(&self, _: &Manifest, _context: &PlanCx) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step::new(DirectoryCreateAtom {
            path: PathBuf::from(&self.path),
        })])
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;
    use crate::manifests::Manifest;
    use std::path::PathBuf;

    fn get_manifest_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("directory")
    }

    #[test]
    fn it_can_be_deserialized() {
        let example_yaml = std::fs::File::open(get_manifest_dir().join("create.yaml")).unwrap();
        let mut manifest: Manifest = serde_yaml_ng::from_reader(example_yaml).unwrap();

        match manifest.actions.pop() {
            Some(Actions::DirectoryCreate(action)) => {
                assert_eq!("/some-directory", action.action.path);
            }
            _ => {
                panic!("DirectoryCopy didn't deserialize to the correct type");
            }
        };
    }
}
