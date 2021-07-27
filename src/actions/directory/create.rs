use crate::atoms::directory::Create as DirectoryCreateAtom;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DirectoryCreate {
    pub path: String,
}

impl Action for DirectoryCreate {
    fn plan(&self, _: &Manifest, _context: &Contexts) -> Vec<Step> {
        vec![Step {
            atom: Box::new(DirectoryCreateAtom {
                path: PathBuf::from(&self.path),
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
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
            .join("examples")
            .join("directory")
    }

    #[test]
    fn it_can_be_deserialized() {
        let example_yaml = std::fs::File::open(get_manifest_dir().join("create.yaml")).unwrap();
        let mut manifest: Manifest = serde_yaml::from_reader(example_yaml).unwrap();

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
