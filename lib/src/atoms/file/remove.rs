use std::path::PathBuf;

use tracing::error;

use crate::atoms::{Atom, Outcome};

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

        return Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        });
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        dbg!(&self.target);
        std::fs::remove_file(&self.target)?;
        Ok(())
    }
}
