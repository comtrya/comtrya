//! Creates a directory

use crate::atoms::{Check, Error, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;

/// Creates a directory and its parents, unless it exists
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Create {
    /// The directory to create
    pub path: PathBuf,
}

impl std::fmt::Display for Create {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The directory {} needs to be created",
            self.path.display(),
        )
    }
}

impl Change for Create {
    fn check(&self) -> Result<Check> {
        Ok(Check::drift_if(!self.path.exists()))
    }

    fn execute(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)
            .map_err(Error::io("create the directory", &self.path))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let atom = Create {
            path: std::path::PathBuf::from("/some-random-path"),
        };
        assert_eq!(Check::Drift(None), atom.check().unwrap());

        let temp = temp_dir();
        let atom = Create { path: temp };
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

        let atom = Create {
            path: temp_dir.path().join("create-me"),
        };

        assert_eq!(false, temp_dir.path().join("create-me").exists());

        assert!(atom.execute().is_ok());
        assert_eq!(Check::InSync, atom.check().unwrap());

        assert_eq!(true, temp_dir.path().join("create-me").is_dir());
    }
}
