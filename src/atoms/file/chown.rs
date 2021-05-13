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
    fn plan(&self) -> bool {
        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(err) => {
                error!(
                    "Couldn't get metadata for {}, rejecting atom: {}",
                    &self.path.as_os_str().to_str().unwrap(),
                    err.to_string()
                );

                return false;
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
                return false;
            }
        };

        let requested_group = match users::get_group_by_name(self.group.as_str()) {
            Some(group) => group,
            None => {
                error!(
                    "Skipping chown as requested group, {}, does not exist",
                    self.group,
                );
                return false;
            }
        };

        if current_owner.uid() != requested_owner.uid() {
            return true;
        }

        if current_group.gid() != requested_group.gid() {
            return true;
        }

        false
    }

    fn execute(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(not(unix))]
impl Atom for FileOwnership {
    fn plan(&self) -> bool {
        // Never run
        false
    }

    fn execute(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can() {
        let user = users::get_current_username()
            .unwrap()
            .into_string()
            .unwrap();
        let group = users::get_current_groupname()
            .unwrap()
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

        assert_eq!(false, file_chown.plan());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: user.clone(),
            group: String::from("daemon"),
        };

        assert_eq!(true, file_chown.plan());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: group.clone(),
        };

        assert_eq!(true, file_chown.plan());

        let file_chown = Chown {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: String::from("daemon"),
        };

        assert_eq!(true, file_chown.plan());
    }
}
