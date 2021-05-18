use super::DirectoryAction;
use crate::{actions::Action, atoms::Atom};
use crate::{atoms::command::Exec, manifests::Manifest};
use serde::{Deserialize, Serialize};
use tera::Context;
use tracing::error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectoryCopy {
    pub from: String,
    pub to: String,
}

impl DirectoryCopy {}

impl DirectoryAction for DirectoryCopy {}

impl Action for DirectoryCopy {
    fn plan(&self, manifest: &Manifest, _context: &Context) -> Vec<Box<dyn Atom>> {
        let from: String = match self.resolve(manifest, &self.from) {
            Ok(from) => from,
            Err(_) => {
                error!("Failed to resolve path for file link");
                return vec![];
            }
        }
        .to_str()
        .unwrap()
        .into();

        vec![
            Box::new(Exec {
                command: String::from("mkdir"),
                arguments: vec![String::from("-p"), self.to.clone()],
                ..Default::default()
            }),
            Box::new(Exec {
                command: String::from("cp"),
                arguments: vec![String::from("-r"), from, self.to.clone()],
                ..Default::default()
            }),
        ]
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
            .join("copy")
    }

    #[test]
    fn it_can_be_deserialized() {
        let example_yaml = std::fs::File::open(get_manifest_dir().join("dircopy.yaml")).unwrap();
        let mut manifest: Manifest = serde_yaml::from_reader(example_yaml).unwrap();

        match manifest.actions.pop() {
            Some(Actions::DirectoryCopy(dir_copy)) => {
                assert_eq!("mydir", dir_copy.from);
                assert_eq!("/tmp/dircopy", dir_copy.to);
            }
            _ => {
                panic!("DirectoryCopy didn't deserialize to the correct type");
            }
        };
    }
}
