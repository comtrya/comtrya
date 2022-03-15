use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;

pub struct Chmod {
    pub path: PathBuf,
    pub mode: u32,
}

impl FileAtom for Chmod {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for Chmod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The permissions on {} need to be set to {}",
            self.path.to_str().unwrap(),
            self.mode
        )
    }
}

#[cfg(unix)]
use {std::os::unix::prelude::PermissionsExt, tracing::error};

#[cfg(unix)]
impl Atom for Chmod {
    fn plan(&self) -> bool {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return true;
        }

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

        // We expect permissions to come through as if the user was using chmod themselves.
        // This means we support 644/755 decimal syntax. We need to add 0o100000 to support
        // the part of chmod they don't often type.
        std::fs::Permissions::from_mode(0o100000 + self.mode).mode()
            != metadata.permissions().mode()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::set_permissions(
            self.path.as_path(),
            std::fs::Permissions::from_mode(self.mode),
        )?;

        Ok(())
    }
}

#[cfg(not(unix))]
impl Atom for Chmod {
    fn plan(&self) -> bool {
        false
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

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

        assert_eq!(false, file_chmod.plan());

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan());
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

        assert_eq!(false, file_chmod.plan());

        let mut file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan());
        assert_eq!(true, file_chmod.execute().is_ok());
        assert_eq!(false, file_chmod.plan());
    }
}
