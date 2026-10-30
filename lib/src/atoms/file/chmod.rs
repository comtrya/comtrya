//! Sets the permissions of a file

use crate::atoms::{Check, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;

/// Sets the permissions of a file, when they differ
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chmod {
    /// The file
    pub path: PathBuf,
    /// The permissions, like `0o644`
    pub mode: u32,
}

impl std::fmt::Display for Chmod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The permissions on {} need to be set to {:o}",
            self.path.display(),
            self.mode
        )
    }
}

#[cfg(unix)]
use {crate::atoms::Error, std::os::unix::prelude::PermissionsExt};

#[cfg(unix)]
impl Change for Chmod {
    fn check(&self) -> Result<Check> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Check::Drift);
        }

        let metadata = std::fs::metadata(&self.path)
            .map_err(Error::io("read the permissions of", &self.path))?;

        // We expect permissions to come through as if the user was using chmod themselves.
        // This means we support 644/755 decimal syntax. We need to add 0o100000 to support
        // the part of chmod they don't often type.
        Ok(Check::drift_if(
            std::fs::Permissions::from_mode(0o100000 + self.mode).mode()
                != metadata.permissions().mode(),
        ))
    }

    fn execute(&self) -> Result<()> {
        std::fs::set_permissions(
            self.path.as_path(),
            std::fs::Permissions::from_mode(self.mode),
        )
        .map_err(Error::io("set the permissions of", &self.path))?;

        Ok(())
    }

    fn describe_change(&self) -> Option<String> {
        // A new file is described by the atom that creates it
        let current = std::fs::metadata(&self.path).ok()?.permissions().mode() & 0o7777;

        Some(format!(
            "Permissions of {} change from {current:o} to {:o}",
            self.path.display(),
            self.mode
        ))
    }
}

#[cfg(not(unix))]
impl Change for Chmod {
    fn check(&self) -> Result<Check> {
        // Never run
        Ok(Check::InSync)
    }

    fn execute(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let temp_dir = match tempfile::tempdir() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        match std::fs::File::create(temp_dir.path().join("644")) {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            std::fs::set_permissions(
                temp_dir.path().join("644"),
                std::fs::Permissions::from_mode(0o644)
            )
            .is_ok(),
        );

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o644,
        };

        assert_eq!(Check::InSync, file_chmod.check().unwrap());

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(Check::Drift, file_chmod.check().unwrap());
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

        match std::fs::File::create(temp_dir.path().join("644")) {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            std::fs::set_permissions(
                temp_dir.path().join("644"),
                std::fs::Permissions::from_mode(0o644)
            )
            .is_ok(),
        );

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o644,
        };

        assert_eq!(Check::InSync, file_chmod.check().unwrap());

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(Check::Drift, file_chmod.check().unwrap());
        assert!(file_chmod.execute().is_ok());
        assert_eq!(Check::InSync, file_chmod.check().unwrap());
    }

    #[test]
    fn describes_a_permission_change() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();

        let chmod = Chmod {
            path: file.path().to_path_buf(),
            mode: 0o600,
        };

        assert!(chmod
            .describe_change()
            .unwrap()
            .ends_with("change from 644 to 600"));
    }
}
