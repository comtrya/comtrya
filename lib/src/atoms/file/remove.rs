//! Removes a file

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::atoms::{Change, Check, Error, Result};

/// Removes a file. Not a directory
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Remove {
    /// The file to remove
    pub target: PathBuf,
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} needs to be removed", self.target.display())
    }
}

impl Change for Remove {
    fn check(&self) -> Result<Check> {
        if !self.target.exists() {
            return Ok(Check::InSync);
        }
        if !self.target.is_file() {
            return Err(Error::NotA {
                path: self.target.clone(),
                kind: "file",
            });
        }

        // A relative `name` has the parent ``
        let parent = match self.target.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let metadata = parent.metadata().map_err(Error::io("read", parent))?;
        if metadata.permissions().readonly() {
            return Err(Error::ReadOnly {
                path: self.target.clone(),
            });
        }

        Ok(Check::Drift(None))
    }

    fn execute(&self) -> Result<()> {
        std::fs::remove_file(&self.target).map_err(Error::io("remove", &self.target))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(Check::Drift(None), file_remove.check().unwrap())
    }

    #[test]
    fn it_can_execute() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(Check::Drift(None), file_remove.check().unwrap());
        assert!(file_remove.execute().is_ok());
        assert_eq!(Check::InSync, file_remove.check().unwrap())
    }

    #[test]
    fn a_missing_file_is_in_sync() {
        let dir = tempfile::tempdir().unwrap();
        let file_remove = Remove {
            target: dir.path().join("gone"),
        };

        assert_eq!(Check::InSync, file_remove.check().unwrap());
    }

    #[test]
    fn a_directory_is_not_removed() {
        let dir = tempfile::tempdir().unwrap();
        let file_remove = Remove {
            target: dir.path().to_path_buf(),
        };

        assert!(file_remove.check().is_err());
    }
}
