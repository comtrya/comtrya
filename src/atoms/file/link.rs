use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::{debug, error, warn};

pub struct FileLink {
    from: PathBuf,
    to: PathBuf,
}

impl FileAtom for FileLink {
    fn get_path(&self) -> &PathBuf {
        &self.from
    }
}

impl std::fmt::Display for FileLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be linked from {}",
            self.to.to_str().unwrap(),
            self.from.to_str().unwrap(),
        )
    }
}

impl Atom for FileLink {
    fn plan(&self) -> bool {
        // First, ensure self.to exists and can be linked to
        if !self.to.exists() {
            error!(
                "Cannot plan: file to link to is missing: {}",
                self.to.to_str().unwrap()
            );
            return false;
        }

        // File doesn't exist, we can run safely
        if !self.from.exists() {
            return true;
        }

        // File exists, lets check if it's a symlink which can be safely updated
        // or return a false and emit some logging that we can't create the link
        // without purging a file
        let link = match std::fs::read_link(&self.from) {
            Ok(link) => link,
            Err(err) => {
                warn!(
                    "Cannot plan: file to link from already exists and isn't a link: {}",
                    self.from.to_str().unwrap()
                );
                debug!("Cannot plan: {}", err);
                return false;
            }
        };

        // If this file doesn't link to what we expect, lets make it so
        !link.eq(&self.to)
    }

    #[cfg(unix)]
    fn execute(&self) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.to, &self.from)?;

        Ok(())
    }

    #[cfg(windows)]
    fn execute(&self) -> anyhow::Result<()> {
        if self.to.is_dir() {
            std::os::windows::fs::symlink_dir(&self.to, &self.from)?;
        } else {
            std::os::windows::fs::symlink_file(&self.to, &self.from)?;
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

        let atom = FileLink {
            from: from_dir.path().join("symlink"),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(true, atom.plan());
        assert_eq!(true, atom.execute().is_ok());
        assert_eq!(false, atom.plan());
    }
}
