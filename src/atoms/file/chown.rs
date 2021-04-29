use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::error;

pub struct FileOwnership {
    path: PathBuf,
    owner: String,
    group: String,
}

impl FileAtom for FileOwnership {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(unix)]
use std::os::unix::prelude::MetadataExt;

#[cfg(unix)]
impl Atom for FileOwnership {
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

        let requested_owner = users::get_user_by_name(self.owner.as_str()).unwrap();
        let requested_group = users::get_group_by_name(self.group.as_str()).unwrap();

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

    fn revert(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_plan() {
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

        let file_chown = FileOwnership {
            path: temp_file.path().to_path_buf(),
            owner: user.clone(),
            group: group.clone(),
        };

        assert_eq!(false, file_chown.plan());

        let file_chown = FileOwnership {
            path: temp_file.path().to_path_buf(),
            owner: user.clone(),
            group: String::from("wheel"),
        };

        assert_eq!(true, file_chown.plan());

        let file_chown = FileOwnership {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: group.clone(),
        };

        assert_eq!(true, file_chown.plan());

        let file_chown = FileOwnership {
            path: temp_file.path().to_path_buf(),
            owner: String::from("root"),
            group: String::from("wheel"),
        };

        assert_eq!(true, file_chown.plan());
    }

    #[test]
    fn it_can_execute() {
        let temp_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_chown = FileOwnership {
            path: temp_file.path().to_path_buf(),
            owner: String::from("nobody"),
            group: String::from("nobody"),
        };

        assert_eq!(true, file_chown.plan());
        // assert_eq!(true, file_chown.execute().is_ok());
        // assert_eq!(false, file_chown.plan());
    }

    #[test]
    fn it_can_revert() {}
}
