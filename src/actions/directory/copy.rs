use std::fs::create_dir_all;

use super::DirectoryAction;
use crate::actions::{Action, ActionError, ActionResult, ActionResultExt};
use crate::manifests::Manifest;
use fs_extra::dir::CopyOptions;
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectoryCopy {
    pub from: String,
    pub to: String,
}

impl DirectoryCopy {}

impl DirectoryAction for DirectoryCopy {}

impl Action for DirectoryCopy {
    fn run(
        &self,
        manifest: &Manifest,
        _context: &Context,
        _dry_run: bool,
    ) -> Result<ActionResult, ActionError> {
        let absolute_path = manifest
            .root_dir
            .clone()
            .unwrap()
            .join("files")
            .join(&self.from);

        create_dir_all(&self.to).context("Failed to create directory")?;

        fs_extra::dir::copy(
            &absolute_path,
            &self.to,
            &CopyOptions {
                overwrite: true,
                content_only: true,
                ..Default::default()
            },
        )
        .context("Failed to copy directory")?;

        Ok(ActionResult {
            message: String::from("Copied"),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::actions::Actions;
    use crate::manifests::Manifest;
    use crate::Action;

    fn get_manifest_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("examples")
            .join("directory")
            .join("copy")
    }

    #[test]
    fn it_can_be_deserialized() {
        let example_yaml = std::fs::File::open(get_manifest_dir().join("main.yaml")).unwrap();
        let mut manifest: Manifest = serde_yaml::from_reader(example_yaml).unwrap();

        match manifest.actions.pop() {
            Some(Actions::DirectoryCopy(dir_copy)) => {
                assert_eq!("mydir", dir_copy.from);
                assert_eq!("mydircopy", dir_copy.to);
            }
            _ => {
                panic!("DirectoryCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_copy_a_directory() {
        let manifest_dir = std::env::current_dir()
            .unwrap()
            .join("examples")
            .join("directory")
            .join("copy");

        let manifest = crate::manifests::Manifest {
            name: Some(String::from("copy")),
            actions: vec![],
            dag_index: None,
            depends: vec![],
            root_dir: Some(manifest_dir.clone()),
        };

        let to = std::env::temp_dir().join("test-case");

        let directory_copy = super::DirectoryCopy {
            from: String::from("mydir"),
            to: String::from(to.to_str().unwrap()),
        };

        directory_copy
            .run(&manifest, &tera::Context::new(), false)
            .unwrap();

        assert_eq!(true, to.is_dir());
        assert_eq!(true, to.join("file-a").exists());
        assert_eq!(true, to.join("file-b").exists());
    }
}
