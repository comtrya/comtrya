//! Copies a file over another one

use crate::atoms::Check;
use anyhow::bail;
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;

/// Copies a file over an existing file, when their contents differ. No action uses it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Copy {
    /// The file to copy
    pub from: PathBuf,
    /// The file to overwrite. Must exist
    pub to: PathBuf,
}

impl std::fmt::Display for Copy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be copied from {}",
            self.to.display(),
            self.from.display(),
        )
    }
}

impl Change for Copy {
    fn check(&self) -> anyhow::Result<Check> {
        if let Some(pending) = Check::needs(&self.from).or_else(|| Check::needs(&self.to)) {
            return Ok(pending);
        }
        if !self.to.is_file() {
            bail!("{} isn't a file", self.to.display());
        }

        Ok(Check::drift_if(!same_contents(&self.from, &self.to)))
    }

    fn execute(&self) -> anyhow::Result<()> {
        std::fs::copy(&self.from, &self.to)?;

        Ok(())
    }
}

/// Whether both files can be read and have the same bytes
fn same_contents(a: &std::path::Path, b: &std::path::Path) -> bool {
    matches!((std::fs::read(a), std::fs::read(b)), (Ok(a), Ok(b)) if a == b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;

    #[test]
    fn it_can_plan() {
        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(Check::Drift, file_copy.check().unwrap());

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: from_file.path().to_path_buf(),
        };

        assert_eq!(Check::InSync, file_copy.check().unwrap());
    }

    #[test]
    fn it_can_execute() {
        use std::io::Write;

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(Check::Drift, file_copy.check().unwrap());
        assert!(file_copy.execute().is_ok());
        assert_eq!(Check::InSync, file_copy.check().unwrap());
    }

    #[test]
    fn it_wont_destroy_directories() {
        let to = match tempfile::TempDir::new() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to.path().to_path_buf(),
        };

        assert!(file_copy.check().is_err());
    }

    #[test]
    fn a_missing_file_is_pending() {
        let dir = tempfile::tempdir().unwrap();
        let existing = tempfile::NamedTempFile::new().unwrap();
        let missing = dir.path().join("missing");

        let from_missing = Copy {
            from: missing.clone(),
            to: existing.path().to_path_buf(),
        };
        let to_missing = Copy {
            from: existing.path().to_path_buf(),
            to: missing.clone(),
        };

        let pending = Check::Pending { needs: missing };
        assert_eq!(pending, from_missing.check().unwrap());
        assert_eq!(pending, to_missing.check().unwrap());
    }
}
