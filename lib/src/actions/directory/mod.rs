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

/// Actions that work on directories below the manifest's `files` directory, or in format 2
/// also next to the manifest (see [`Manifest::source_path`])
pub trait DirectoryAction: Action {
    /// The source `path`, which must exist
    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf> {
        let path = crate::actions::file::source_path(manifest, path)?;

        path.normalize()
            .map(normpath::BasePathBuf::into_path_buf)
            .map_err(|_| Error::NotFound(path))
    }
}
