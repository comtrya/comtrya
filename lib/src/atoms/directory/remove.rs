//! Removes a directory

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::atoms::{Change, Check, Error, Result};

/// Removes an empty directory
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Remove {
    /// The directory to remove
    pub target: PathBuf,
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The directory {} needs to be removed",
            self.target.display(),
        )
    }
}

impl Change for Remove {
    fn check(&self) -> Result<Check> {
        if !self.target.exists() {
            return Ok(Check::InSync);
        }
        if !self.target.is_dir() {
            return Err(Error::NotA {
                path: self.target.clone(),
                kind: "directory",
            });
        }
        let mut entries = self
            .target
            .read_dir()
            .map_err(Error::io("read", &self.target))?;
        if entries.next().is_some() {
            return Err(Error::NotEmpty {
                path: self.target.clone(),
            });
        }

        Ok(Check::Drift(None))
    }

    fn execute(&self) -> Result<()> {
        std::fs::remove_dir(&self.target).map_err(Error::io("remove", &self.target))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{fs::File, io::Write};

    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn it_can_plan() {
        let temp = tempdir().unwrap().keep();
        let atom = Remove {
            target: temp.clone(),
        };
        assert_eq!(Check::Drift(None), atom.check().unwrap());

        let file_path = temp.join("my-temporary-note.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Brian was here. Briefly.").unwrap();

        // Can't run when dir is not empty
        assert!(atom.check().is_err());

        // Can't run if path is not a dir
        let atom = Remove {
            target: file_path.clone(),
        };
        assert!(atom.check().is_err());

        // Nothing to do if it's gone already
        let atom = Remove {
            target: temp.join("missing"),
        };
        assert_eq!(Check::InSync, atom.check().unwrap());
    }

    #[test]
    fn it_can_execute() {
        let temp_dir = match tempfile::tempdir() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let path = temp_dir.path();

        assert_eq!(true, path.exists());

        let atom = Remove {
            target: temp_dir.path().to_path_buf(),
        };

        // Deletes dir
        assert_eq!(true, atom.execute().is_ok());
        // Dir is deleted and dont exists
        assert_eq!(false, temp_dir.path().exists())
    }
}
