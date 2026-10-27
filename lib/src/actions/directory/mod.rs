use crate::{actions::Action, manifests::Manifest};
use anyhow::anyhow;
use normpath::PathExt;
use std::path::PathBuf;

mod copy;
mod create;
mod remove;
pub use copy::DirectoryCopy;
pub use create::DirectoryCreate;
pub use remove::DirectoryRemove;

pub trait DirectoryAction: Action {
    /// `path` below the manifest's `files` directory, which must exist
    fn resolve(&self, manifest: &Manifest, path: &str) -> anyhow::Result<PathBuf> {
        let root_dir = manifest
            .root_dir
            .as_ref()
            .ok_or_else(|| anyhow!("the manifest has no directory to find {path} in"))?;
        let path = root_dir.join("files").join(path);

        path.normalize()
            .map(|path| path.into_path_buf())
            .map_err(|_| anyhow!("{} was not found", path.display()))
    }
}
