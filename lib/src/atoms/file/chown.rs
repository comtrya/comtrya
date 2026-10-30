//! Sets the owner and group of a file

use crate::atoms::{Check, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::PathBuf;

/// Sets the owner and group of a file, when they differ. Can't run if either doesn't exist
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chown {
    /// The file
    pub path: PathBuf,
    /// The user name of the new owner
    pub owner: String,
    /// The name of the new group
    pub group: String,
}

impl std::fmt::Display for Chown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The owner and group on {} need to be set to {}:{}",
            self.path.display(),
            self.owner,
            self.group,
        )
    }
}

#[cfg(unix)]
use {crate::atoms::Error, std::os::unix::prelude::MetadataExt};

#[cfg(unix)]
impl Change for Chown {
    fn check(&self) -> Result<Check> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Check::Drift(None));
        }

        let metadata =
            std::fs::metadata(&self.path).map_err(Error::io("read the owner of", &self.path))?;
        let owner = uzers::get_user_by_name(&self.owner)
            .ok_or_else(|| Error::UnknownUser(self.owner.clone()))?;
        let group = uzers::get_group_by_name(&self.group)
            .ok_or_else(|| Error::UnknownGroup(self.group.clone()))?;

        Ok(Check::drift_if(
            metadata.uid() != owner.uid() || metadata.gid() != group.gid(),
        ))
    }

    fn execute(&self) -> Result<()> {
        let uid = match self.owner.as_str() {
            "" => None,
            owner => Some(
                uzers::get_user_by_name(owner)
                    .ok_or_else(|| Error::UnknownUser(owner.to_string()))?
                    .uid(),
            ),
        };

        let gid = match self.group.as_str() {
            "" => None,
            group => Some(
                uzers::get_group_by_name(group)
                    .ok_or_else(|| Error::UnknownGroup(group.to_string()))?
                    .gid(),
            ),
        };

        std::os::unix::fs::chown(&self.path, uid, gid)
            .map_err(Error::io("change the owner of", &self.path))?;

        Ok(())
    }
}

#[cfg(not(unix))]
impl Change for Chown {
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
    fn it_can() {
        // Using unwrap_or_else which catches the CI build where the users
        // crate can't seem to detect the user within a container.
        // Which I know to be root.
        let user = uzers::get_current_username()
            .unwrap_or_else(|| std::ffi::OsString::from("root"))
            .into_string()
            .unwrap();

        let group = uzers::get_current_groupname()
            .unwrap_or_else(|| std::ffi::OsString::from("root"))
            .into_string()
            .unwrap();

        let temp_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user.clone(),
            group: group.clone(),
        };

        assert_eq!(Check::InSync, file_chown.check().unwrap());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user,
            group: String::from("daemon"),
        };

        assert_eq!(Check::Drift(None), file_chown.check().unwrap());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group,
        };

        assert_eq!(Check::Drift(None), file_chown.check().unwrap());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: String::from("daemon"),
        };

        assert_eq!(Check::Drift(None), file_chown.check().unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn it_changes_the_owner_to_the_current_user() {
        let user = uzers::get_current_username()
            .unwrap_or_else(|| std::ffi::OsString::from("root"))
            .into_string()
            .unwrap();
        let group = uzers::get_current_groupname()
            .unwrap_or_else(|| std::ffi::OsString::from("root"))
            .into_string()
            .unwrap();
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        // Changing to the owner it already has needs no privileges
        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user,
            group,
        };
        file_chown.execute().unwrap();

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("comtrya-no-such-user"),
            group: String::new(),
        };
        let error = file_chown.execute().unwrap_err();
        assert!(
            error.to_string().contains("comtrya-no-such-user"),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_owner_is_an_error() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("comtrya-no-such-user"),
            group: String::from("comtrya-no-such-group"),
        };

        let error = file_chown.check().unwrap_err();
        assert!(
            error.to_string().contains("comtrya-no-such-user"),
            "{error}"
        );
    }
}
