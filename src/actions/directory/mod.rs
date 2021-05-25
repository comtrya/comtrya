use crate::{actions::Action, manifests::Manifest};
use anyhow::{anyhow, Result};
use std::path::PathBuf;

mod copy;
mod create;
pub use copy::DirectoryCopy;
pub use create::DirectoryCreate;

pub trait DirectoryAction: Action {
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
}
