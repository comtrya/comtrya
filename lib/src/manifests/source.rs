//! Where the files that actions read are: below the manifest's `files` directory, or, in
//! format 2, next to the manifest file

use super::{v2, Manifest};
use std::path::{Component, Path, PathBuf};

impl Manifest {
    /// Where the source `path` of an action is. In format 2, `./x` and `../x` are relative to
    /// the manifest file's directory; anything else is below its `files` directory, as in
    /// format 1. `None` when the manifest has no directory
    pub fn source_path(&self, path: &str) -> Option<PathBuf> {
        let root_dir = self.root_dir.as_ref()?;
        if self.is_next_to_manifest(path) {
            Some(root_dir.join(path))
        } else {
            Some(root_dir.join("files").join(path))
        }
    }

    /// Whether `path` is written relative to the manifest file: `./x` or `../x`, in format 2
    pub fn is_next_to_manifest(&self, path: &str) -> bool {
        self.version == Some(v2::VERSION)
            && matches!(
                Path::new(path).components().next(),
                Some(Component::CurDir | Component::ParentDir)
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest of `version` in `/dots/zsh`
    fn manifest(version: Option<u32>) -> Manifest {
        Manifest {
            version,
            root_dir: Some(PathBuf::from("/dots/zsh")),
            ..Default::default()
        }
    }

    #[test]
    fn in_format_2_dot_paths_are_next_to_the_manifest() {
        let manifest = manifest(Some(2));

        assert_eq!(
            Some(PathBuf::from("/dots/zsh/./zshrc")),
            manifest.source_path("./zshrc")
        );
        assert_eq!(
            Some(PathBuf::from("/dots/zsh/../shared/x")),
            manifest.source_path("../shared/x")
        );
    }

    #[test]
    fn plain_names_are_below_files_in_both_formats() {
        for version in [None, Some(1), Some(2)] {
            assert_eq!(
                Some(PathBuf::from("/dots/zsh/files/zshrc")),
                manifest(version).source_path("zshrc")
            );
        }
    }

    #[test]
    fn in_format_1_dot_paths_stay_below_files() {
        assert_eq!(
            Some(PathBuf::from("/dots/zsh/files/./zshrc")),
            manifest(None).source_path("./zshrc")
        );
    }

    #[test]
    fn without_a_directory_there_is_no_source() {
        assert_eq!(None, Manifest::default().source_path("./zshrc"));
    }
}
