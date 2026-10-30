//! Creates a symlink

use crate::atoms::Check;
use serde::{Deserialize, Serialize};

use super::super::Change;
use anyhow::bail;
use std::path::{Path, PathBuf};

/// Links `target` to `source`. Replaces symlinks that point elsewhere or are broken, but
/// never files or directories
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// What the link points to. Must exist
    pub source: PathBuf,
    /// Where the link is created
    pub target: PathBuf,
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be linked from {}",
            self.target.display(),
            self.source.display(),
        )
    }
}

impl Change for Link {
    fn check(&self) -> anyhow::Result<Check> {
        if let Some(pending) = Check::needs(&self.source) {
            return Ok(pending);
        }

        // Target doesn't exist, we can run safely.
        // `symlink_metadata` doesn't follow links, so a broken symlink still counts as existing.
        let metadata = match std::fs::symlink_metadata(&self.target) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Check::Drift),
            Err(err) => return Err(err.into()),
        };

        // A link can be replaced, anything else would be lost
        if !metadata.file_type().is_symlink() {
            bail!("{} already exists and isn't a link", self.target.display());
        }

        let link = std::fs::read_link(&self.target)?;

        let source = if cfg!(target_os = "windows") {
            const PREFIX: &str = r"\\?\";
            PathBuf::from(&self.source.display().to_string().replace(PREFIX, ""))
        } else {
            self.source.to_owned()
        };

        // If this file doesn't link to what we expect, lets make it so
        Ok(Check::drift_if(!link.eq(&source)))
    }

    #[cfg(unix)]
    fn execute(&self) -> anyhow::Result<()> {
        remove_existing_link(&self.target)?;
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    #[cfg(windows)]
    fn execute(&self) -> anyhow::Result<()> {
        remove_existing_link(&self.target)?;
        if self.source.is_dir() {
            std::os::windows::fs::symlink_dir(&self.source, &self.target)?;
        } else {
            std::os::windows::fs::symlink_file(&self.source, &self.target)?;
        }

        Ok(())
    }
}

/// Removes `path` if it is a symlink (broken or pointing elsewhere), so it can be relinked.
/// Regular files and directories are never touched, `plan()` refuses to run for those.
fn remove_existing_link(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            // On Windows, a directory symlink must be removed with `remove_dir`
            std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can() {
        let from_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let atom = Link {
            target: from_dir.path().join("symlink"),
            source: to_file.path().to_path_buf(),
        };
        assert_eq!(Check::Drift, atom.check().unwrap());
        match atom.execute() {
            Ok(_) => {
                assert_eq!(Check::InSync, atom.check().unwrap());
            }
            Err(e) => {
                #[cfg(windows)]
                if let Some(os_err) = e.downcast_ref::<std::io::Error>() {
                    if os_err.raw_os_error() == Some(1314) {
                        // "A required privilege is not held by the client"
                        // Symlinks on Windows require Developer Mode or Admin rights
                        println!("Skipping link test execution due to missing privileges (OS Error 1314).");
                        return;
                    }
                }
                panic!("Execute failed: {e}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn it_replaces_a_broken_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "content").unwrap();
        let target = dir.path().join("target");
        std::os::unix::fs::symlink(dir.path().join("moved-away"), &target).unwrap();

        let atom = Link {
            source: source.clone(),
            target: target.clone(),
        };
        assert_eq!(Check::Drift, atom.check().unwrap());
        atom.execute().unwrap();

        assert_eq!(source, std::fs::read_link(&target).unwrap());
        assert_eq!(Check::InSync, atom.check().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn it_replaces_a_symlink_to_another_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "content").unwrap();
        let other = dir.path().join("other");
        std::fs::write(&other, "other content").unwrap();
        let target = dir.path().join("target");
        std::os::unix::fs::symlink(&other, &target).unwrap();

        let atom = Link {
            source: source.clone(),
            target: target.clone(),
        };
        assert_eq!(Check::Drift, atom.check().unwrap());
        atom.execute().unwrap();

        assert_eq!(source, std::fs::read_link(&target).unwrap());
        assert_eq!("other content", std::fs::read_to_string(&other).unwrap());
    }

    #[test]
    fn it_does_not_replace_a_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "content").unwrap();
        let target = dir.path().join("target");
        std::fs::write(&target, "keep me").unwrap();

        let atom = Link {
            source,
            target: target.clone(),
        };
        assert!(atom.check().is_err());
        assert_eq!("keep me", std::fs::read_to_string(&target).unwrap());
    }

    #[test]
    fn a_missing_source_is_pending() {
        let dir = tempfile::tempdir().unwrap();
        let atom = Link {
            source: dir.path().join("cloned/config"),
            target: dir.path().join("link"),
        };

        assert_eq!(
            Check::Pending {
                needs: dir.path().join("cloned/config")
            },
            atom.check().unwrap()
        );
    }
}
