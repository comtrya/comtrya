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
            .clone()
            .unwrap()
            .join("files")
            .join(path)
            .normalize()
            .unwrap()
            .as_path()
            .to_path_buf()
    }
}
