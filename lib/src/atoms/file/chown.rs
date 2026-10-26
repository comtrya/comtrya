use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;

#[cfg(unix)]
use tracing::error;

#[derive(Debug)]
pub struct Chown {
    pub path: PathBuf,
    pub owner: String,
    pub group: String,
}

impl FileAtom for Chown {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
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
use std::os::unix::prelude::MetadataExt;

#[cfg(unix)]
use anyhow::anyhow;

#[cfg(unix)]
impl Atom for Chown {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }

        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(err) => {
                error!(
                    "Couldn't get metadata for {}, rejecting atom: {}",
                    &self.path.display(),
                    err.to_string()
                );

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        if let (Some(current_owner), Some(current_group)) = (
            uzers::get_user_by_uid(metadata.uid()),
            uzers::get_group_by_gid(metadata.gid()),
        ) {
            let requested_owner = match uzers::get_user_by_name(self.owner.as_str()) {
                Some(owner) => owner,
                None => {
                    error!(
                        "Skipping chown as requested owner, {}, does not exist",
                        self.owner,
                    );
                    return Ok(Outcome {
                        side_effects: vec![],
                        should_run: false,
                    });
                }
            };

            let requested_group = match uzers::get_group_by_name(self.group.as_str()) {
                Some(group) => group,
                None => {
                    error!(
                        "Skipping chown as requested group, {}, does not exist",
                        self.group,
                    );

                    return Ok(Outcome {
                        side_effects: vec![],
                        should_run: false,
                    });
                }
            };

            if current_owner.uid() != requested_owner.uid() {
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: true,
                });
            }

            if current_group.gid() != requested_group.gid() {
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: true,
                });
            }
        }

        error!("Something happened here");

        Ok(Outcome {
            side_effects: vec![],
            should_run: false,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let uid = match self.owner.as_str() {
            "" => None,
            owner => Some(
                uzers::get_user_by_name(owner)
                    .ok_or_else(|| anyhow!("user {owner} doesn't exist"))?
                    .uid(),
            ),
        };

        let gid = match self.group.as_str() {
            "" => None,
            group => Some(
                uzers::get_group_by_name(group)
                    .ok_or_else(|| anyhow!("group {group} doesn't exist"))?
                    .gid(),
            ),
        };

        std::os::unix::fs::chown(&self.path, uid, gid)?;

        Ok(())
    }
}

#[cfg(not(unix))]
impl Atom for Chown {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // Never run
        Ok(Outcome {
            side_effects: vec![],
            should_run: false,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
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

        assert_eq!(false, file_chown.plan().unwrap().should_run);

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user,
            group: String::from("daemon"),
        };

        assert_eq!(true, file_chown.plan().unwrap().should_run);

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group,
        };

        assert_eq!(true, file_chown.plan().unwrap().should_run);

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: String::from("daemon"),
        };

        assert_eq!(true, file_chown.plan().unwrap().should_run);
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
        let mut file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user,
            group,
        };
        file_chown.execute().unwrap();

        let mut file_chown = Chown {
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
}
