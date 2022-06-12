use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::{debug, error, warn};

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
            self.target.to_str().unwrap(),
            self.source.to_str().unwrap(),
        )
    }
}

impl Atom for Link {
    fn plan(&self) -> bool {
        // First, ensure source exists and can be linked to
        if !self.source.exists() {
            error!(
                "Cannot plan: source file is missing: {}",
                self.source.to_str().unwrap()
            );
            return false;
        }

        // Target file doesn't exist, we can run safely
        if !self.target.exists() {
            return true;
        }
        // Target file exists, lets check if it's a symlink which can be safely updated
        // or return a false and emit some logging that we can't create the link
        // without purging a file
        let link = match std::fs::read_link(&self.target) {
            Ok(link) => link,
            Err(err) => {
                warn!(
                    "Cannot plan: target already exists and isn't a link: {}",
                    self.target.to_str().unwrap()
                );
                debug!("Cannot plan: {}", err);
                return false;
            }
        };

        // If this file doesn't link to what we expect, lets make it so
        !link.eq(&self.source)
    }

    #[cfg(unix)]
    fn execute(&mut self) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    #[cfg(windows)]
    fn execute(&mut self) -> anyhow::Result<()> {
        if self.target.is_dir() {
            std::os::windows::fs::symlink_dir(&self.source, &self.target)?;
        } else {
            std::os::windows::fs::symlink_file(&self.source, &self.target)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(true, atom.plan());
        assert_eq!(true, atom.execute().is_ok());
        assert_eq!(false, atom.plan());
    }
}
