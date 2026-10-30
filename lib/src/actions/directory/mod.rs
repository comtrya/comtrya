//! `directory.copy`, `directory.create` and `directory.remove`

use crate::actions::{Error, Result};
use crate::{actions::Action, manifests::Manifest};
use normpath::PathExt;
use std::path::PathBuf;

mod copy;
mod create;
mod remove;
pub use copy::DirectoryCopy;
pub use create::DirectoryCreate;
pub use remove::DirectoryRemove;

/// Actions that work on directories below the manifest's `files` directory
pub trait DirectoryAction: Action {
    /// `path` below the manifest's `files` directory, which must exist
    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf> {
        let root_dir = manifest
            .root_dir
            .as_ref()
            .ok_or_else(|| Error::NoManifestDir(path.to_string()))?;
        let path = root_dir.join("files").join(path);

        path.normalize()
            .map(|path| path.into_path_buf())
            .map_err(|_| Error::NotFound(path))
    }
}
