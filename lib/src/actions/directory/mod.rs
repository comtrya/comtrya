use crate::{actions::Action, manifests::Manifest};
use normpath::PathExt;
use std::path::PathBuf;

mod copy;
mod create;
pub use copy::DirectoryCopy;
pub use create::DirectoryCreate;

pub trait DirectoryAction: Action {
    fn resolve(&self, manifest: &Manifest, path: &str) -> PathBuf {
        manifest
            .root_dir
            .as_ref()
            .and_then(|root_dir| root_dir.join("files").join(path).normalize().ok())
            .map(|path| path.as_path().to_path_buf())
            .expect("Failed to resolve path")
    }
}
