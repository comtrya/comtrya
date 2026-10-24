use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use std::path::{Path, PathBuf};
use tracing::{error, warn};

pub struct Link {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl FileAtom for Link {
    fn get_path(&self) -> &PathBuf {
        &self.source
    }
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

impl Atom for Link {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // First, ensure source exists and can be linked to
        if !self.source.exists() {
            error!(
                "Cannot plan: source file is missing: {}",
                self.source.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        // Target doesn't exist, we can run safely.
        // `symlink_metadata` doesn't follow links, so a broken symlink still counts as existing.
        let metadata = match std::fs::symlink_metadata(&self.target) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: true,
                })
            }
            Err(err) => return Err(err.into()),
        };

        // Target exists, lets check if it's a symlink which can be safely updated
        // or return a false and emit some logging that we can't create the link
        // without purging a file
        if !metadata.file_type().is_symlink() {
            warn!(
                "Cannot plan: target already exists and isn't a link: {}",
                self.target.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        let link = std::fs::read_link(&self.target)?;

        let source = if cfg!(target_os = "windows") {
            const PREFIX: &str = r"\\?\";
            PathBuf::from(&self.source.display().to_string().replace(PREFIX, ""))
        } else {
            self.source.to_owned()
        };

        // If this file doesn't link to what we expect, lets make it so
        Ok(Outcome {
            side_effects: vec![],
            should_run: !link.eq(&source),
        })
    }

    #[cfg(unix)]
    fn execute(&mut self) -> anyhow::Result<()> {
        remove_existing_link(&self.target)?;
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    #[cfg(windows)]
    fn execute(&mut self) -> anyhow::Result<()> {
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

        let mut atom = Link {
            target: from_dir.path().join("symlink"),
            source: to_file.path().to_path_buf(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);
        match atom.execute() {
            Ok(_) => {
                assert_eq!(false, atom.plan().unwrap().should_run);
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

        let mut atom = Link {
            source: source.clone(),
            target: target.clone(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);
        atom.execute().unwrap();

        assert_eq!(source, std::fs::read_link(&target).unwrap());
        assert_eq!(false, atom.plan().unwrap().should_run);
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

        let mut atom = Link {
            source: source.clone(),
            target: target.clone(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);
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
        assert_eq!(false, atom.plan().unwrap().should_run);
        assert_eq!("keep me", std::fs::read_to_string(&target).unwrap());
    }
}
