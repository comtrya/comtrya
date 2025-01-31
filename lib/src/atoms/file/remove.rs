use std::path::PathBuf;

use tracing::error;

use crate::atoms::{Atom, Outcome};
use crate::utilities::password_manager::PasswordManager;

use super::FileAtom;

pub struct Remove {
    pub target: PathBuf,
}

impl FileAtom for Remove {
    fn get_path(&self) -> &std::path::PathBuf {
        &self.target
    }
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} needs to be removed", self.target.display())
    }
}

#[async_trait::async_trait]
impl Atom for Remove {
    fn plan(&self) -> anyhow::Result<crate::atoms::Outcome> {
        if !self.target.is_file() {
            error!(
                "Cannot plan: target isn`t a file: {}",
                self.target.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        let metadata = self.target.parent().map(|p| p.metadata());

        match metadata {
            Some(v) => {
                if v?.permissions().readonly() {
                    error!(
                        "Cannot plan: Dont have permission to delete {}",
                        self.target.display()
                    );

                    return Ok(Outcome {
                        side_effects: vec![],
                        should_run: false,
                    });
                }
            }
            None => {
                error!(
                    "Cannot plan: Failed to get parent directory of file: {}",
                    self.target.display()
                );

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        std::fs::remove_file(&self.target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(true, file_remove.plan().unwrap().should_run)
    }

    #[tokio::test]
    async fn it_can_execute() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(true, file_remove.plan().unwrap().should_run);
        assert_eq!(true, file_remove.execute(None).await.is_ok());
        assert_eq!(false, file_remove.plan().unwrap().should_run)
    }
}
