pub mod copy;
pub mod link;

use std::path::PathBuf;

use super::ActionError;
use crate::actions::Action;
use crate::manifest::Manifest;
use tera::Tera;
use tracing::trace;

pub trait FileAction: Action {
    fn init(&self, manifest: &Manifest) -> Tera {
        let files_directory = manifest.root_dir.clone().unwrap().join("files");

        trace!(
            message = "Creating Private Tera for File Action",
            directory = files_directory.join("**/*").to_str().unwrap()
        );

        match Tera::new(files_directory.join("**/*").to_str().unwrap()) {
            Ok(t) => t,
            Err(e) => panic!(
                "Failed to initialise Tera for {:?}: {:?}",
                manifest.name.clone().unwrap(),
                e
            ),
        }
    }

    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf, ActionError> {
        manifest
            .root_dir
            .clone()
            .unwrap()
            .join("files")
            .join(path)
            .canonicalize()
            .map_err(|e| ActionError {
                message: format!("Failed because  {}", e.to_string()),
            })
    }

    fn load(&self, manifest: &Manifest, path: &str) -> Result<String, ActionError> {
        std::fs::read_to_string(manifest.root_dir.clone().unwrap().join("files").join(path))
            .map_err(|e| ActionError {
                message: e.to_string(),
            })
    }
}
