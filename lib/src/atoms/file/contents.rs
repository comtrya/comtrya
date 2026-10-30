//! Writes the contents of a file

use crate::atoms::Check;
use serde::{Deserialize, Serialize};

use super::super::Change;
use anyhow::Context;
use similar::TextDiff;
use std::path::PathBuf;

/// Writes the contents of a file, when they differ
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetContents {
    /// The file
    pub path: PathBuf,
    /// The new contents
    pub contents: Vec<u8>,
    /// Don't show the contents in a diff, e.g. because they were decrypted
    pub redact: bool,
}

impl std::fmt::Display for SetContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents need to be set",
            self.path.display(),
        )
    }
}

impl Change for SetContents {
    fn check(&self) -> anyhow::Result<Check> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Check::Drift);
        }

        let contents = std::fs::read(&self.path)
            .with_context(|| format!("can't read {}", self.path.display()))?;

        Ok(Check::drift_if(!contents.eq(&self.contents)))
    }

    fn execute(&self) -> anyhow::Result<()> {
        std::fs::write(&self.path, &self.contents)?;

        Ok(())
    }

    fn describe_change(&self) -> Option<String> {
        let path = self.path.display();

        if !self.path.exists() {
            return Some(format!("{path} will be created"));
        }

        if self.redact {
            return Some(format!(
                "Contents of {path} change (not shown, they may contain secrets)"
            ));
        }

        let Ok(current) = std::fs::read(&self.path) else {
            return Some(format!("Contents of {path} change"));
        };

        // Also a file that was just created by an earlier atom
        if current.is_empty() {
            return Some(format!("Contents of {path} are written"));
        }

        match (
            std::str::from_utf8(&current),
            std::str::from_utf8(&self.contents),
        ) {
            (Ok(current), Ok(new)) => {
                let diff = TextDiff::from_lines(current, new)
                    .unified_diff()
                    .context_radius(3)
                    .header("current", "new")
                    .to_string();

                Some(format!("Contents of {path} change:\n{}", diff.trim_end()))
            }
            _ => Some(format!("Binary contents of {path} change")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can() {
        let file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("").into_bytes(),
            redact: false,
        };

        assert_eq!(Check::InSync, file_contents.check().unwrap());

        let file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("Hello, world!").into_bytes(),
            redact: false,
        };

        assert_eq!(Check::Drift, file_contents.check().unwrap());
        assert!(file_contents.execute().is_ok());
        assert_eq!(Check::InSync, file_contents.check().unwrap());
    }

    fn set_contents(path: &std::path::Path, contents: &[u8], redact: bool) -> SetContents {
        SetContents {
            path: path.to_path_buf(),
            contents: contents.to_vec(),
            redact,
        }
    }

    #[test]
    fn describes_a_new_file_without_its_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new");

        let change = set_contents(&path, b"secret", false)
            .describe_change()
            .unwrap();

        assert!(change.ends_with("will be created"), "{change}");
        assert!(!change.contains("secret"), "{change}");
    }

    #[test]
    fn describes_an_empty_file_without_the_new_contents() {
        let file = tempfile::NamedTempFile::new().unwrap();

        let change = set_contents(file.path(), b"secret", false)
            .describe_change()
            .unwrap();

        assert!(change.ends_with("are written"), "{change}");
    }

    #[test]
    fn describes_changed_text_as_a_unified_diff() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "one\ntwo\nthree\n").unwrap();

        let change = set_contents(file.path(), b"one\n2\nthree\n", false)
            .describe_change()
            .unwrap();

        assert!(change.contains("@@"), "{change}");
        assert!(change.contains("\n-two\n"), "{change}");
        assert!(change.contains("\n+2\n"), "{change}");
        assert!(change.contains(" one\n"), "{change}");
    }

    #[test]
    fn does_not_show_redacted_contents() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "old-secret\n").unwrap();

        let change = set_contents(file.path(), b"new-secret\n", true)
            .describe_change()
            .unwrap();

        assert!(!change.contains("secret\n"), "{change}");
        assert!(change.contains("not shown"), "{change}");
    }

    #[test]
    fn does_not_diff_binary_contents() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), [0xff, 0xfe, 0x00]).unwrap();

        let change = set_contents(file.path(), b"text\n", false)
            .describe_change()
            .unwrap();

        assert!(change.starts_with("Binary contents of"), "{change}");
    }
}
