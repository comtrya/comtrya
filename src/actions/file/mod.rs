pub mod copy;
pub mod download;
pub mod link;

use crate::actions::Action;
use crate::manifests::Manifest;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub trait FileAction: Action {
    fn resolve(&self, manifest: &Manifest, path: &str) -> PathBuf {
        manifest.root_dir.clone().unwrap().join("files").join(path)
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
