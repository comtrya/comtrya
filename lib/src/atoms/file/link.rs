use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use crate::utilities::password_manager::PasswordManager;
use std::path::PathBuf;
use tracing::{error, warn};

pub struct Link {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl FileAtom for Link {
    fn get_path(&self) -> &PathBuf {
        &self.source
    }
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be linked from {}",
            self.target.display(),
            self.source.display(),
        )
    }
}

#[async_trait::async_trait]
impl Atom for Link {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // First, ensure source exists and can be linked to
        if !self.source.exists() {
            error!(
                "Cannot plan: source file is missing: {}",
                self.source.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        // Target file doesn't exist, we can run safely
        if !self.target.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }
        // Target file exists, lets check if it's a symlink which can be safely updated
        // or return a false and emit some logging that we can't create the link
        // without purging a file
        let link = match std::fs::read_link(&self.target) {
            Ok(link) => link,
            Err(err) => {
                warn!(
                    "Cannot plan: target already exists and isn't a link: {}",
                    self.target.display()
                );
                error!("Cannot plan: {}", err);

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        let source = if cfg!(target_os = "windows") {
            const PREFIX: &str = r"\\?\";
            PathBuf::from(&self.source.display().to_string().replace(PREFIX, ""))
        } else {
            self.source.to_owned()
        };

        // If this file doesn't link to what we expect, lets make it so
        Ok(Outcome {
            side_effects: vec![],
            should_run: !link.eq(&source),
        })
    }

    #[cfg(unix)]
    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    #[cfg(windows)]
    async fn execute(&mut self) -> anyhow::Result<()> {
        if self.target.is_dir() {
            std::os::windows::fs::symlink_dir(&self.source, &self.target)?;
        } else {
            std::os::windows::fs::symlink_file(&self.source, &self.target)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn it_can() {
        let from_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut atom = Link {
            target: from_dir.path().join("symlink"),
            source: to_file.path().to_path_buf(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);
        assert_eq!(true, atom.execute(None).await.is_ok());
        assert_eq!(false, atom.plan().unwrap().should_run);
    }
}
