use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::error;

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
            self.path.to_str().unwrap(),
            self.owner,
            self.group,
        )
    }
}

#[cfg(unix)]
use std::os::unix::prelude::MetadataExt;

#[cfg(unix)]
impl Atom for Chown {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                should_run: true,
                side_effects: Vec::new(),
            });
        }

        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(err) => {
                error!(
                    "Couldn't get metadata for {}, rejecting atom: {}",
                    &self.path.as_os_str().to_str().unwrap(),
                    err.to_string()
                );

                return Ok(Outcome {
                    should_run: false,
                    side_effects: Vec::new(),
                });
            }
        };

        // Not happy with the unwrap's, but I'll loop back to this ... promise?
        let current_owner = users::get_user_by_uid(metadata.uid()).unwrap();
        let current_group = users::get_group_by_gid(metadata.gid()).unwrap();

        let requested_owner = match users::get_user_by_name(self.owner.as_str()) {
            Some(owner) => owner,
            None => {
                error!(
                    "Skipping chown as requested owner, {}, does not exist",
                    self.owner,
                );
                return Ok(Outcome {
                    should_run: false,
                    side_effects: Vec::new(),
                });
            }
        };

        let requested_group = match users::get_group_by_name(self.group.as_str()) {
            Some(group) => group,
            None => {
                error!(
                    "Skipping chown as requested group, {}, does not exist",
                    self.group,
                );
                return Ok(Outcome {
                    should_run: false,
                    side_effects: Vec::new(),
                });
            }
        };

        if current_owner.uid() != requested_owner.uid() {
            return Ok(Outcome {
                should_run: true,
                side_effects: Vec::new(),
            });
        }

        if current_group.gid() != requested_group.gid() {
            return Ok(Outcome {
                should_run: true,
                side_effects: Vec::new(),
            });
        }

        Ok(Outcome {
            should_run: false,
            side_effects: Vec::new(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(not(unix))]
impl Atom for Chown {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // Never run
        Ok(Outcome {
            should_run: false,
            side_effects: Vec::new(),
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
        let user = users::get_current_username()
            .unwrap_or_else(|| std::ffi::OsString::from("root"))
            .into_string()
            .unwrap();

        let group = users::get_current_groupname()
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
}
