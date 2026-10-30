//! Creates an empty file

use crate::atoms::{Check, Error, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;

/// Creates an empty file, unless it exists
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Create {
    /// The file to create
    pub path: PathBuf,
}

impl std::fmt::Display for Create {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} needs to be created", self.path.display(),)
    }
}

impl Change for Create {
    fn check(&self) -> Result<Check> {
        Ok(Check::drift_if(!self.path.exists()))
    }

    fn execute(&self) -> Result<()> {
        std::fs::File::create(&self.path).map_err(Error::io("create", &self.path))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let file_create = Create {
            path: std::path::PathBuf::from("some-random-path"),
        };

        assert_eq!(Check::Drift(None), file_create.check().unwrap());

        let temp_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_create = Create {
            path: temp_file.path().to_path_buf(),
        };

        assert_eq!(Check::InSync, file_create.check().unwrap());
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

        let file_create = Create {
            path: temp_dir.path().join("create-me"),
        };

        assert!(file_create.execute().is_ok());
        assert_eq!(Check::InSync, file_create.check().unwrap());
    }

    #[test]
    fn an_error_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-dir/file");

        let error = Create { path: path.clone() }.execute().unwrap_err();

        assert_eq!(
            format!("can't create {}", path.display()),
            error.to_string()
        );
        assert!(matches!(error, Error::Io { .. }));
    }
}
