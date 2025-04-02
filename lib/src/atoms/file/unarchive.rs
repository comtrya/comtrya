use std::{fs::File, path::PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::atoms::{Atom, Outcome};
use crate::utilities::password_manager::PasswordManager;

use super::FileAtom;

pub struct Unarchive {
    pub origin: PathBuf,
    pub dest: PathBuf,
    pub force: bool,
}

impl FileAtom for Unarchive {
    fn get_path(&self) -> &PathBuf {
        &self.origin
    }
}

#[async_trait::async_trait]
impl Atom for Unarchive {
    // Determine if this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome> {
        if self.dest.exists() {
            if self.force {
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: self.origin.exists(),
                });
            }
            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        Ok(Outcome {
            side_effects: vec![],
            should_run: self.origin.exists(),
        })
    }

    // Apply new to old
    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        let tar_gz = File::open(&self.origin)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&self.dest)?;
        Ok(())
    }
}

impl std::fmt::Display for Unarchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let origin_path = self.origin.display().to_string();
        let dest_path = self.dest.display().to_string();

        write!(
            f,
            "The archive {} to be decompressed to {}",
            origin_path, dest_path
        )
    }
}
