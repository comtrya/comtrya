use crate::atoms::Outcome;
use crate::utilities::password_manager::PasswordManager;

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
            "The permissions on {} need to be set to {:o}",
            self.path.display(),
            self.mode
        )
    }
}

#[cfg(unix)]
use {std::os::unix::prelude::PermissionsExt, tracing::error};

#[cfg(unix)]
#[async_trait::async_trait]
impl Atom for Chmod {
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

        // We expect permissions to come through as if the user was using chmod themselves.
        // This means we support 644/755 decimal syntax. We need to add 0o100000 to support
        // the part of chmod they don't often type.
        Ok(Outcome {
            side_effects: vec![],
            should_run: std::fs::Permissions::from_mode(0o100000 + self.mode).mode()
                != metadata.permissions().mode(),
        })
    }

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        std::fs::set_permissions(
            self.path.as_path(),
            std::fs::Permissions::from_mode(self.mode),
        )?;

        Ok(())
    }
}

#[cfg(not(unix))]
#[async_trait]
impl Atom for Chmod {
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

        assert_eq!(false, file_chmod.plan().unwrap().should_run);

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan().unwrap().should_run);
    }

    #[tokio::test]
    async fn it_can_execute() {
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

        assert_eq!(false, file_chmod.plan().unwrap().should_run);

        let mut file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan().unwrap().should_run);
        assert_eq!(true, file_chmod.execute(None).await.is_ok());
        assert_eq!(false, file_chmod.plan().unwrap().should_run);
    }
}
