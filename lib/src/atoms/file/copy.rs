//! Copies a file over another one

use crate::atoms::Outcome;
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;
use tracing::error;

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
    fn plan(&self) -> anyhow::Result<Outcome> {
        if !self.to.is_file() {
            error!("Cannot plan: target isn't a file: {}", self.to.display());

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        Ok(Outcome {
            side_effects: vec![],
            should_run: !same_contents(&self.from, &self.to),
        })
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

        assert_eq!(true, file_copy.plan().unwrap().should_run);

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: from_file.path().to_path_buf(),
        };

        assert_eq!(false, file_copy.plan().unwrap().should_run);
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

        assert_eq!(true, file_copy.plan().unwrap().should_run);
        assert_eq!(true, file_copy.execute().is_ok());
        assert_eq!(false, file_copy.plan().unwrap().should_run);
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

        assert_eq!(false, file_copy.plan().unwrap().should_run);
    }
}
