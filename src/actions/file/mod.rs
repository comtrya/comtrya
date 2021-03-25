pub mod copy;
pub mod link;

use std::path::PathBuf;

use crate::actions::Action;
use crate::manifests::Manifest;
use anyhow::{anyhow, Result};
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

    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf> {
        use std::io::ErrorKind;

        let file_path = manifest.root_dir.clone().unwrap().join("files").join(path);

        file_path.canonicalize().map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!(
                "Failed because {} was not found",
                file_path.to_string_lossy()
            ),
            _ => anyhow!("Failed because {}", e.to_string()),
        })
    }

    fn load(&self, manifest: &Manifest, path: &str) -> Result<String> {
        use std::io::ErrorKind;
        let file_path = manifest.root_dir.clone().unwrap().join("files").join(path);

        std::fs::read_to_string(file_path.clone()).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!(
                "Failed because {} was not found",
                file_path.to_string_lossy()
            ),
            _ => anyhow!("Failed because {}", e.to_string()),
        })
    }
}
